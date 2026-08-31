use chrono::Utc;
use pomotask_cli::app::Task;
use pomotask_cli::ipc::{
    execute_ipc_command, load_blocklist, load_runtime_state, save_blocklist, save_runtime_state,
    BlocklistConfig, RuntimeState,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tokio::sync::Mutex;

static TEST_LOCK: Mutex<()> = Mutex::const_new(());

struct TestContext {
    temp_dir: PathBuf,
}

impl TestContext {
    fn new(test_name: &str) -> Self {
        let temp_dir = std::env::temp_dir().join(format!(
            "pomotask_test_ipc_{}_{}",
            test_name,
            rand::random::<u64>()
        ));
        fs::create_dir_all(&temp_dir).unwrap();
        std::env::set_var("POMOTASK_CONFIG_DIR", temp_dir.to_str().unwrap());
        Self { temp_dir }
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.temp_dir);
    }
}

#[tokio::test]
async fn test_ipc_status() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("status");
    let state = RuntimeState {
        state: "running".to_string(),
        mode: "work".to_string(),
        remaining_seconds: 1400,
        total_seconds: 1500,
        session_pomodoros: 3,
        active_task_id: Some("task_99".to_string()),
        active_task_title: Some("My Special Task".to_string()),
        strict_break: false,
        anti_distraction: true,
    };
    save_runtime_state(&state).unwrap();

    let output = execute_ipc_command(&["status".to_string()])
        .await
        .expect("execute status");
    let parsed: RuntimeState = serde_json::from_str(&output).expect("parse runtime state JSON");
    assert_eq!(parsed.state, "running");
    assert_eq!(parsed.remaining_seconds, 1400);
    assert_eq!(parsed.active_task_title.as_deref(), Some("My Special Task"));
}

#[tokio::test]
async fn test_ipc_timer_commands() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("timer");
    let mut state = RuntimeState::default();
    state.state = "stopped".to_string();
    state.mode = "work".to_string();
    state.remaining_seconds = 1500;
    state.total_seconds = 1500;
    save_runtime_state(&state).unwrap();

    // Start
    execute_ipc_command(&["timer".to_string(), "start".to_string()])
        .await
        .expect("timer start");
    let updated = load_runtime_state();
    assert_eq!(updated.state, "running");

    // Pause
    execute_ipc_command(&["timer".to_string(), "pause".to_string()])
        .await
        .expect("timer pause");
    let updated = load_runtime_state();
    assert_eq!(updated.state, "paused");

    // Toggle (from paused -> running)
    execute_ipc_command(&["timer".to_string(), "toggle".to_string()])
        .await
        .expect("timer toggle");
    let updated = load_runtime_state();
    assert_eq!(updated.state, "running");

    // Toggle (from running -> paused)
    execute_ipc_command(&["timer".to_string(), "toggle".to_string()])
        .await
        .expect("timer toggle");
    let updated = load_runtime_state();
    assert_eq!(updated.state, "paused");

    // Skip (from work -> short_break)
    execute_ipc_command(&["timer".to_string(), "skip".to_string()])
        .await
        .expect("timer skip");
    let updated = load_runtime_state();
    assert_eq!(updated.mode, "short_break");
    assert_eq!(updated.session_pomodoros, 1);

    // Reset
    execute_ipc_command(&["timer".to_string(), "reset".to_string()])
        .await
        .expect("timer reset");
    let updated = load_runtime_state();
    assert_eq!(updated.remaining_seconds, updated.total_seconds);
    assert_ne!(updated.state, "running");
}

