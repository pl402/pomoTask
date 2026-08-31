use crate::api::ApiClient;
use crate::app::Task;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeState {
    pub state: String, // "running", "paused", "stopped"
    pub mode: String,  // "work", "short_break", "long_break"
    pub remaining_seconds: u32,
    pub total_seconds: u32,
    pub session_pomodoros: u32,
    pub active_task_id: Option<String>,
    pub active_task_title: Option<String>,
    pub strict_break: bool,
    pub anti_distraction: bool,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            state: "stopped".to_string(),
            mode: "work".to_string(),
            remaining_seconds: 25 * 60,
            total_seconds: 25 * 60,
            session_pomodoros: 0,
            active_task_id: None,
            active_task_title: None,
            strict_break: false,
            anti_distraction: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlocklistConfig {
    pub title_keywords: Vec<String>,
    pub blocked_classes: Vec<String>,
    pub action: String, // "warn", "warn_and_unfocus", "minimize"
}

impl Default for BlocklistConfig {
    fn default() -> Self {
        Self {
            title_keywords: vec![
                "facebook".to_string(),
                "twitter".to_string(),
                "x.com".to_string(),
                "instagram".to_string(),
                "reddit".to_string(),
                "youtube.com".to_string(),
                "tiktok".to_string(),
                "netflix".to_string(),
            ],
            blocked_classes: vec![
                "steam".to_string(),
                "discord".to_string(),
                "spotify".to_string(),
            ],
            action: "warn".to_string(),
        }
    }
}

impl BlocklistConfig {
    pub fn is_distraction(&self, title: &str, app_class: &str) -> bool {
        let title_lower = title.to_lowercase();
        let class_lower = app_class.to_lowercase();

        for kw in &self.title_keywords {
            if title_lower.contains(&kw.to_lowercase()) {
                return true;
            }
        }
        for cls in &self.blocked_classes {
            let cls_lower = cls.to_lowercase();
            if class_lower == cls_lower || class_lower.contains(&cls_lower) {
                return true;
            }
        }
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TimerCommand {
    Start,
    Pause,
    Toggle,
    Skip,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcCommand {
    Status,
    Timer(TimerCommand),
    TasksList {
        list_id: Option<String>,
    },
    TaskComplete {
        task_id: String,
    },
    TaskCreate {
        title: String,
        list_id: Option<String>,
        parent_id: Option<String>,
    },
    TaskFocus {
        task_id: Option<String>,
    },
    Sync,
    BlocklistGet,
    BlocklistToggleStrict,
    BlocklistToggleAntiDistraction,
    BlocklistSetAction {
        action: String,
    },
    BlocklistAddTitle {
        keyword: String,
    },
    BlocklistAddClass {
        class_name: String,
    },
    BlocklistRemoveTitle {
        keyword: String,
    },
    BlocklistRemoveClass {
        class_name: String,
    },
}

pub fn get_config_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("POMOTASK_CONFIG_DIR") {
        let path = PathBuf::from(dir);
        let _ = fs::create_dir_all(&path);
        return path;
    }
    let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("pomotask");
    let _ = fs::create_dir_all(&path);
    path
}

pub fn get_runtime_state_path() -> PathBuf {
    get_config_dir().join("runtime_state.json")
}

pub fn get_blocklist_path() -> PathBuf {
    get_config_dir().join("blocklist.json")
}

pub fn get_tasks_cache_path() -> PathBuf {
    get_config_dir().join("tasks_cache.json")
}

pub fn load_runtime_state() -> RuntimeState {
    load_runtime_state_from(&get_runtime_state_path())
}

pub fn load_runtime_state_from(path: &Path) -> RuntimeState {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(state) = serde_json::from_str(&data) {
            return state;
        }
    }
    RuntimeState::default()
}

pub fn save_runtime_state(state: &RuntimeState) -> std::io::Result<()> {
    save_runtime_state_to(state, &get_runtime_state_path())
}

pub fn save_runtime_state_to(state: &RuntimeState, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, data)
}

pub fn load_blocklist() -> BlocklistConfig {
    load_blocklist_from(&get_blocklist_path())
}

pub fn load_blocklist_from(path: &Path) -> BlocklistConfig {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(config) = serde_json::from_str(&data) {
            return config;
        }
    }
    BlocklistConfig::default()
}

pub fn save_blocklist(config: &BlocklistConfig) -> std::io::Result<()> {
    save_blocklist_to(config, &get_blocklist_path())
}

pub fn save_blocklist_to(config: &BlocklistConfig, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, data)
}

