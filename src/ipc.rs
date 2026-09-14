use crate::api::ApiClient;
use crate::app::{Stats, Task, TaskList};
use crate::events::Event;
use crate::outbox::{self, PendingOp};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::sync::mpsc;

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
    #[serde(default)]
    pub target_end_timestamp: Option<i64>,
    /// Estado de la conexión con Google Tasks. `None` = aún no se ha comprobado.
    #[serde(default)]
    pub google_connected: Option<bool>,
    /// Timestamp (segundos Unix) de la última sincronización completa exitosa.
    #[serde(default)]
    pub last_sync_at: Option<i64>,
    /// Último error de red/autenticación. Empieza por `auth_required` si hay que volver a iniciar sesión.
    #[serde(default)]
    pub last_sync_error: Option<String>,
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
            target_end_timestamp: None,
            google_connected: None,
            last_sync_at: None,
            last_sync_error: None,
        }
    }
}

/// Mensaje estable que el plugin detecta (prefijo `auth_required`) para ofrecer re-autenticación.
pub const AUTH_REQUIRED_MSG: &str =
    "auth_required: Google session expired or revoked. Open the PomoTask TUI (pomotask-cli) to sign in again";

/// Marca la conexión con Google como activa. Si `synced` es true, también registra la hora de sincronización.
pub fn mark_google_connected(synced: bool) {
    let mut state = load_runtime_state();
    state.google_connected = Some(true);
    state.last_sync_error = None;
    if synced {
        state.last_sync_at = Some(Utc::now().timestamp());
    }
    let _ = save_runtime_state(&state);
}

/// Marca la conexión con Google como perdida y guarda el motivo para que el plugin lo muestre.
pub fn mark_google_error(error: &str) {
    let mut state = load_runtime_state();
    state.google_connected = Some(false);
    state.last_sync_error = Some(error.to_string());
    let _ = save_runtime_state(&state);
}

/// Cliente de API para el modo IPC (headless). Conservamos el receptor de eventos para detectar
/// cuándo yup_oauth2 intenta abrir el flujo interactivo de login (`Event::NeedsAuth`), que aquí
/// nadie puede atender: sin esto el comando se quedaría colgado esperando al navegador.
async fn ipc_api_client() -> (ApiClient, mpsc::UnboundedReceiver<Event>) {
    let (sender, receiver) = mpsc::unbounded_channel();
    let client = ApiClient::new(sender).await;
    (client, receiver)
}

/// Espera hasta que el autenticador pida login interactivo. Si el canal se cierra, no resuelve nunca.
async fn wait_needs_auth(rx: &mut mpsc::UnboundedReceiver<Event>) {
    loop {
        match rx.recv().await {
            Some(Event::NeedsAuth(_)) => return,
            Some(_) => continue,
            None => std::future::pending::<()>().await,
        }
    }
}

/// Ejecuta una llamada a la API con timeout y aborta de inmediato con `AUTH_REQUIRED_MSG`
/// si el flujo OAuth necesita al usuario (token expirado o revocado).
async fn with_auth_guard<T, F>(
    rx: &mut mpsc::UnboundedReceiver<Event>,
    timeout_secs: u64,
    what: &str,
    fut: F,
) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, Box<dyn std::error::Error + Send + Sync>>>,
{
    tokio::select! {
        res = tokio::time::timeout(Duration::from_secs(timeout_secs), fut) => match res {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(format!("{}: {}", what, e)),
            Err(_) => Err(format!("{}: timed out after {}s", what, timeout_secs)),
        },
        _ = wait_needs_auth(rx) => Err(AUTH_REQUIRED_MSG.to_string()),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlocklistConfig {
    pub title_keywords: Vec<String>,
    pub blocked_classes: Vec<String>,
    #[serde(default = "default_allowed_title_keywords")]
    pub allowed_title_keywords: Vec<String>,
    #[serde(default = "default_allowed_classes")]
    pub allowed_classes: Vec<String>,
    pub action: String, // "warn" (aviso discreto), "hud" (pantalla de enfoque), "minimize" (ocultar ventana)
    #[serde(default = "default_overlay_dimming")]
    pub overlay_dimming: f64, // 0.0 to 1.0 (default 0.40 = 60% background visibility)
}

fn default_overlay_dimming() -> f64 {
    0.40
}

fn default_allowed_title_keywords() -> Vec<String> {
    vec!["youtube music".to_string(), "music.youtube.com".to_string()]
}

fn default_allowed_classes() -> Vec<String> {
    vec![
        "youtube music".to_string(),
        "youtube-music".to_string(),
        "com.github.th_ch.youtube_music".to_string(),
    ]
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
                "youtube".to_string(),
                "tiktok".to_string(),
                "netflix".to_string(),
                "twitch".to_string(),
            ],
            blocked_classes: vec![
                "steam".to_string(),
                "discord".to_string(),
                "spotify".to_string(),
            ],
            allowed_title_keywords: default_allowed_title_keywords(),
            allowed_classes: default_allowed_classes(),
            action: "warn".to_string(),
            overlay_dimming: default_overlay_dimming(),
        }
    }
}