#[tokio::test]
async fn test_ipc_tasks_list() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("tasks_list");
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let mut map: HashMap<String, Vec<Task>> = HashMap::new();
    map.insert(
        "list_1".to_string(),
        vec![Task {
            id: "t1".to_string(),
            list_id: "list_1".to_string(),
            title: "Task 1".to_string(),
            completed: false,
            due: None,
            updated: Utc::now(),
            completed_at: None,
            notes: None,
            parent_id: None,
            pomodoros: 0,
        }],
    );
    map.insert(
        "list_2".to_string(),
        vec![Task {
            id: "t2".to_string(),
            list_id: "list_2".to_string(),
            title: "Task 2".to_string(),
            completed: false,
            due: None,
            updated: Utc::now(),
            completed_at: None,
            notes: None,
            parent_id: None,
            pomodoros: 0,
        }],
    );
    fs::write(&cache_path, serde_json::to_string(&map).unwrap()).unwrap();

    // List all
    let output_all = execute_ipc_command(&["tasks".to_string(), "list".to_string()])
        .await
        .expect("list all");
    let tasks_all: Vec<Task> = serde_json::from_str(&output_all).expect("parse tasks list JSON");
    assert_eq!(tasks_all.len(), 2);

    // List specific list
    let output_l1 = execute_ipc_command(&[
        "tasks".to_string(),
        "list".to_string(),
        "--list-id".to_string(),
        "list_1".to_string(),
    ])
    .await
    .expect("list list_1");
    let tasks_l1: Vec<Task> = serde_json::from_str(&output_l1).expect("parse tasks list JSON");
    assert_eq!(tasks_l1.len(), 1);
    assert_eq!(tasks_l1[0].id, "t1");
}

#[tokio::test]
async fn test_ipc_task_complete() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("task_complete");
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let mut map: HashMap<String, Vec<Task>> = HashMap::new();
    map.insert(
        "list_1".to_string(),
        vec![Task {
            id: "t1".to_string(),
            list_id: "list_1".to_string(),
            title: "Task 1".to_string(),
            completed: false,
            due: None,
            updated: Utc::now(),
            completed_at: None,
            notes: None,
            parent_id: None,
            pomodoros: 0,
        }],
    );
    fs::write(&cache_path, serde_json::to_string(&map).unwrap()).unwrap();

    execute_ipc_command(&["task".to_string(), "complete".to_string(), "t1".to_string()])
        .await
        .expect("complete task");

    let cache_data = fs::read_to_string(&cache_path).unwrap();
    let loaded_map: HashMap<String, Vec<Task>> = serde_json::from_str(&cache_data).unwrap();
    let task = loaded_map
        .get("list_1")
        .unwrap()
        .iter()
        .find(|t| t.id == "t1")
        .unwrap();
    assert!(task.completed);
}

#[tokio::test]
async fn test_ipc_task_create() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("task_create");
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let map: HashMap<String, Vec<Task>> = HashMap::new();
    fs::write(&cache_path, serde_json::to_string(&map).unwrap()).unwrap();

    execute_ipc_command(&[
        "task".to_string(),
        "create".to_string(),
        "--title".to_string(),
        "Buy milk".to_string(),
        "--list-id".to_string(),
        "personal".to_string(),
    ])
    .await
    .expect("create task");

    let cache_data = fs::read_to_string(&cache_path).unwrap();
    let loaded_map: HashMap<String, Vec<Task>> = serde_json::from_str(&cache_data).unwrap();
    let tasks = loaded_map.get("personal").expect("personal list");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "Buy milk");
}

#[tokio::test]
async fn test_ipc_task_focus() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("task_focus");
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let mut map: HashMap<String, Vec<Task>> = HashMap::new();
    map.insert(
        "list_1".to_string(),
        vec![Task {
            id: "t_focus".to_string(),
            list_id: "list_1".to_string(),
            title: "Crucial Feature".to_string(),
            completed: false,
            due: None,
            updated: Utc::now(),
            completed_at: None,
            notes: None,
            parent_id: None,
            pomodoros: 0,
        }],
    );
    fs::write(&cache_path, serde_json::to_string(&map).unwrap()).unwrap();

    // Focus on t_focus
    execute_ipc_command(&[
        "task".to_string(),
        "focus".to_string(),
        "t_focus".to_string(),
    ])
    .await
    .expect("focus task");
    let state = load_runtime_state();
    assert_eq!(state.active_task_id.as_deref(), Some("t_focus"));
    assert_eq!(state.active_task_title.as_deref(), Some("Crucial Feature"));

    // Clear focus
    execute_ipc_command(&["task".to_string(), "focus".to_string(), "clear".to_string()])
        .await
        .expect("clear focus");
    let state = load_runtime_state();
    assert_eq!(state.active_task_id, None);
    assert_eq!(state.active_task_title, None);
}

