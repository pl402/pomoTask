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
        auto_cycle: false,
        target_end_timestamp: None,
        google_connected: None,
        last_sync_at: None,
        last_sync_error: None,
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

    // Mode switch
    execute_ipc_command(&[
        "timer".to_string(),
        "mode".to_string(),
        "long_break".to_string(),
    ])
    .await
    .expect("timer mode long_break");
    let updated = load_runtime_state();
    assert_eq!(updated.mode, "long_break");
    assert_eq!(updated.state, "stopped");

    execute_ipc_command(&["timer".to_string(), "mode".to_string(), "work".to_string()])
        .await
        .expect("timer mode work");
    let updated = load_runtime_state();
    assert_eq!(updated.mode, "work");
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

    let mut initial_state = load_runtime_state();
    initial_state.active_task_id = Some("t1".to_string());
    initial_state.active_task_title = Some("Task 1".to_string());
    save_runtime_state(&initial_state).unwrap();

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

    let state = load_runtime_state();
    assert_eq!(state.active_task_id, None);
    assert_eq!(state.active_task_title, None);
}

#[tokio::test]
async fn test_ipc_task_complete_in_all_list() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("task_complete_all");
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let mut map: HashMap<String, Vec<Task>> = HashMap::new();
    map.insert(
        "@all".to_string(),
        vec![Task {
            id: "t_all_1".to_string(),
            list_id: "list_real".to_string(),
            title: "Task in All".to_string(),
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

    execute_ipc_command(&[
        "task".to_string(),
        "complete".to_string(),
        "t_all_1".to_string(),
    ])
    .await
    .expect("complete task in all");

    let cache_data = fs::read_to_string(&cache_path).unwrap();
    let loaded_map: HashMap<String, Vec<Task>> = serde_json::from_str(&cache_data).unwrap();
    let task = loaded_map
        .get("@all")
        .unwrap()
        .iter()
        .find(|t| t.id == "t_all_1")
        .unwrap();
    assert!(task.completed);
    assert!(task.completed_at.is_some());
}

#[tokio::test]
async fn test_ipc_task_create() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("task_create");
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let mut map: HashMap<String, Vec<Task>> = HashMap::new();
    map.insert("@all".to_string(), Vec::new());
    map.insert("personal".to_string(), Vec::new());
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

    let all_tasks = loaded_map.get("@all").expect("@all list");
    assert_eq!(all_tasks.len(), 1);
    assert_eq!(all_tasks[0].title, "Buy milk");
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

    // add-allowed-title & remove-allowed-title
    execute_ipc_command(&[
        "blocklist".to_string(),
        "add-allowed-title".to_string(),
        "spotify web".to_string(),
    ])
    .await
    .expect("add-allowed-title");
    let cfg_with_allowed = load_blocklist();
    assert!(cfg_with_allowed
        .allowed_title_keywords
        .contains(&"spotify web".to_string()));

    execute_ipc_command(&[
        "blocklist".to_string(),
        "remove-allowed-title".to_string(),
        "spotify web".to_string(),
    ])
    .await
    .expect("remove-allowed-title");
    let cfg_after_remove = load_blocklist();
    assert!(!cfg_after_remove
        .allowed_title_keywords
        .contains(&"spotify web".to_string()));
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

#[tokio::test]
async fn test_ipc_target_end_timestamp_and_countdown() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("target_end_ts");
    let mut state = RuntimeState::default();
    state.state = "stopped".to_string();
    state.mode = "work".to_string();
    state.remaining_seconds = 1500;
    state.total_seconds = 1500;
    save_runtime_state(&state).unwrap();

    // Start timer: target_end_timestamp should be set
    execute_ipc_command(&["timer".to_string(), "start".to_string()])
        .await
        .expect("timer start");
    let started = load_runtime_state();
    assert_eq!(started.state, "running");
    assert!(started.target_end_timestamp.is_some());
    let target = started.target_end_timestamp.unwrap();
    let now = Utc::now().timestamp();
    assert!(target >= now + 1495 && target <= now + 1505);

    // Simulate 50 seconds passing by modifying target_end_timestamp
    let mut simulated = load_runtime_state();
    simulated.target_end_timestamp = Some(now + 1450);
    save_runtime_state(&simulated).unwrap();

    // Loading should dynamically calculate remaining_seconds = ~1450
    let reloaded = load_runtime_state();
    assert!(reloaded.remaining_seconds >= 1448 && reloaded.remaining_seconds <= 1452);

    // Pause timer: target_end_timestamp should become None and remaining_seconds frozen
    execute_ipc_command(&["timer".to_string(), "pause".to_string()])
        .await
        .expect("timer pause");
    let paused = load_runtime_state();
    assert_eq!(paused.state, "paused");
    assert_eq!(paused.target_end_timestamp, None);
    assert!(paused.remaining_seconds >= 1448 && paused.remaining_seconds <= 1452);
}

#[tokio::test]
async fn test_ipc_headless_stats_recording() {
    let _lock = TEST_LOCK.lock().await;
    let ctx = TestContext::new("headless_stats");
    let mut state = RuntimeState::default();
    state.state = "running".to_string();
    state.mode = "work".to_string();
    state.remaining_seconds = 1500;
    state.total_seconds = 1500;
    state.active_task_id = Some("active_task_1".to_string());
    save_runtime_state(&state).unwrap();

    // Skip should transition work -> short_break and record pomodoro in stats.json
    execute_ipc_command(&["timer".to_string(), "skip".to_string()])
        .await
        .expect("timer skip");

    let stats_path = ctx.temp_dir.join("stats.json");
    assert!(stats_path.exists());
    let stats_data = fs::read_to_string(&stats_path).unwrap();
    let stats: pomotask_cli::app::Stats = serde_json::from_str(&stats_data).unwrap();
    assert_eq!(stats.lifetime_pomodoros, 1);
    assert_eq!(stats.task_pomodoros.get("active_task_1"), Some(&1));

    // Also complete task should record lifetime_tasks_done
    let cache_path = ctx.temp_dir.join("tasks_cache.json");
    let mut map: HashMap<String, Vec<Task>> = HashMap::new();
    map.insert(
        "default".to_string(),
        vec![Task {
            id: "t_done".to_string(),
            list_id: "default".to_string(),
            title: "Task Done".to_string(),
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

    execute_ipc_command(&[
        "task".to_string(),
        "complete".to_string(),
        "t_done".to_string(),
    ])
    .await
    .expect("task complete");

    let stats_data_2 = fs::read_to_string(&stats_path).unwrap();
    let stats_2: pomotask_cli::app::Stats = serde_json::from_str(&stats_data_2).unwrap();
    assert_eq!(stats_2.lifetime_tasks_done, 1);
}

#[tokio::test]
async fn test_ipc_lists_command() {
    use pomotask_cli::app::TaskList;
    use pomotask_cli::ipc::save_task_lists_cache;

    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("lists_command");

    let sample_lists = vec![
        TaskList {
            id: "@all".to_string(),
            title: "Todas las listas".to_string(),
        },
        TaskList {
            id: "list_work".to_string(),
            title: "Trabajo".to_string(),
        },
    ];
    save_task_lists_cache(&sample_lists).unwrap();

    let output = execute_ipc_command(&["lists".to_string()])
        .await
        .expect("ipc lists");
    let parsed: Vec<TaskList> = serde_json::from_str(&output).expect("parse lists output");
    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[1].title, "Trabajo");

    let output2 = execute_ipc_command(&["tasks".to_string(), "lists".to_string()])
        .await
        .expect("ipc tasks lists");
    let parsed2: Vec<TaskList> = serde_json::from_str(&output2).expect("parse tasks lists output");
    assert_eq!(parsed2.len(), 2);
    assert_eq!(parsed2[1].title, "Trabajo");
}

#[tokio::test]
async fn test_ipc_auth_status_without_token_marks_disconnected() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("auth_status_no_token");

    let output = execute_ipc_command(&["auth-status".to_string()])
        .await
        .expect("auth-status must not fail without token");
    let parsed: RuntimeState = serde_json::from_str(&output).expect("parse runtime state JSON");
    assert_eq!(parsed.google_connected, Some(false));
    assert!(parsed
        .last_sync_error
        .as_deref()
        .unwrap_or("")
        .starts_with("no_token"));

    // El estado persistido también refleja la desconexión.
    let persisted = load_runtime_state();
    assert_eq!(persisted.google_connected, Some(false));
    assert!(persisted.last_sync_at.is_none());
}

#[tokio::test]
async fn test_ipc_sync_without_token_is_skipped_and_marks_disconnected() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("sync_no_token");

    let output = execute_ipc_command(&["sync".to_string()])
        .await
        .expect("sync without token is skipped, not an error");
    assert!(output.contains("Sync skipped"));
    let persisted = load_runtime_state();
    assert_eq!(persisted.google_connected, Some(false));
}

#[tokio::test]
async fn test_ipc_offline_changes_go_to_outbox_and_survive_in_cache() {
    use pomotask_cli::outbox::{load_outbox, PendingOp};

    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("outbox_offline");

    // Sin token: crear una tarea la deja en caché y en el buzón; el modo local no es un error.
    let res = execute_ipc_command(&[
        "task".to_string(),
        "create".to_string(),
        "--title".to_string(),
        "Tarea sin conexión".to_string(),
        "--list-id".to_string(),
        "lista_x".to_string(),
    ])
    .await;
    let created: Task =
        serde_json::from_str(&res.expect("sin sesión, crear es válido en modo local")).unwrap();
    assert_eq!(created.title, "Tarea sin conexión");

    let ops = load_outbox();
    assert_eq!(ops.len(), 1);
    let temp_id = match &ops[0] {
        PendingOp::Create {
            temp_id,
            list_id,
            title,
            ..
        } => {
            assert_eq!(list_id, "lista_x");
            assert_eq!(title, "Tarea sin conexión");
            temp_id.clone()
        }
        other => panic!("se esperaba Create, había {:?}", other),
    };
    assert!(temp_id.starts_with("task_"));

    // La tarea está en la caché local y se lista.
    let listed = execute_ipc_command(&["tasks".to_string(), "list".to_string()])
        .await
        .expect("tasks list");
    let tasks: Vec<Task> = serde_json::from_str(&listed).unwrap();
    assert!(tasks.iter().any(|t| t.id == temp_id));

    // Completarla offline encola un Complete detrás del Create y no requiere sesión.
    let out = execute_ipc_command(&["task".to_string(), "complete".to_string(), temp_id.clone()])
        .await
        .expect("completar una tarea local nunca falla");
    assert!(out.contains("queued"));
    let ops = load_outbox();
    assert_eq!(ops.len(), 2);
    assert!(matches!(&ops[0], PendingOp::Create { .. }));
    assert!(matches!(&ops[1], PendingOp::Complete { task_id, .. } if task_id == &temp_id));

    // El comando `outbox` expone lo pendiente.
    let dump = execute_ipc_command(&["outbox".to_string()])
        .await
        .expect("outbox");
    let parsed: Vec<PendingOp> = serde_json::from_str(&dump).unwrap();
    assert_eq!(parsed, ops);
}

#[tokio::test]
async fn test_ipc_config_durations() {
    let _lock = TEST_LOCK.lock().await;
    let _ctx = TestContext::new("config");

    // Sin config.json: `get` devuelve los valores por defecto.
    let out = execute_ipc_command(&["config".to_string(), "get".to_string()])
        .await
        .expect("config get");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["focus_duration"], 25 * 60);
    assert_eq!(v["short_break_duration"], 5 * 60);
    assert_eq!(v["long_break_duration"], 15 * 60);

    // Temporizador detenido en modo trabajo: al cambiar el enfoque se refleja en el estado.
    let state = RuntimeState {
        mode: "work".to_string(),
        state: "stopped".to_string(),
        total_seconds: 25 * 60,
        remaining_seconds: 25 * 60,
        ..RuntimeState::default()
    };
    save_runtime_state(&state).unwrap();

    let out = execute_ipc_command(&[
        "config".to_string(),
        "set".to_string(),
        "focus".to_string(),
        "50".to_string(),
    ])
    .await
    .expect("config set focus");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["focus_duration"], 50 * 60);
    let after = load_runtime_state();
    assert_eq!(after.total_seconds, 50 * 60);
    assert_eq!(after.remaining_seconds, 50 * 60);

    // Cambiar el descanso corto no toca el estado (modo trabajo), pero sí persiste.
    execute_ipc_command(&[
        "config".to_string(),
        "set".to_string(),
        "short".to_string(),
        "7".to_string(),
    ])
    .await
    .expect("config set short");
    let after2 = load_runtime_state();
    assert_eq!(after2.total_seconds, 50 * 60);
    let out = execute_ipc_command(&["config".to_string(), "get".to_string()])
        .await
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["short_break_duration"], 7 * 60);

    // El config.json resultante sigue siendo una Config completa para la TUI.
    let raw = fs::read_to_string(_ctx.temp_dir.join("config.json")).unwrap();
    let cfg: pomotask_cli::app::Config = serde_json::from_str(&raw).expect("Config completa");
    assert_eq!(cfg.focus_duration, 50 * 60);

    // Validaciones.
    assert!(execute_ipc_command(&[
        "config".to_string(),
        "set".to_string(),
        "focus".to_string(),
        "0".to_string(),
    ])
    .await
    .is_err());
    assert!(execute_ipc_command(&[
        "config".to_string(),
        "set".to_string(),
        "nada".to_string(),
        "5".to_string(),
    ])
    .await
    .is_err());
}