/// Normaliza la acción anti-distracción a uno de los tres valores que entiende el plugin.
/// Acepta los nombres antiguos (`warn_and_unfocus`, `unfocus`) como sinónimos de `hud`.
/// Devuelve `None` si el valor no se reconoce.
pub fn normalize_distraction_action(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "warn" => Some("warn"),
        "hud" | "warn_and_unfocus" | "unfocus" => Some("hud"),
        "minimize" => Some("minimize"),
        _ => None,
    }
}

impl BlocklistConfig {
    pub fn is_distraction(&self, title: &str, app_class: &str) -> bool {
        let title_lower = title.to_lowercase();
        let class_lower = app_class.to_lowercase();

        // 1. Verificar excepciones de la lista blanca primero (ej. YouTube Music)
        for allow_kw in &self.allowed_title_keywords {
            let kw_trimmed = allow_kw.trim();
            if !kw_trimmed.is_empty() && title_lower.contains(&kw_trimmed.to_lowercase()) {
                return false;
            }
        }
        for allow_cls in &self.allowed_classes {
            let cls_trimmed = allow_cls.trim();
            if !cls_trimmed.is_empty() {
                let cls_low = cls_trimmed.to_lowercase();
                if class_lower == cls_low || class_lower.contains(&cls_low) {
                    return false;
                }
            }
        }

        // 2. Verificar reglas de bloqueo
        for kw in &self.title_keywords {
            let kw_trimmed = kw.trim();
            if kw_trimmed.is_empty() {
                continue;
            }
            if title_lower.contains(&kw_trimmed.to_lowercase()) {
                return true;
            }
        }
        for cls in &self.blocked_classes {
            let cls_trimmed = cls.trim();
            if cls_trimmed.is_empty() {
                continue;
            }
            let cls_lower = cls_trimmed.to_lowercase();
            if class_lower == cls_lower {
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

pub fn get_lists_cache_path() -> PathBuf {
    get_config_dir().join("lists_cache.json")
}

pub fn load_task_lists_cache() -> Vec<TaskList> {
    load_task_lists_cache_from(&get_lists_cache_path())
}

pub fn load_task_lists_cache_from(path: &Path) -> Vec<TaskList> {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(lists) = serde_json::from_str::<Vec<TaskList>>(&data) {
            return lists;
        }
    }
    Vec::new()
}

pub fn save_task_lists_cache(lists: &[TaskList]) -> std::io::Result<()> {
    save_task_lists_cache_to(lists, &get_lists_cache_path())
}

pub fn save_task_lists_cache_to(lists: &[TaskList], path: &Path) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(lists)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write(path, &data)
}

pub fn atomic_write(path: &Path, content: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("file");
    let tmp_file_name = format!(".{}_{}.tmp", file_name, rand::random::<u64>());
    let tmp_path = path.with_file_name(tmp_file_name);

    fs::write(&tmp_path, content)?;
    if let Err(e) = fs::rename(&tmp_path, path) {
        let _ = fs::remove_file(&tmp_path);
        return Err(e);
    }
    Ok(())
}

pub fn record_headless_pomodoro(active_task_id: Option<&str>) {
    let path = get_config_dir().join("stats.json");
    let mut stats: Stats = if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Stats::default()
    };

    let hour_key = chrono::Local::now().format("%Y-%m-%d %H:00").to_string();
    let entry = stats.hourly_pomodoros.entry(hour_key).or_insert(0);
    *entry += 1;
    stats.lifetime_pomodoros += 1;

    if let Some(task_id) = active_task_id {
        let t_entry = stats.task_pomodoros.entry(task_id.to_string()).or_insert(0);
        *t_entry += 1;
    }

    if let Ok(data) = serde_json::to_string_pretty(&stats) {
        let _ = atomic_write(&path, &data);
    }
}

pub fn record_headless_task_done() {
    let path = get_config_dir().join("stats.json");
    let mut stats: Stats = if let Ok(data) = fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        Stats::default()
    };

