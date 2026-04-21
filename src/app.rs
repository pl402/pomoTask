use ratatui::style::Color;
use chrono::{DateTime, Utc, Local};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum TimerMode { Focus, ShortBreak, LongBreak }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config { 
    pub focus_duration: u32, 
    pub short_break_duration: u32, 
    pub long_break_duration: u32, 
    pub language: Language,
    pub last_list_id: Option<String>,
    pub last_task_id: Option<String>,
    pub show_completed: bool, // Nuevo: Switch para tareas hechas
}

impl Default for Config { 
    fn default() -> Self { 
        Self { 
            focus_duration: 25 * 60, 
            short_break_duration: 5 * 60, 
            long_break_duration: 15 * 60, 
            language: Language::Spanish,
            last_list_id: None,
            last_task_id: None,
            show_completed: false,
        } 
    } 
}

impl TimerMode { pub fn duration(&self, config: &Config) -> u32 { match self { TimerMode::Focus => config.focus_duration, TimerMode::ShortBreak => config.short_break_duration, TimerMode::LongBreak => config.long_break_duration } } }

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct TaskTimerState { pub remaining: u32, pub mode: TimerMode }

#[derive(Debug, Clone)]
pub struct Task { 
    pub id: String, 
    pub title: String, 
    pub completed: bool, 
    pub due: Option<DateTime<Utc>>, 
    pub updated: DateTime<Utc>, // Nuevo campo
    pub notes: Option<String>, 
    pub parent_id: Option<String>,
    pub pomodoros: u64,
}

#[derive(Debug, Clone)]
pub struct TaskList { pub id: String, pub title: String }

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Language { English, Spanish }

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppMode { Loading, Timer, Auth, AuthSuccess, ListSelector, Input, SubtaskInput, Edit, ConfirmComplete }

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InputField { Title, Notes, Due }

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Stats { 
    pub hourly_pomodoros: BTreeMap<String, u64>,
    pub task_pomodoros: BTreeMap<String, u64>,
    pub task_timers: BTreeMap<String, TaskTimerState>,
}

pub struct App {
    pub running: bool,
    pub mode: AppMode,
    pub auth_url: Option<String>,
    pub input_title: String,
    pub input_notes: String,
    pub input_due: String,
    pub focused_input: InputField,
    pub editing_task_id: Option<String>,
    pub timer_active: bool,
    pub timer_seconds: u32,
    pub timer_mode: TimerMode,
    pub tasks: Vec<Task>,
    pub task_lists: Vec<TaskList>,
    pub selected_list_idx: usize,
    pub selected_task: usize,
    pub loading: bool,
    pub spinner_frame: usize,
    pub session_pomodoros: u32,
    pub tick_count: u32,
    pub config: Config,
    pub stats: Stats,
}

impl App {
    pub fn new() -> Self {
        let config = Self::load_config().unwrap_or_default();
        let stats = Self::load_stats().unwrap_or_default();
        Self {
            running: true,
            mode: AppMode::Loading,
            auth_url: None,
            input_title: String::new(),
            input_notes: String::new(),
            input_due: String::new(),
            focused_input: InputField::Title,
            editing_task_id: None,
            timer_active: false,
            timer_seconds: TimerMode::Focus.duration(&config),
            timer_mode: TimerMode::Focus,
            tasks: Vec::new(),
            task_lists: Vec::new(),
            selected_list_idx: 0,
            selected_task: 0,
            loading: true,
            spinner_frame: 0,
            session_pomodoros: 0,
            tick_count: 0,
            config,
            stats,
        }
    }