#[tokio::test]
async fn test_ipc_blocklist_commands() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("blocklist");
    let mut config = BlocklistConfig::default();
    config.action = "warn".to_string();
    save_blocklist(&config).unwrap();

    let mut state = RuntimeState::default();
    state.strict_break = false;
    state.anti_distraction = true;
    save_runtime_state(&state).unwrap();

    // blocklist get
    let output = execute_ipc_command(&["blocklist".to_string(), "get".to_string()])
        .await
        .expect("blocklist get");
    let parsed: BlocklistConfig = serde_json::from_str(&output).expect("parse blocklist config");
    assert_eq!(parsed.action, "warn");

    // toggle-strict
    execute_ipc_command(&["blocklist".to_string(), "toggle-strict".to_string()])
        .await
        .expect("toggle-strict");
    let updated_state = load_runtime_state();
    assert!(updated_state.strict_break);

    // toggle-anti-distraction
    execute_ipc_command(&[
        "blocklist".to_string(),
        "toggle-anti-distraction".to_string(),
    ])
    .await
    .expect("toggle-anti-distraction");
    let updated_state = load_runtime_state();
    assert!(!updated_state.anti_distraction);

    // set-action
    execute_ipc_command(&[
        "blocklist".to_string(),
        "set-action".to_string(),
        "minimize".to_string(),
    ])
    .await
    .expect("set-action");
    let updated_config = load_blocklist();
    assert_eq!(updated_config.action, "minimize");
}

#[tokio::test]
async fn test_ipc_prefix_and_unknown_commands() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("prefix");
    // "ipc status"
    let out1 = execute_ipc_command(&["ipc".to_string(), "status".to_string()])
        .await
        .expect("ipc status");
    assert!(out1.contains("\"state\""));

    // "--ipc status"
    let out2 = execute_ipc_command(&["--ipc".to_string(), "status".to_string()])
        .await
        .expect("--ipc status");
    assert!(out2.contains("\"state\""));

    // Unknown command
    let err = execute_ipc_command(&["unknown_cmd".to_string()]).await;
    assert!(err.is_err());
}

#[tokio::test]
async fn test_ipc_args_with_literal_ipc_value() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("literal_ipc");
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let map: HashMap<String, Vec<Task>> = HashMap::new();
    fs::write(&cache_path, serde_json::to_string(&map).unwrap()).unwrap();

    // Create task with title "ipc"
    execute_ipc_command(&[
        "ipc".to_string(),
        "task".to_string(),
        "create".to_string(),
        "--title".to_string(),
        "ipc".to_string(),
        "--list-id".to_string(),
        "default".to_string(),
    ])
    .await
    .expect("create task with ipc title");

    let cache_data = fs::read_to_string(&cache_path).unwrap();
    let loaded_map: HashMap<String, Vec<Task>> = serde_json::from_str(&cache_data).unwrap();
    let tasks = loaded_map.get("default").expect("default list");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].title, "ipc");

    // Add blocklist title keyword "ipc"
    execute_ipc_command(&[
        "ipc".to_string(),
        "blocklist".to_string(),
        "add-title".to_string(),
        "ipc".to_string(),
    ])
    .await
    .expect("add-title ipc");

    let config = load_blocklist();
    assert!(config.title_keywords.contains(&"ipc".to_string()));
}