    let hour_key = chrono::Local::now().format("%Y-%m-%d %H:00").to_string();
    let entry = stats.hourly_tasks_done.entry(hour_key).or_insert(0);
    *entry += 1;
    stats.lifetime_tasks_done += 1;

    if let Ok(data) = serde_json::to_string_pretty(&stats) {
        let _ = atomic_write(&path, &data);
    }
}

pub fn load_runtime_state() -> RuntimeState {
    load_runtime_state_from(&get_runtime_state_path())
}

pub fn load_runtime_state_from(path: &Path) -> RuntimeState {
    if let Ok(data) = fs::read_to_string(path) {
        if let Ok(mut state) = serde_json::from_str::<RuntimeState>(&data) {
            if state.state == "running" {
                if let Some(target) = state.target_end_timestamp {
                    let now = Utc::now().timestamp();
                    let diff = target - now;
                    if diff <= 0 {
                        let (focus_dur, short_dur, long_dur) = load_config_durations();
                        if state.mode == "work" {
                            state.session_pomodoros += 1;
                            record_headless_pomodoro(state.active_task_id.as_deref());
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
                        state.target_end_timestamp = None;
                        let _ = save_runtime_state_to(&state, path);
                    } else {
                        state.remaining_seconds = diff as u32;
                    }
                }
            }
            return state;
        }
    }
    RuntimeState::default()
}

pub fn save_runtime_state(state: &RuntimeState) -> std::io::Result<()> {
    save_runtime_state_to(state, &get_runtime_state_path())
}

pub fn save_runtime_state_to(state: &RuntimeState, path: &Path) -> std::io::Result<()> {
    let data = serde_json::to_string_pretty(state)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write(path, &data)
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
    let data = serde_json::to_string_pretty(config)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write(path, &data)
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
    let data = serde_json::to_string_pretty(cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    atomic_write(path, &data)
}

/// Escribe una duración (en segundos) en config.json conservando el resto de campos. Si el
/// archivo no existe o está corrupto se parte de la configuración por defecto de la TUI, para
/// que ésta siga pudiendo deserializarlo completo.
pub fn set_config_duration(key: &str, seconds: u32) -> Result<(), String> {
    let path = get_config_dir().join("config.json");
    let mut cfg: serde_json::Value = fs::read_to_string(&path)
        .ok()
        .and_then(|d| serde_json::from_str(&d).ok())
        .filter(|v: &serde_json::Value| v.is_object())
        .unwrap_or_else(|| {
            serde_json::to_value(crate::app::Config::default()).unwrap_or(serde_json::json!({}))
        });
    if let Some(obj) = cfg.as_object_mut() {
        obj.insert(key.to_string(), serde_json::json!(seconds));
    }
    let data = serde_json::to_string_pretty(&cfg).map_err(|e| e.to_string())?;
    fs::write(&path, data).map_err(|e| format!("config.json: {}", e))
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
    let mut clean_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    if !clean_args.is_empty() && (clean_args[0] == "ipc" || clean_args[0] == "--ipc") {
        clean_args.remove(0);
    }

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
                    state.target_end_timestamp =
                        Some(Utc::now().timestamp() + state.remaining_seconds as i64);
                }
                "pause" => {
                    if state.state == "running" {
                        if let Some(target) = state.target_end_timestamp {
                            let diff = target - Utc::now().timestamp();
                            state.remaining_seconds = diff.max(0) as u32;
                        }
                    }
                    state.state = "paused".to_string();
                    state.target_end_timestamp = None;
                }
                "toggle" => {
                    if state.state == "running" {
                        if let Some(target) = state.target_end_timestamp {
                            let diff = target - Utc::now().timestamp();
                            state.remaining_seconds = diff.max(0) as u32;
                        }
                        state.state = "paused".to_string();
                        state.target_end_timestamp = None;
                    } else {
                        state.state = "running".to_string();
                        state.target_end_timestamp =
                            Some(Utc::now().timestamp() + state.remaining_seconds as i64);
                    }
                }
                "skip" => {
                    state.target_end_timestamp = None;
                    if state.mode == "work" {
                        state.session_pomodoros += 1;
                        record_headless_pomodoro(state.active_task_id.as_deref());
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
                    state.target_end_timestamp = None;
                    state.remaining_seconds = state.total_seconds;
                    state.state = "stopped".to_string();
                }
                "mode" | "set-mode" => {
                    state.target_end_timestamp = None;
                    if clean_args.len() < 3 {
                        return Err("Missing mode: work, short_break, long_break".to_string());
                    }
                    let target_mode = clean_args[2];
                    match target_mode {
                        "work" => {
                            state.mode = "work".to_string();
                            state.total_seconds = focus_dur;
                        }
                        "short_break" | "short" => {
                            state.mode = "short_break".to_string();
                            state.total_seconds = short_dur;
                        }
                        "long_break" | "long" => {
                            state.mode = "long_break".to_string();
                            state.total_seconds = long_dur;
                        }
                        other => {
                            return Err(format!(
                                "Unknown mode: '{}'. Expected work, short_break, long_break",
                                other
                            ));
                        }
                    }
                    state.remaining_seconds = state.total_seconds;
                    state.state = "stopped".to_string();
                }
                other => {
                    return Err(format!(
                        "Unknown timer command: '{}'. Expected start, pause, toggle, skip, reset, mode",
                        other
                    ));
                }
            }

            save_runtime_state(&state).map_err(|e| e.to_string())?;
            serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
        }
        "lists" => {
            let lists = load_task_lists_cache();
            serde_json::to_string_pretty(&lists).map_err(|e| e.to_string())
        }
        "tasks" => {
            if clean_args.len() > 1 && clean_args[1] == "lists" {
                let lists = load_task_lists_cache();
                return serde_json::to_string_pretty(&lists).map_err(|e| e.to_string());
            }

            if clean_args.len() > 1 && clean_args[1] != "list" {
                return Err(format!(
                    "Unknown tasks command: '{}'. Expected 'list' or 'lists'",
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
                                if found_list_id.is_none() {
                                    if !t.list_id.is_empty() && t.list_id != "@all" {
                                        found_list_id = Some(t.list_id.clone());
                                    } else if lid != "@all" {
                                        found_list_id = Some(lid.clone());
                                    }
                                }
                            }
                        }
                    }

                    save_tasks_cache(&cache).map_err(|e| e.to_string())?;
                    record_headless_task_done();

                    // Si la tarea completada era la activa, quitarla de runtime_state
                    let mut state = load_runtime_state();
                    if state.active_task_id.as_deref() == Some(task_id) {
                        state.active_task_id = None;
                        state.active_task_title = None;
                        let _ = save_runtime_state(&state);
                    }

                    // Sincronizar con Google Tasks si hay sesión. La caché local ya quedó actualizada;
                    // si Google falla, el cambio queda en el buzón de salida y devolvemos error
                    // (exit 1) para que el plugin lo muestre.
                    let lid = found_list_id.unwrap_or_else(|| "@default".to_string());
                    let pending = PendingOp::Complete {
                        task_id: task_id.to_string(),
                        list_id: lid.clone(),
                        completed_at: Utc::now(),
                    };
                    if task_id.starts_with(outbox::LOCAL_ID_PREFIX) {
                        // Tarea aún no subida: se completará cuando su `Create` llegue a Google.
                        outbox::enqueue(pending).map_err(|e| e.to_string())?;
                        return Ok(format!(
                            "Task {} marked as completed locally (queued: {} pending change(s))",
                            task_id,
                            outbox::pending_count()
                        ));
                    }
                    if !get_config_dir().join("pomotask_token.json").exists() {
                        // Sin sesión de Google el modo local es válido: encolamos y no es un error.
                        outbox::enqueue(pending).map_err(|e| e.to_string())?;
                        mark_google_error("no_token: not signed in to Google. Open the PomoTask TUI (pomotask-cli) to sign in");
                        return Ok(format!(
                            "Task {} marked as completed locally (queued: {} pending change(s); not signed in to Google)",
                            task_id,
                            outbox::pending_count()
                        ));
                    }

                    let (client, mut rx) = ipc_api_client().await;
                    let res = with_auth_guard(
                        &mut rx,
                        10,
                        "Google Tasks completion",
                        client.toggle_task_completion(&lid, task_id, true),
                    )
                    .await;
                    match res {
                        Ok(()) => mark_google_connected(false),
                        Err(e) => {
                            outbox::enqueue(pending).map_err(|e| e.to_string())?;
                            mark_google_error(&e);
                            return Err(format!(
                                "Task {} completed locally and queued for upload ({} pending). {}",
                                task_id,
                                outbox::pending_count(),
                                e
                            ));
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
                    if let Some(all_list) = cache.get_mut("@all") {
                        all_list.push(new_task.clone());
                    }
                    save_tasks_cache(&cache).map_err(|e| e.to_string())?;

                    // Crear la tarea en Google Tasks si hay sesión. Si falla, la tarea queda en la
                    // caché local y en el buzón de salida, y devolvemos error (exit 1) para que el
                    // plugin lo muestre. Si el padre es una tarea local aún no subida, también va al buzón.
                    let pending = PendingOp::Create {
                        temp_id: temp_id.clone(),
                        list_id: target_list_id.clone(),
                        title: title_val.clone(),
                        parent_id: parent.clone(),
                        created_at: new_task.updated,
                    };
                    let parent_is_local = parent
                        .as_deref()
                        .map(|p| p.starts_with(outbox::LOCAL_ID_PREFIX))
                        .unwrap_or(false);

                    if parent_is_local {
                        outbox::enqueue(pending).map_err(|e| e.to_string())?;
                        return serde_json::to_string_pretty(&new_task).map_err(|e| e.to_string());
                    }
                    if !get_config_dir().join("pomotask_token.json").exists() {
                        // Sin sesión de Google el modo local es válido: encolamos y devolvemos la tarea.
                        outbox::enqueue(pending).map_err(|e| e.to_string())?;
                        mark_google_error("no_token: not signed in to Google. Open the PomoTask TUI (pomotask-cli) to sign in");
                        return serde_json::to_string_pretty(&new_task).map_err(|e| e.to_string());
                    }

                    let (client, mut rx) = ipc_api_client().await;
                    let res = with_auth_guard(
                        &mut rx,
                        10,
                        "Google Tasks create",
                        client.create_task_returning_id(
                            &target_list_id,
                            &title_val,
                            None,
                            None,
                            parent,
                        ),
                    )
                    .await;
                    match res {
                        Ok(new_id) => {
                            mark_google_connected(false);
                            let mut created = new_task;
                            if !new_id.is_empty() {
                                outbox::replace_temp_id(&temp_id, &new_id);
                                created.id = new_id;
                            }
                            serde_json::to_string_pretty(&created).map_err(|e| e.to_string())
                        }
                        Err(e) => {
                            outbox::enqueue(pending).map_err(|e| e.to_string())?;
                            mark_google_error(&e);
                            Err(format!(
                                "Task '{}' saved locally and queued for upload ({} pending). {}",
                                title_val,
                                outbox::pending_count(),
                                e
                            ))
                        }
                    }
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
        // Duraciones del pomodoro (config.json). Los valores se reciben en MINUTOS y se guardan en
        // segundos, como los usa la TUI. `get` devuelve las tres duraciones en segundos.
        "config" => {
            let action = clean_args.get(1).copied().unwrap_or("get");
            match action {
                "get" => {
                    let (focus, short, long) = load_config_durations();
                    let out = serde_json::json!({
                        "focus_duration": focus,
                        "short_break_duration": short,
                        "long_break_duration": long,
                    });
                    serde_json::to_string_pretty(&out).map_err(|e| e.to_string())
                }
                "set" => {
                    if clean_args.len() < 4 {
                        return Err("Usage: config set <focus|short|long> <minutes>".to_string());
                    }
                    let key = match clean_args[2] {
                        "focus" | "focus_duration" | "work" => "focus_duration",
                        "short" | "short_break" | "short_break_duration" => "short_break_duration",
                        "long" | "long_break" | "long_break_duration" => "long_break_duration",
                        other => {
                            return Err(format!(
                                "Unknown duration key: '{}'. Expected focus, short, long",
                                other
                            ))
                        }
                    };
                    let minutes: u64 = clean_args[3]
                        .parse()
                        .map_err(|_| format!("Invalid minutes: '{}'", clean_args[3]))?;
                    if !(1..=180).contains(&minutes) {
                        return Err("Minutes must be between 1 and 180".to_string());
                    }
                    let seconds = (minutes * 60) as u32;
                    set_config_duration(key, seconds)?;

                    // Si el temporizador está detenido en ese modo, reflejar la nueva duración ya.
                    let mut state = load_runtime_state();
                    let affects_mode = matches!(
                        (key, state.mode.as_str()),
                        ("focus_duration", "work")
                            | ("short_break_duration", "short_break")
                            | ("long_break_duration", "long_break")
                    );
                    if affects_mode && state.state == "stopped" {
                        state.total_seconds = seconds;
                        state.remaining_seconds = seconds;
                        save_runtime_state(&state).map_err(|e| e.to_string())?;
                    }

                    let (focus, short, long) = load_config_durations();
                    let out = serde_json::json!({
                        "focus_duration": focus,
                        "short_break_duration": short,
                        "long_break_duration": long,
                    });
                    serde_json::to_string_pretty(&out).map_err(|e| e.to_string())
                }
                other => Err(format!(
                    "Unknown config command: '{}'. Expected get, set",
                    other
                )),
            }
        }
        "blocklist" => {
            if clean_args.len() < 2 {
                return Err(
                    "Missing blocklist action: get, toggle-strict, toggle-anti-distraction, set-action, set-dimming, add-title, add-class, remove-title, remove-class".to_string(),
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
                        return Err("Missing action parameter (warn, hud, minimize)".to_string());
                    }
                    let action = normalize_distraction_action(clean_args[2]).ok_or_else(|| {
                        format!(
                            "Unknown distraction action '{}'. Expected warn, hud or minimize",
                            clean_args[2]
                        )
                    })?;
                    let mut config = load_blocklist();
                    config.action = action.to_string();
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "set-dimming" => {
                    if clean_args.len() < 3 {
                        return Err("Missing dimming value (0.0 to 1.0)".to_string());
                    }
                    let val: f64 = clean_args[2]
                        .parse()
                        .map_err(|_| "Invalid dimming value, expected number between 0.0 and 1.0".to_string())?;
                    let clamped = val.clamp(0.0, 1.0);
                    let mut config = load_blocklist();
                    config.overlay_dimming = clamped;
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
                "add-allowed-title" => {
                    if clean_args.len() < 3 {
                        return Err("Missing allowed keyword to add".to_string());
                    }
                    let mut config = load_blocklist();
                    let kw = clean_args[2].to_string();
                    if !config.allowed_title_keywords.contains(&kw) {
                        config.allowed_title_keywords.push(kw);
                    }
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "remove-allowed-title" => {
                    if clean_args.len() < 3 {
                        return Err("Missing allowed keyword to remove".to_string());
                    }
                    let mut config = load_blocklist();
                    let kw = clean_args[2];
                    config.allowed_title_keywords.retain(|k| k != kw);
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "add-allowed-class" => {
                    if clean_args.len() < 3 {
                        return Err("Missing allowed class name to add".to_string());
                    }
                    let mut config = load_blocklist();
                    let cls = clean_args[2].to_string();
                    if !config.allowed_classes.contains(&cls) {
                        config.allowed_classes.push(cls);
                    }
                    save_blocklist(&config).map_err(|e| e.to_string())?;
                    serde_json::to_string_pretty(&config).map_err(|e| e.to_string())
                }
                "remove-allowed-class" => {
                    if clean_args.len() < 3 {
                        return Err("Missing allowed class name to remove".to_string());
                    }
                    let mut config = load_blocklist();
                    let cls = clean_args[2];
                    config.allowed_classes.retain(|c| c != cls);
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
        "auth-status" => {
            // Comprueba (sin abrir el navegador) si la sesión de Google sigue siendo válida y
            // actualiza runtime_state.json. Devuelve el estado completo, igual que `status`.
            if !get_config_dir().join("pomotask_token.json").exists() {
                mark_google_error("no_token: not signed in to Google. Open the PomoTask TUI (pomotask-cli) to sign in");
            } else {
                let (client, mut rx) = ipc_api_client().await;
                match with_auth_guard(&mut rx, 8, "Google auth check", client.check_auth()).await {
                    Ok(()) => mark_google_connected(false),
                    Err(e) => mark_google_error(&e),
                }
            }
            let state = load_runtime_state();
            serde_json::to_string_pretty(&state).map_err(|e| e.to_string())
        }
        "sync" => {
            if !get_config_dir().join("pomotask_token.json").exists() {
                mark_google_error("no_token: not signed in to Google. Open the PomoTask TUI (pomotask-cli) to sign in");
                return Ok("Sync skipped: No active Google session token found".to_string());
            }
            let (client, mut rx) = ipc_api_client().await;

            // 1) Subir primero lo que quedó pendiente sin conexión. Si la sesión expiró abortamos;
            //    cualquier otro fallo se anota y seguimos, re-aplicando lo pendiente sobre la caché.
            let mut push_error: Option<String> = None;
            if outbox::pending_count() > 0 {
                match with_auth_guard(
                    &mut rx,
                    30,
                    "Uploading pending changes",
                    outbox::push_pending(&client),
                )
                .await
                {
                    Ok(_) => {}
                    Err(e) if e.starts_with("auth_required") => {
                        mark_google_error(&e);
                        return Err(e);
                    }
                    Err(e) => push_error = Some(e),
                }
            }

            // 2) Descargar listas y tareas.
            let lists = match with_auth_guard(
                &mut rx,
                10,
                "Sync fetching lists",
                client.fetch_task_lists(),
            )
            .await
            {
                Ok(l) => l,
                Err(e) => {
                    mark_google_error(&e);
                    return Err(e);
                }
            };

            let mut all_lists = vec![TaskList {
                id: "@all".to_string(),
                title: "Todas las listas".to_string(),
            }];
            all_lists.extend(lists.clone());
            let _ = save_task_lists_cache(&all_lists);

            // Partimos de la caché previa: si una lista falla conservamos sus tareas anteriores
            // en lugar de vaciarla.
            let previous = load_tasks_cache();
            let mut cache: HashMap<String, Vec<Task>> = HashMap::new();
            let mut all_tasks = Vec::new();
            let mut failed_lists: Vec<String> = Vec::new();

            for list in lists {
                if list.id == "@all" {
                    continue;
                }
                let tasks = match with_auth_guard(
                    &mut rx,
                    20,
                    &format!("Sync fetching list '{}'", list.title),
                    client.fetch_tasks(&list.id, true),
                )
                .await
                {
                    Ok(t) => t,
                    Err(e) => {
                        failed_lists.push(e);
                        previous.get(&list.id).cloned().unwrap_or_default()
                    }
                };
                all_tasks.extend(tasks.clone());
                cache.insert(list.id, tasks);
            }
            if !all_tasks.is_empty() {
                cache.insert("@all".to_string(), all_tasks);
            }

            // 3) Lo que siga pendiente (no se pudo subir) se re-aplica para no perderlo al pisar la caché.
            let still_pending = outbox::load_outbox();
            if !still_pending.is_empty() {
                outbox::apply_to_cache(&still_pending, &mut cache);
                if let Some(all) = cache.get_mut("@all") {
                    // `@all` es la unión: garantizamos que las locales también estén ahí.
                    outbox::apply_to_list(&still_pending, "@all", all);
                }
            }

            save_tasks_cache(&cache).map_err(|e| e.to_string())?;

            if let Some(first_error) = failed_lists.first() {
                mark_google_error(first_error);
                return Err(format!(
                    "Sync incomplete: {} list(s) failed. {}",
                    failed_lists.len(),
                    first_error
                ));
            }
            mark_google_connected(true);
            match push_error {
                Some(e) => Ok(format!(
                    "Sync completed, but {} pending change(s) could not be uploaded: {}",
                    still_pending.len(),
                    e
                )),
                None => Ok("Sync completed successfully".to_string()),
            }
        }
        "outbox" => {
            // Cambios hechos sin conexión que aún no se han subido a Google.
            let ops = outbox::load_outbox();
            serde_json::to_string_pretty(&ops).map_err(|e| e.to_string())
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
            target_end_timestamp: Some(1700000000),
            google_connected: None,
            last_sync_at: None,
            last_sync_error: None,
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
        assert_eq!(decoded.target_end_timestamp, Some(1700000000));
    }

    #[test]
    fn test_runtime_state_legacy_json_without_connection_fields() {
        // runtime_state.json escrito por versiones anteriores no trae los campos de conexión.
        let legacy = r#"{"state":"stopped","mode":"work","remaining_seconds":1500,"total_seconds":1500,
            "session_pomodoros":0,"active_task_id":null,"active_task_title":null,
            "strict_break":false,"anti_distraction":true}"#;
        let decoded: RuntimeState = serde_json::from_str(legacy).expect("legacy deserialize");
        assert_eq!(decoded.google_connected, None);
        assert_eq!(decoded.last_sync_at, None);
        assert_eq!(decoded.last_sync_error, None);
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
        assert_eq!(default_state.target_end_timestamp, None);
    }

    #[test]
    fn test_blocklist_matching() {
        let mut blocklist = BlocklistConfig::default();
        blocklist.title_keywords.push("youtube".to_string());
        blocklist.title_keywords.push("facebook".to_string());
        blocklist.title_keywords.push("   ".to_string());
        blocklist.blocked_classes.push("steam".to_string());
        blocklist.blocked_classes.push("".to_string());

        // YouTube estándar está bloqueado
        assert!(
            blocklist.is_distraction("Rick Astley - Never Gonna Give You Up - YouTube", "firefox")
        );
        assert!(blocklist.is_distraction("Facebook - Log In", "firefox"));
        assert!(blocklist.is_distraction("Any Title", "steam"));

        // YouTube Music está explícitamente en la lista blanca de excepciones
        assert!(!blocklist.is_distraction("Coldplay - Yellow - YouTube Music", "google-chrome"));
        assert!(!blocklist.is_distraction("music.youtube.com", "zen"));
        assert!(!blocklist.is_distraction("Any Song", "youtube-music"));

        assert!(blocklist.is_distraction("Twitter / X", "google-chrome"));
        assert!(!blocklist.is_distraction("GitHub - Pull Requests", "zen"));
        assert!(!blocklist.is_distraction("Rust Docs", "firefox"));
        assert!(!blocklist.is_distraction("", ""));
    }

    #[test]
    fn test_normalize_distraction_action() {
        assert_eq!(normalize_distraction_action("warn"), Some("warn"));
        assert_eq!(normalize_distraction_action(" HUD "), Some("hud"));
        assert_eq!(normalize_distraction_action("minimize"), Some("minimize"));
        // Nombres antiguos guardados en blocklist.json de versiones previas
        assert_eq!(
            normalize_distraction_action("warn_and_unfocus"),
            Some("hud")
        );
        assert_eq!(normalize_distraction_action("unfocus"), Some("hud"));
        assert_eq!(normalize_distraction_action("explode"), None);
        assert_eq!(normalize_distraction_action(""), None);
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
            target_end_timestamp: None,
            google_connected: None,
            last_sync_at: None,
            last_sync_error: None,
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
        assert_eq!(loaded.target_end_timestamp, None);

        let _ = std::fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn test_save_and_load_blocklist() {
        let temp_dir =
            std::env::temp_dir().join(format!("pomotask_test_bl_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let bl_path = temp_dir.join("blocklist.json");

        let mut config = BlocklistConfig::default();
        config.action = "hud".to_string();
        config.blocked_classes.push("vlc".to_string());

        save_blocklist_to(&config, &bl_path).unwrap();
        let loaded = load_blocklist_from(&bl_path);

        assert_eq!(loaded.action, "hud");
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

    #[test]
    fn test_save_and_load_task_lists_cache() {
        let temp_dir =
            std::env::temp_dir().join(format!("pomotask_test_lists_{}", rand::random::<u64>()));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let lists_path = temp_dir.join("lists_cache.json");

        let sample_lists = vec![
            TaskList {
                id: "@all".to_string(),
                title: "Todas las listas".to_string(),
            },
            TaskList {
                id: "list_1".to_string(),
                title: "Trabajo".to_string(),
            },
            TaskList {
                id: "list_2".to_string(),
                title: "Personal".to_string(),
            },
        ];

        save_task_lists_cache_to(&sample_lists, &lists_path).unwrap();
        let loaded = load_task_lists_cache_from(&lists_path);

        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[1].title, "Trabajo");
        assert_eq!(loaded[2].title, "Personal");

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