    pub fn get_config_dir() -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("pomotask");
        let _ = fs::create_dir_all(&path);
        path
    }

    fn load_config() -> Option<Config> {
        let mut path = Self::get_config_dir();
        path.push("config.json");
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save_config(&self) {
        let mut path = Self::get_config_dir();
        path.push("config.json");
        if let Ok(data) = serde_json::to_string_pretty(&self.config) { let _ = fs::write(path, data); }
    }

    fn load_stats() -> Option<Stats> {
        let mut path = Self::get_config_dir();
        path.push("stats.json");
        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn save_stats(&self) {
        let mut path = Self::get_config_dir();
        path.push("stats.json");
        if let Ok(data) = serde_json::to_string_pretty(&self.stats) { let _ = fs::write(path, data); }
    }

    pub fn get_sparkline_data(&self) -> Vec<u64> {
        let now = Local::now();
        let mut data = Vec::new();
        for i in (0..24).rev() {
            let hour_ago = now - chrono::Duration::hours(i);
            let key = hour_ago.format("%Y-%m-%d %H:00").to_string();
            data.push(*self.stats.hourly_pomodoros.get(&key).unwrap_or(&0));
        }
        data
    }

    pub fn translate<'a>(&self, key: &'a str) -> &'a str {
        match self.config.language {
            Language::Spanish => match key {
                "title" => " 🍅 PomoTask-CLI ",
                "loading_app" => "Cargando PomoTask...",
                "sync" => "Sinc",
                "session" => "Sesión",
                "timer" => " Temporizador ",
                "tasks" => " Tareas ",
                "productivity" => " Productividad (24h) ",
                "info" => " Información de Tarea ",
                "status" => "Estado: ",
                "running" => "CORRIENDO",
                "paused" => "PAUSADO",
                "current_task" => "Tarea Actual:",
                "no_task" => "Ninguna seleccionada",
                "focus" => "ENFOQUE",
                "break" => "DESCANSO",
                "long_break" => "DESCANSO LARGO",
                "start_pause" => "Inic/Pausa",
                "reset" => "Reiniciar",
                "switch_mode" => "Cambiar Modo",
                "lang" => "Idioma",
                "quit" => "Salir",
                "welcome" => "¡Bienvenido a PomoTask!",
                "login_msg" => "Inicia sesión para sincronizar con Google Tasks.",
                "login_btn" => "Presiona ENTER para conectar",
                "auth_instructions" => "Abre el link en tu navegador:",
                "auth_waiting" => "Esperando autorización...",
                "auth_success_title" => "¡CONEXIÓN EXITOSA!",
                "auth_success_msg" => "Google Tasks se ha vinculado correctamente.",
                "auth_success_hint" => "Cierra el navegador y prepárate para ser productivo.",
                "change_list" => "Listas",
                "new_task" => "Nueva",
                "new_subtask" => "Subtarea",
                "edit_task" => "Editar",
                "toggle_completed" => "Hechas",
                "complete_task_hotkey" => "Hecho/Pendiente",
                "confirm_title" => " Confirmar Acción ",
                "confirm_msg_done" => "¿Marcar esta tarea como completada?",
                "confirm_msg_undone" => "¿Marcar como pendiente?",
                "confirm_hint" => " ENTER: Sí | ESC: No ",
                "lists_title" => " Tus Listas ",
                "input_title" => " Nueva Tarea ",
                "subtask_title" => " Nueva Subtarea ",
                "edit_title" => " Editar Tarea ",
                "input_hint" => " TAB: Sig. Campo | ESC: Cancelar | ENTER: Guardar ",
                "due_date" => "Vencimiento",
                "due_date_hint" => " (formato: YYYY-MM-DD) ",
                "created_date" => "Creada",
                "notes" => "Notas",
                "no_notes" => "Sin notas",
                "pomodoro_label" => "Pomodoros",
                "notify_focus_end" => "¡Tiempo de enfoque terminado! Toma un descanso.",
                "notify_break_end" => "¡El descanso terminó! A trabajar.",
                "focus_msg_0" => "Estás en la zona, ignorando el mundo por: ",
                "focus_msg_1" => "Shhh... estás concentrado en: ",
                "focus_msg_2" => "Haciendo magia con: ",
                "focus_msg_3" => "Nadie te molesta mientras trabajas en: ",
                "focus_msg_4" => "Modo bestia activado para: ",
                "focus_msg_5" => "Eres una máquina de productividad enfocada en: ",
                "focus_msg_6" => "Picando código (o lo que sea) en: ",
                "focus_msg_7" => "El tiempo vuela cuando te enfocas en: ",
                "focus_msg_8" => "Un tomate a la vez... ahora toca: ",
                "focus_msg_9" => "Tu yo del futuro te agradecerá terminar: ",
                _ => key,
            },
            Language::English => match key {
                "title" => " 🍅 PomoTask-CLI ",
                "loading_app" => "Loading PomoTask...",
                "sync" => "Sync",
                "session" => "Session",
                "timer" => " Timer ",
                "tasks" => " Tasks ",
                "productivity" => " Productivity (24h) ",
                "info" => " Task Information ",
                "status" => "Status: ",
                "running" => "RUNNING",
                "paused" => "PAUSED",
                "current_task" => "Current Task:",
                "no_task" => "None selected",
                "focus" => "FOCUS",
                "break" => "BREAK",
                "long_break" => "LONG BREAK",
                "start_pause" => "Start/Pause",
                "reset" => "Reset",
                "switch_mode" => "Switch Mode",
                "lang" => "Language",
                "quit" => "Quit",
                "welcome" => "Welcome to PomoTask!",
                "login_msg" => "Log in to sync with Google Tasks.",
                "login_btn" => "Press ENTER to connect",
                "auth_instructions" => "Open the link in your browser:",
                "auth_waiting" => "Waiting for authorization...",
                "auth_success_title" => "CONNECTION SUCCESSFUL!",
                "auth_success_msg" => "Google Tasks has been linked correctly.",
                "auth_success_hint" => "Close your browser and get ready to be productive.",
                "change_list" => "Lists",
                "new_task" => "New",
                "new_subtask" => "Subtask",
                "edit_task" => "Edit",
                "toggle_completed" => "Done",
                "complete_task_hotkey" => "Done/Pending",
                "confirm_title" => " Confirm Action ",
                "confirm_msg_done" => "Mark this task as completed?",
                "confirm_msg_undone" => "Mark as pending?",
                "confirm_hint" => " ENTER: Yes | ESC: No ",
                "lists_title" => " Your Lists ",
                "input_title" => " New Task ",
                "subtask_title" => " New Subtask ",
                "edit_title" => " Edit Task ",
                "input_hint" => " TAB: Next Field | ESC: Cancel | ENTER: Save ",
                "due_date" => "Due Date",
                "due_date_hint" => " (format: YYYY-MM-DD) ",
                "created_date" => "Created",
                "notes" => "Notes",
                "no_notes" => "No notes",
                "pomodoro_label" => "Pomodoros",
                "notify_focus_end" => "Focus session complete! Take a break.",
                "notify_break_end" => "Break is over! Time to work.",
                "focus_msg_0" => "In the zone, ignoring the world for: ",
                "focus_msg_1" => "Shhh... focusing hard on: ",
                "focus_msg_2" => "Doing magic with: ",
                "focus_msg_3" => "Nobody disturbs you while working on: ",
                "focus_msg_4" => "Beast mode activated for: ",
                "focus_msg_5" => "Productivity machine focused on: ",
                "focus_msg_6" => "Crunching through: ",
                "focus_msg_7" => "Time flies when you focus on: ",
                "focus_msg_8" => "One tomato at a time... now: ",
                "focus_msg_9" => "Your future self will thank you for: ",
                _ => key,
            },
        }
    }

    pub fn toggle_language(&mut self) {
        self.config.language = match self.config.language { Language::Spanish => Language::English, Language::English => Language::Spanish };
        self.save_config();
    }

    pub fn tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
        if self.timer_active && self.timer_seconds > 0 {
            self.tick_count += 1;
            if self.tick_count >= 4 { 
                self.timer_seconds -= 1; 
                self.tick_count = 0; 
                if self.timer_seconds % 5 == 0 {
                    if let Some(task) = self.tasks.get(self.selected_task) {
                        self.stats.task_timers.insert(task.id.clone(), TaskTimerState { remaining: self.timer_seconds, mode: self.timer_mode });
                        self.save_stats();
                    }
                }
                if self.timer_seconds == 0 { self.on_timer_complete(); } 
            }
        }
    }

    fn on_timer_complete(&mut self) {
        self.timer_active = false;
        if let Some(task) = self.tasks.get(self.selected_task) { self.stats.task_timers.remove(&task.id); }
        let (title, msg) = if self.timer_mode == TimerMode::Focus {
            self.session_pomodoros += 1;
            let hour_key = Local::now().format("%Y-%m-%d %H:00").to_string();
            let entry = self.stats.hourly_pomodoros.entry(hour_key).or_insert(0);
            *entry += 1;
            if let Some(task) = self.tasks.get(self.selected_task) {
                let t_entry = self.stats.task_pomodoros.entry(task.id.clone()).or_insert(0);
                *t_entry += 1;
            }
            self.save_stats();
            self.timer_mode = TimerMode::ShortBreak;
            ("PomoTask", self.translate("notify_focus_end"))
        } else {
            self.timer_mode = TimerMode::Focus;
            ("PomoTask", self.translate("notify_break_end"))
        };
        let _ = notify_rust::Notification::new().summary(title).body(msg).icon("alarm-clock").timeout(notify_rust::Timeout::Milliseconds(5000)).show();
        self.timer_seconds = self.timer_mode.duration(&self.config);
        if let Some(task) = self.tasks.get(self.selected_task) {
            self.stats.task_timers.insert(task.id.clone(), TaskTimerState { remaining: self.timer_seconds, mode: self.timer_mode });
            self.save_stats();
        }
    }

    pub fn toggle_timer(&mut self) { self.timer_active = !self.timer_active; }
    pub fn reset_timer(&mut self) { 
        self.timer_active = false; 
        if let Some(task) = self.tasks.get(self.selected_task) { self.stats.task_timers.remove(&task.id); self.save_stats(); }
        self.timer_seconds = self.timer_mode.duration(&self.config); 
    }
}