pub fn load_tasks_cache() -> HashMap<String, Vec<Task>> {
    load_tasks_cache_from(&get_tasks_cache_path())
}

pub fn load_tasks_cache_from(path: &Path) -> HashMap<String, Vec<Task>> {
    let data = match fs::read_to_string(path) {
        Ok(d) => d,
        Err(_) => return HashMap::new(),
    };

    if let Ok(map) = serde_json::from_str::<HashMap<String, Vec<Task>>>(&data) {
        return map;
    }

    if let Ok(list) = serde_json::from_str::<Vec<Task>>(&data) {
        let mut map: HashMap<String, Vec<Task>> = HashMap::new();
        for t in &list {
            map.entry(t.list_id.clone()).or_default().push(t.clone());
        }
        if !list.is_empty() {
            map.insert("@all".to_string(), list);
        }
        return map;
    }

    HashMap::new()
}

pub fn save_tasks_cache(cache: &HashMap<String, Vec<Task>>) -> std::io::Result<()> {
    save_tasks_cache_to(cache, &get_tasks_cache_path())
}

pub fn save_tasks_cache_to(cache: &HashMap<String, Vec<Task>>, path: &Path) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let data = serde_json::to_string_pretty(cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    fs::write(path, data)
}

pub fn load_config_durations() -> (u32, u32, u32) {
    let path = get_config_dir().join("config.json");
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(cfg) = serde_json::from_str::<serde_json::Value>(&data) {
            let focus = cfg
                .get("focus_duration")
                .and_then(|v| v.as_u64())
                .unwrap_or(25 * 60) as u32;
            let short = cfg
                .get("short_break_duration")
                .and_then(|v| v.as_u64())
                .unwrap_or(5 * 60) as u32;
            let long = cfg
                .get("long_break_duration")
                .and_then(|v| v.as_u64())
                .unwrap_or(15 * 60) as u32;
            return (focus, short, long);
        }
    }
    (25 * 60, 5 * 60, 15 * 60)
}

