use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    TasksList { list_id: Option<String> },
    TaskComplete { task_id: String },
    TaskCreate { title: String, list_id: Option<String>, parent_id: Option<String> },
    TaskFocus { task_id: Option<String> },
    Sync,
    BlocklistGet,
    BlocklistToggleStrict,
    BlocklistToggleAntiDistraction,
    BlocklistSetAction { action: String },
}

pub fn get_config_dir() -> PathBuf {
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
        let temp_dir = std::env::temp_dir().join(format!("pomotask_test_{}", rand::random::<u64>()));
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
        let temp_dir = std::env::temp_dir().join(format!("pomotask_test_bl_{}", rand::random::<u64>()));
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
        let non_existent_path = std::path::PathBuf::from("/tmp/non_existent_pomotask_state_12345.json");
        let _ = std::fs::remove_file(&non_existent_path);
        let loaded = load_runtime_state_from(&non_existent_path);
        assert_eq!(loaded.state, "stopped");
        assert_eq!(loaded.remaining_seconds, 25 * 60);
    }
}