pub struct Palette;
impl Palette {
    pub const ROSEWATER: Color = Color::Rgb(245, 224, 220); pub const FLAMINGO: Color = Color::Rgb(242, 205, 205); pub const PINK: Color = Color::Rgb(245, 194, 231); pub const MAUVE: Color = Color::Rgb(203, 166, 247); pub const RED: Color = Color::Rgb(243, 139, 168); pub const MAROON: Color = Color::Rgb(235, 160, 172); pub const PEACH: Color = Color::Rgb(250, 179, 135); pub const YELLOW: Color = Color::Rgb(249, 226, 175); pub const GREEN: Color = Color::Rgb(166, 227, 161); pub const TEAL: Color = Color::Rgb(148, 226, 213); pub const SKY: Color = Color::Rgb(137, 220, 235); pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236); pub const BLUE: Color = Color::Rgb(137, 180, 250); pub const LAVENDER: Color = Color::Rgb(180, 190, 254); pub const TEXT: Color = Color::Rgb(205, 214, 244); pub const SUBTEXT1: Color = Color::Rgb(186, 194, 222); pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200); pub const OVERLAY2: Color = Color::Rgb(147, 153, 178); pub const OVERLAY1: Color = Color::Rgb(127, 132, 156); pub const OVERLAY0: Color = Color::Rgb(108, 112, 134); pub const SURFACE2: Color = Color::Rgb(88, 91, 112); pub const SURFACE1: Color = Color::Rgb(69, 71, 90); pub const SURFACE0: Color = Color::Rgb(49, 50, 68); pub const BASE: Color = Color::Rgb(30, 30, 46); pub const MANTLE: Color = Color::Rgb(24, 24, 37); pub const CRUST: Color = Color::Rgb(17, 17, 27);
}