pub async fn execute_ipc_command(args: &[String]) -> Result<String, String> {
    let clean_args: Vec<&str> = args
        .iter()
        .map(|s| s.as_str())
        .filter(|&s| s != "ipc" && s != "--ipc")
        .collect();

    if clean_args.is_empty() {
        return Err("No IPC subcommand provided. Use 'pomotask-cli ipc --help'".to_string());
    }

    match clean_args[0] {
        "status" => {
            let state = load_runtime_state();
            serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
        }
        "timer" => {
            if clean_args.len() < 2 {
                return Err("Missing timer action: start, pause, toggle, skip, reset".to_string());
            }
            let (focus_dur, short_dur, long_dur) = load_config_durations();
            let mut state = load_runtime_state();

            match clean_args[1] {
                "start" => {
                    state.state = "running".to_string();
                }
                "pause" => {
                    state.state = "paused".to_string();
                }
                "toggle" => {
                    if state.state == "running" {
                        state.state = "paused".to_string();
                    } else {
                        state.state = "running".to_string();
                    }
                }
                "skip" => {
                    if state.mode == "work" {
                        state.session_pomodoros += 1;
                        if state.session_pomodoros.is_multiple_of(4) {
                            state.mode = "long_break".to_string();
                            state.total_seconds = long_dur;
                        } else {
                            state.mode = "short_break".to_string();
                            state.total_seconds = short_dur;
                        }
                    } else {
                        state.mode = "work".to_string();
                        state.total_seconds = focus_dur;
                    }
                    state.remaining_seconds = state.total_seconds;
                    state.state = "stopped".to_string();
                }
                "reset" => {
                    state.remaining_seconds = state.total_seconds;
                    state.state = "stopped".to_string();
                }
                other => {
                    return Err(format!(
                        "Unknown timer command: '{}'. Expected start, pause, toggle, skip, reset",
                        other
                    ));
                }
            }

            save_runtime_state(&state).map_err(|e| e.to_string())?;
            serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
        }
        "tasks" => {
            if clean_args.len() > 1 && clean_args[1] != "list" {
                return Err(format!(
                    "Unknown tasks command: '{}'. Expected 'list'",
                    clean_args[1]
                ));
            }

            let mut list_id_filter: Option<String> = None;
            let mut i = 1;
            while i < clean_args.len() {
                if clean_args[i] == "--list-id" && i + 1 < clean_args.len() {
                    list_id_filter = Some(clean_args[i + 1].to_string());
                    i += 2;
                } else if let Some(stripped) = clean_args[i].strip_prefix("--list-id=") {
                    list_id_filter = Some(stripped.to_string());
                    i += 1;
                } else {
                    i += 1;
                }
            }

            let cache = load_tasks_cache();
            let tasks: Vec<Task> = match list_id_filter {
                Some(lid) if lid != "@all" => cache.get(&lid).cloned().unwrap_or_default(),
                _ => {
                    let mut all = Vec::new();
                    for (lid, mut list_tasks) in cache {
                        if lid != "@all" {
                            all.append(&mut list_tasks);
                        }
                    }
                    all
                }
            };

            serde_json::to_string_pretty(&tasks).map_err(|e| e.to_string())
        }
        "task" => {
            if clean_args.len() < 2 {
                return Err("Missing task action: complete, create, focus".to_string());
            }

            match clean_args[1] {
                "complete" => {
                    if clean_args.len() < 3 {
                        return Err("Missing task ID to complete".to_string());
                    }
                    let task_id = clean_args[2];
                    let mut cache = load_tasks_cache();
                    let mut found_list_id: Option<String> = None;

                    for (lid, list_tasks) in cache.iter_mut() {
                        for t in list_tasks.iter_mut() {
                            if t.id == task_id {
                                t.completed = true;
                                t.completed_at = Some(Utc::now());
                                if found_list_id.is_none() && lid != "@all" {
                                    found_list_id = Some(t.list_id.clone());
                                }
                            }
                        }
                    }

                    save_tasks_cache(&cache).map_err(|e| e.to_string())?;

                    // Intentar sincronizar con Google Tasks API en segundo plano si el token existe
                    if get_config_dir().join("pomotask_token.json").exists() {
                        if let Some(lid) = found_list_id {
                            let (sender, _) = tokio::sync::mpsc::unbounded_channel();
                            let client = ApiClient::new(sender).await;
                            let _ = tokio::time::timeout(
                                Duration::from_secs(5),
                                client.toggle_task_completion(&lid, task_id, true),
                            )
                            .await;
                        }
                    }

                    Ok(format!("Task {} marked as completed", task_id))
                }
                "create" => {
                    let mut title: Option<String> = None;
                    let mut list_id: Option<String> = None;
                    let mut parent: Option<String> = None;

                    let mut i = 2;
                    while i < clean_args.len() {
                        let arg = clean_args[i];
                        if arg == "--title" && i + 1 < clean_args.len() {
                            title = Some(clean_args[i + 1].to_string());
                            i += 2;
                        } else if let Some(stripped) = arg.strip_prefix("--title=") {
                            title = Some(stripped.to_string());
                            i += 1;
                        } else if arg == "--list-id" && i + 1 < clean_args.len() {
                            list_id = Some(clean_args[i + 1].to_string());
                            i += 2;
                        } else if let Some(stripped) = arg.strip_prefix("--list-id=") {
                            list_id = Some(stripped.to_string());
                            i += 1;
                        } else if arg == "--parent" && i + 1 < clean_args.len() {
                            parent = Some(clean_args[i + 1].to_string());
                            i += 2;
                        } else if let Some(stripped) = arg.strip_prefix("--parent=") {
                            parent = Some(stripped.to_string());
                            i += 1;
                        } else {
                            if title.is_none() && !arg.starts_with('-') {
                                title = Some(arg.to_string());
                            }
                            i += 1;
                        }
                    }

                    let title_val = title.ok_or_else(|| "Missing required --title".to_string())?;
                    let mut cache = load_tasks_cache();
                    let target_list_id = list_id.unwrap_or_else(|| {
                        cache
                            .keys()
                            .find(|k| k.as_str() != "@all")
                            .cloned()
                            .unwrap_or_else(|| "@default".to_string())
                    });

                    let temp_id = format!("task_{}", rand::random::<u32>());
                    let new_task = Task {
                        id: temp_id.clone(),
                        list_id: target_list_id.clone(),
                        title: title_val.clone(),
                        completed: false,
                        due: None,
                        updated: Utc::now(),
                        completed_at: None,
                        notes: None,
                        parent_id: parent.clone(),
                        pomodoros: 0,
                    };

                    cache
                        .entry(target_list_id.clone())
                        .or_default()
                        .push(new_task.clone());
                    save_tasks_cache(&cache).map_err(|e| e.to_string())?;

                    // Intentar crear tarea en Google Tasks API si el token existe
                    if get_config_dir().join("pomotask_token.json").exists() {
                        let (sender, _) = tokio::sync::mpsc::unbounded_channel();
                        let client = ApiClient::new(sender).await;
                        let _ = tokio::time::timeout(
                            Duration::from_secs(5),
                            client.create_task(&target_list_id, &title_val, None, None, parent),
                        )
                        .await;
                    }

                    serde_json::to_string_pretty(&new_task).map_err(|e| e.to_string())
                }
                "focus" => {
                    if clean_args.len() < 3 {
                        return Err("Missing task ID or 'clear' for focus command".to_string());
                    }
                    let target = clean_args[2];
                    let mut state = load_runtime_state();

                    if target == "clear" {
                        state.active_task_id = None;
                        state.active_task_title = None;
                    } else {
                        let cache = load_tasks_cache();
                        let title = cache
                            .values()
                            .flatten()
                            .find(|t| t.id == target)
                            .map(|t| t.title.clone())
                            .unwrap_or_else(|| target.to_string());

                        state.active_task_id = Some(target.to_string());
                        state.active_task_title = Some(title);
                    }

                    save_runtime_state(&state).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
                }
                other => Err(format!(
                    "Unknown task subcommand: '{}'. Expected complete, create, focus",
                    other
                )),
            }
        }
        "blocklist" => {
            if clean_args.len() < 2 {
                return Err(
                    "Missing blocklist action: get, toggle-strict, toggle-anti-distraction, set-action, add-title, add-class, remove-title, remove-class".to_string(),
                );
            }

            match clean_args[1] {
                "get" => {
                    let config = load_blocklist();
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "toggle-strict" => {
                    let mut state = load_runtime_state();
                    state.strict_break = !state.strict_break;
                    save_runtime_state(&state).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
                }
                "toggle-anti-distraction" => {
                    let mut state = load_runtime_state();
                    state.anti_distraction = !state.anti_distraction;
                    save_runtime_state(&state).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
                }
                "set-action" => {
                    if clean_args.len() < 3 {
                        return Err("Missing action parameter (warn, warn_and_unfocus, minimize)".to_string());
                    }
                    let mut config = load_blocklist();
                    config.action = clean_args[2].to_string();
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "add-title" => {
                    if clean_args.len() < 3 {
                        return Err("Missing keyword to add".to_string());
                    }
                    let mut config = load_blocklist();
                    let kw = clean_args[2].to_string();
                    if !config.title_keywords.contains(&kw) {
                        config.title_keywords.push(kw);
                    }
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "add-class" => {
                    if clean_args.len() < 3 {
                        return Err("Missing class name to add".to_string());
                    }
                    let mut config = load_blocklist();
                    let cls = clean_args[2].to_string();
                    if !config.blocked_classes.contains(&cls) {
                        config.blocked_classes.push(cls);
                    }
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "remove-title" => {
                    if clean_args.len() < 3 {
                        return Err("Missing keyword to remove".to_string());
                    }
                    let mut config = load_blocklist();
                    let kw = clean_args[2];
                    config.title_keywords.retain(|k| k != kw);
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "remove-class" => {
                    if clean_args.len() < 3 {
                        return Err("Missing class name to remove".to_string());
                    }
                    let mut config = load_blocklist();
                    let cls = clean_args[2];
                    config.blocked_classes.retain(|c| c != cls);
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                other => Err(format!(
                    "Unknown blocklist command: '{}'. Expected get, toggle-strict, toggle-anti-distraction, set-action, add-title, add-class, remove-title, remove-class",
                    other
                )),
            }
        }
        "sync" => {
            if !get_config_dir().join("pomotask_token.json").exists() {
                return Ok("Sync skipped: No active Google session token found".to_string());
            }
            let (sender, _) = tokio::sync::mpsc::unbounded_channel();
            let client = ApiClient::new(sender).await;
            let lists = tokio::time::timeout(Duration::from_secs(10), client.fetch_task_lists())
                .await
                .map_err(|_| "Sync timed out fetching lists".to_string())?
                .map_err(|e| e.to_string())?;
            let mut cache: HashMap<String, Vec<Task>> = HashMap::new();
            let mut all_tasks = Vec::new();

            for list in lists {
                if list.id == "@all" {
                    continue;
                }
                let tasks = client.fetch_tasks(&list.id, true).await.unwrap_or_default();
                all_tasks.extend(tasks.clone());
                cache.insert(list.id, tasks);
            }
            if !all_tasks.is_empty() {
                cache.insert("@all".to_string(), all_tasks);
            }

            save_tasks_cache(&cache).map_err(|e| e.to_string())?;
            Ok("Sync completed successfully".to_string())
        }
        other => Err(format!("Unknown IPC command: '{}'", other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_state_serialization() {
        let state = RuntimeState {
            state: "running".to_string(),
            mode: "work".to_string(),
            remaining_seconds: 1500,
            total_seconds: 1500,
            session_pomodoros: 2,
            active_task_id: Some("task_123".to_string()),
            active_task_title: Some("Implement IPC".to_string()),
            strict_break: false,
            anti_distraction: true,
        };
        let json = serde_json::to_string(&state).expect("serialize");
        let decoded: RuntimeState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.state, "running");
        assert_eq!(decoded.mode, "work");
        assert_eq!(decoded.remaining_seconds, 1500);
        assert_eq!(decoded.total_seconds, 1500);
        assert_eq!(decoded.session_pomodoros, 2);
        assert_eq!(decoded.active_task_id.as_deref(), Some("task_123"));
        assert_eq!(decoded.active_task_title.as_deref(), Some("Implement IPC"));
        assert!(!decoded.strict_break);
        assert!(decoded.anti_distraction);
    }

    #[test]
    fn test_runtime_state_default() {
        let default_state = RuntimeState::default();
        assert_eq!(default_state.state, "stopped");
        assert_eq!(default_state.mode, "work");
        assert_eq!(default_state.remaining_seconds, 25 * 60);
        assert_eq!(default_state.total_seconds, 25 * 60);
        assert_eq!(default_state.session_pomodoros, 0);
        assert_eq!(default_state.active_task_id, None);
        assert_eq!(default_state.active_task_title, None);
        assert!(!default_state.strict_break);
        assert!(default_state.anti_distraction);
    }

    #[test]
    fn test_blocklist_matching() {
        let mut blocklist = BlocklistConfig::default();
        blocklist.title_keywords.push("facebook".to_string());
        blocklist.blocked_classes.push("steam".to_string());

        assert!(blocklist.is_distraction("Facebook - Log In", "firefox"));
        assert!(blocklist.is_distraction("Any Title", "steam"));
        assert!(blocklist.is_distraction("Twitter / X", "google-chrome"));
        assert!(!blocklist.is_distraction("GitHub - Pull Requests", "zen"));
        assert!(!blocklist.is_distraction("Rust Docs", "firefox"));
    }

    #[test]
    fn test_blocklist_serialization() {
        let blocklist = BlocklistConfig::default();
        let json = serde_json::to_string_pretty(&blocklist).expect("serialize");
        let decoded: BlocklistConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.action, "warn");
        assert!(decoded.title_keywords.contains(&"reddit".to_string()));
        assert!(decoded.blocked_classes.contains(&"discord".to_string()));
    }

    #[test]
    fn test_save_and_load_runtime_state() {
        let temp_dir =
            std::env::temp_dir().join(format!("pomotask_test_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let state_path = temp_dir.join("runtime_state.json");

        let state = RuntimeState {
            state: "paused".to_string(),
            mode: "short_break".to_string(),
            remaining_seconds: 240,
            total_seconds: 300,
            session_pomodoros: 4,
            active_task_id: Some("t1".to_string()),
            active_task_title: Some("My Task".to_string()),
            strict_break: true,
            anti_distraction: false,
        };

        save_runtime_state_to(&state, &state_path).unwrap();
        let loaded = load_runtime_state_from(&state_path);

        assert_eq!(loaded.state, "paused");
        assert_eq!(loaded.mode, "short_break");
        assert_eq!(loaded.remaining_seconds, 240);
        assert_eq!(loaded.total_seconds, 300);
        assert_eq!(loaded.session_pomodoros, 4);
        assert_eq!(loaded.active_task_id.as_deref(), Some("t1"));
        assert_eq!(loaded.active_task_title.as_deref(), Some("My Task"));
        assert!(loaded.strict_break);
        assert!(!loaded.anti_distraction);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_save_and_load_blocklist() {
        let temp_dir =
            std::env::temp_dir().join(format!("pomotask_test_bl_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let bl_path = temp_dir.join("blocklist.json");

        let mut config = BlocklistConfig::default();
        config.action = "warn_and_unfocus".to_string();
        config.blocked_classes.push("vlc".to_string());

        save_blocklist_to(&config, &bl_path).unwrap();
        let loaded = load_blocklist_from(&bl_path);

        assert_eq!(loaded.action, "warn_and_unfocus");
        assert!(loaded.blocked_classes.contains(&"vlc".to_string()));

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_load_runtime_state_missing_file_returns_default() {
        let non_existent_path =
            std::path::PathBuf::from("/tmp/non_existent_pomotask_state_12345.json");
        let _ = std::fs::remove_file(&non_existent_path);
        let loaded = load_runtime_state_from(&non_existent_path);
        assert_eq!(loaded.state, "stopped");
        assert_eq!(loaded.remaining_seconds, 25 * 60);
    }
}
