use ratatui::style::Color;
use chrono::{DateTime, Utc, Local, Datelike};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::fs;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum TimerMode { Focus, ShortBreak, LongBreak }

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Theme { CatppuccinMocha, Nord, Gruvbox, Dracula }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config { 
    pub focus_duration: u32, 
    pub short_break_duration: u32, 
    pub long_break_duration: u32, 
    pub language: Language,
    pub theme: Theme,
    pub last_list_id: Option<String>,
    pub last_task_id: Option<String>,
    pub show_completed: bool, 
}

impl Default for Config { 
    fn default() -> Self { 
        Self { 
            focus_duration: 25 * 60, 
            short_break_duration: 5 * 60, 
            long_break_duration: 15 * 60, 
            language: Language::Spanish,
            theme: Theme::CatppuccinMocha,
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
pub enum AppMode { Loading, Timer, Auth, AuthSuccess, ListSelector, Input, SubtaskInput, Edit, ConfirmComplete, Help, Settings, ConfirmLogout }

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InputField { Title, Notes, Due, List }

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DatePreset { Today, Tomorrow, Custom, None }

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Stats { 
    pub hourly_pomodoros: BTreeMap<String, u64>,
    pub hourly_seconds: BTreeMap<String, u64>,
    pub hourly_tasks_done: BTreeMap<String, u64>,
    pub task_pomodoros: BTreeMap<String, u64>,
    pub task_timers: BTreeMap<String, TaskTimerState>,
}

#[derive(Debug, Clone)]
pub struct Particle { pub x: f64, pub y: f64, pub vx: f64, pub vy: f64, pub life: f32, pub char: char }

#[derive(Default)]
pub struct AnimationState {
    pub task_id: Option<String>,
    pub progress: f32, // 0.0 to 1.0
    pub particles: Vec<Particle>,
    pub spawn_x: u16,
    pub spawn_y: u16,
    pub spawn_w: u16,
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
    pub selected_settings_idx: usize,
    pub selected_date_preset: DatePreset,
    pub loading: bool,
    pub spinner_frame: usize,
    pub session_pomodoros: u32,
    pub tick_count: u32,
    pub config: Config,
    pub stats: Stats,
    pub animation: AnimationState,
    pub focus_subtask_idx: usize,
    pub input_list_idx: usize,
    pub confirming_task_id: Option<String>,
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
            selected_settings_idx: 0,
            selected_date_preset: DatePreset::Custom,
            loading: true,
            spinner_frame: 0,
            session_pomodoros: 0,
            tick_count: 0,
            config,
            stats,
            animation: AnimationState::default(),
            focus_subtask_idx: 0,
            input_list_idx: 0,
            confirming_task_id: None,
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

    pub fn translate(&self, key: &str) -> String {
        let res = match self.config.language {
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
                "running" => "Corriendo",
                "paused" => "Pausado",
                "current_task" => "Tarea actual:",
                "no_task" => "Ninguna seleccionada",
                "focus" => "Enfoque",
                "break" => "Un relax de: ",
                "long_break" => "Descanso largo de: ",
                "start_pause" => "Inic/Pausa",
                "reset" => "Reiniciar",
                "switch_mode" => "Cambiar modo",
                "lang" => "Idioma",
                "quit" => "Salir",
                "welcome" => "¡Bienvenido a PomoTask!",
                "login_msg" => "Inicia sesión para sincronizar con Google Tasks.",
                "login_btn" => "Presiona Enter para conectar",
                "auth_instructions" => "Abre el link en tu navegador:",
                "auth_waiting" => "Esperando autorización...",
                "auth_success_title" => "¡Conexión exitosa!",
                "auth_success_msg" => "Google Tasks se ha vinculado correctamente.",
                "help_title" => " Ayuda - Atajos de teclado ",
                "help_hint" => " Presiona cualquier tecla para volver ",
                "footer_hint" => " [? Ayuda | , Conf] ",
                "sync_manual" => "Sincronizar (Manual)",
                "help_label" => "Ayuda",
                "timer_short" => "T:",
                "sync_short" => "S:",
                "pomodoro_short" => "P:",
                "settings_title" => " Configuración ",
                "settings_focus" => "Duración enfoque (min)",
                "settings_short" => "Descanso corto (min)",
                "settings_long" => "Descanso largo (min)",
                "settings_lang" => "Idioma",
                "settings_theme" => "Tema",
                "settings_logout" => "Cerrar sesión (Google)",
                "settings_hint" => " ↑↓: Navegar | ←→: Ajustar | Esc: Volver ",
                "logout_confirm" => "Sesión cerrada. Reinicia para reconectar.",
                "change_list" => "Listas (Tab)",
                "h_l_change_list" => "Cambiar lista (←/→)",
                "j_k_navigate_timer" => "Navegar focus (j/k)",
                "new_task" => "Nueva",
                "new_subtask" => "Subtarea",
                "edit_task" => "Editar",
                "toggle_completed" => "Hechas",
                "complete_task_hotkey" => "Hecho/Pendiente",
                "confirm_title" => " Confirmar acción ",
                "confirm_msg_done" => "¿Marcar como completada la tarea: {}?",
                "confirm_msg_undone" => "¿Marcar como pendiente la tarea: {}?",
                "confirm_hint" => " Enter: Sí | Esc: No ",
                "logout_confirm_msg" => "¿Estás seguro de que deseas cerrar sesión?",
                "logout_confirm_title" => " Cerrar sesión ",
                "lists_title" => " Tus listas ",
                "list_all" => " Todas ",
                "list_selection" => " Lista de Destino ",
                "input_title" => " Nueva Tarea ",
                "subtask_title" => " Nueva Subtarea ",
                "edit_title" => " Editar Tarea ",
                "input_hint" => " TAB: Sig. Campo | ESC: Cancelar | ENTER: Guardar ",
                "due_date" => "Vencimiento",
                "due_date_hint" => " (formato: YYYY-MM-DD) ",
                "date_today" => "Hoy",
                "date_tomorrow" => "Mañana",
                "date_custom" => "Personalizado",
                "date_none" => "Sin fecha",
                "created_date" => "Creada",
                "notes" => "Notas",
                "no_notes" => "Sin notas",
                "pomodoro_label" => "Pomodoros",
                "focus_completed_today" => "completados hoy en esta tarea",
                "notify_focus_end" => "¡Tiempo de enfoque terminado! Toma un descanso.",
                "notify_break_end" => "¡El descanso terminó! A trabajar.",
                "focus_msg_0" => "En la zona, ignorando al mundo por: ",
                "focus_msg_1" => "Shhh... estás concentrado en: ",
                "focus_msg_2" => "Haciendo magia con: ",
                "focus_msg_3" => "Nadie te molesta mientras trabajas en: ",
                "focus_msg_4" => "Modo bestia activado para: ",
                "focus_msg_5" => "Eres una máquina de productividad en: ",
                "focus_msg_6" => "Picando código (o lo que sea) en: ",
                "focus_msg_7" => "El tiempo vuela cuando te enfocas en: ",
                "focus_msg_8" => "Un tomate a la vez... ahora toca: ",
                "focus_msg_9" => "Tu yo del futuro te agradecerá terminar: ",
                "month_1" => "Enero", "month_2" => "Febrero", "month_3" => "Marzo", "month_4" => "Abril",
                "month_5" => "Mayo", "month_6" => "Junio", "month_7" => "Julio", "month_8" => "Agosto",
                "month_9" => "Septiembre", "month_10" => "Octubre", "month_11" => "Noviembre", "month_12" => "Diciembre",
                "day_0" => "Domingo", "day_1" => "Lunes", "day_2" => "Martes", "day_3" => "Miércoles",
                "day_4" => "Jueves", "day_5" => "Viernes", "day_6" => "Sábado",
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
                "running" => "Running",
                "paused" => "Paused",
                "current_task" => "Current task:",
                "no_task" => "None selected",
                "focus" => "Focus",
                "break" => "Break",
                "long_break" => "Long break",
                "start_pause" => "Start/Pause",
                "reset" => "Reset",
                "switch_mode" => "Switch mode",
                "lang" => "Language",
                "quit" => "Quit",
                "welcome" => "Welcome to PomoTask!",
                "login_msg" => "Log in to sync with Google Tasks.",
                "login_btn" => "Press Enter to connect",
                "auth_instructions" => "Open the link in your browser:",
                "auth_waiting" => "Waiting for authorization...",
                "auth_success_title" => "Connection successful!",
                "auth_success_msg" => "Google Tasks has been linked correctly.",
                "help_title" => " Help - Keyboard shortcuts ",
                "help_hint" => " Press any key to return ",
                "footer_hint" => " [? Help | , Config] ",
                "sync_manual" => "Sync (Manual)",
                "help_label" => "Help",
                "timer_short" => "T:",
                "sync_short" => "S:",
                "pomodoro_short" => "P:",
                "settings_title" => " Configuration ",
                "settings_focus" => "Focus duration (min)",
                "settings_short" => "Short break (min)",
                "settings_long" => "Long break (min)",
                "settings_lang" => "Language",
                "settings_theme" => "Theme",
                "settings_logout" => "Logout (Google)",
                "settings_hint" => " ↑↓: Navigate | ←→: Adjust | Esc: Back ",
                "logout_confirm" => "Logged out. Restart to reconnect.",
                "change_list" => "Lists (Tab)",
                "h_l_change_list" => "Switch list (←/→)",
                "j_k_navigate_timer" => "Timer nav (j/k)",
                "new_task" => "New",
                "new_subtask" => "Subtask",
                "edit_task" => "Edit",
                "toggle_completed" => "Done",
                "complete_task_hotkey" => "Done/Pending",
                "confirm_title" => " Confirm action ",
                "confirm_msg_done" => "Mark task as completed: {}?",
                "confirm_msg_undone" => "Mark task as pending: {}?",
                "confirm_hint" => " Enter: Yes | Esc: No ",
                "logout_confirm_msg" => "Are you sure you want to logout?",
                "logout_confirm_title" => " Logout ",
                "lists_title" => " Your lists ",
                "list_all" => " All ",
                "list_selection" => " Target List ",
                "input_title" => " New Task ",
                "subtask_title" => " New Subtask ",
                "edit_title" => " Edit Task ",
                "input_hint" => " TAB: Next Field | ESC: Cancel | ENTER: Save ",
                "due_date" => "Due Date",
                "due_date_hint" => " (format: YYYY-MM-DD) ",
                "date_today" => "Today",
                "date_tomorrow" => "Tomorrow",
                "date_custom" => "Custom",
                "date_none" => "No date",
                "created_date" => "Created",
                "notes" => "Notes",
                "no_notes" => "No notes",
                "pomodoro_label" => "Pomodoros",
                "focus_completed_today" => "completed today in this task",
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
                "month_1" => "January", "month_2" => "February", "month_3" => "March", "month_4" => "April",
                "month_5" => "May", "month_6" => "June", "month_7" => "July", "month_8" => "August",
                "month_9" => "September", "month_10" => "October", "month_11" => "November", "month_12" => "December",
                "day_0" => "Sunday", "day_1" => "Monday", "day_2" => "Tuesday", "day_3" => "Wednesday",
                "day_4" => "Thursday", "day_5" => "Friday", "day_6" => "Saturday",
                _ => key,
            },
        };
        res.to_string()
    }

    pub fn toggle_language(&mut self) {
        self.config.language = match self.config.language { Language::Spanish => Language::English, Language::English => Language::Spanish };
        self.save_config();
    }

    pub fn logout(&mut self) {
        let mut path = Self::get_config_dir();
        path.push("pomotask_token.json");
        if path.exists() { let _ = fs::remove_file(path); }
    }

    pub fn format_full_date(&self, dt: DateTime<Utc>) -> String {
        let local_dt = dt.with_timezone(&Local);
        let day = local_dt.day();
        let month = local_dt.month();
        let weekday = local_dt.weekday().num_days_from_sunday() as usize;

        let month_key = format!("month_{}", month);
        let day_key = format!("day_{}", weekday);
        
        let month_name = self.translate(&month_key);
        let day_name = self.translate(&day_key);

        match self.config.language {
            Language::Spanish => format!("{}, {} de {}", day_name, day, month_name),
            Language::English => format!("{}, {} {}", day_name, month_name, day),
        }
    }

    pub fn format_date(&self, dt: DateTime<Utc>) -> String {
        let local_dt = dt.with_timezone(&Local);
        let day = local_dt.day();
        let month = local_dt.month();
        let month_key = match month {
            1 => "month_1", 2 => "month_2", 3 => "month_3", 4 => "month_4",
            5 => "month_5", 6 => "month_6", 7 => "month_7", 8 => "month_8",
            9 => "month_9", 10 => "month_10", 11 => "month_11", 12 => "month_12",
            _ => "month_1",
        };
        let month_name = self.translate(month_key);
        
        match self.config.language {
            Language::Spanish => format!("{} {}", day, month_name),
            Language::English => format!("{} {}", month_name, day),
        }
    }

    pub fn format_due_date(&self, dt: DateTime<Utc>) -> String {
        // Para fechas de vencimiento de Google Tasks, ignoramos la zona horaria local
        // ya que la API siempre devuelve 00:00:00Z y representa un día absoluto.
        let day = dt.day();
        let month = dt.month();
        let month_key = match month {
            1 => "month_1", 2 => "month_2", 3 => "month_3", 4 => "month_4",
            5 => "month_5", 6 => "month_6", 7 => "month_7", 8 => "month_8",
            9 => "month_9", 10 => "month_10", 11 => "month_11", 12 => "month_12",
            _ => "month_1",
        };
        let month_name = self.translate(month_key);
        
        match self.config.language {
            Language::Spanish => format!("{} {}", day, month_name),
            Language::English => format!("{} {}", month_name, day),
        }
    }

    pub fn start_completion_animation(&mut self, task_id: String, x: u16, y: u16, w: u16) {
        self.animation.task_id = Some(task_id);
        self.animation.progress = 0.0;
        self.animation.particles.clear();
        self.animation.spawn_x = x;
        self.animation.spawn_y = y;
        self.animation.spawn_w = w;
        
        let chars = ['✨', '⭐', '💥', '•', '·'];
        for _ in 0..40 { // Menos partículas, más localizadas
            let vx = (rand::random::<f64>() - 0.5) * 1.0; 
            let vy = (rand::random::<f64>() - 0.5) * 0.5; 
            let start_x_offset = rand::random::<f64>() * (w as f64);
            self.animation.particles.push(Particle {
                x: start_x_offset, y: 0.0,
                vx, vy,
                life: 0.5 + rand::random::<f32>() * 0.5,
                char: chars[rand::random::<usize>() % chars.len()],
            });
        }
    }

    pub fn tick(&mut self) {
        self.spinner_frame = self.spinner_frame.wrapping_add(1);
        
        // Update Animation
        if self.animation.task_id.is_some() {
            self.animation.progress += 0.08; // Progresión del tachado
            for p in &mut self.animation.particles {
                p.x += p.vx; p.y += p.vy;
                p.vy += 0.25; // Gravedad más fuerte para efecto "snappy"
                p.life -= 0.03; // Se desvanecen un poco más rápido
            }
            self.animation.particles.retain(|p| p.life > 0.0);
            
            if self.animation.progress >= 2.0 && self.animation.particles.is_empty() {
                self.animation.task_id = None;
                self.animation.progress = 0.0;
            }
        }

        if self.timer_active && self.timer_seconds > 0 {
            self.tick_count += 1;
            if self.tick_count >= 20 { // 20 ticks * 50ms = 1s
                self.timer_seconds -= 1; 
                self.tick_count = 0; 
                
                // Grabar tiempo de enfoque (segundos) si estamos en Focus
                if self.timer_mode == TimerMode::Focus {
                    let hour_key = Local::now().format("%Y-%m-%d %H:00").to_string();
                    let entry = self.stats.hourly_seconds.entry(hour_key).or_insert(0);
                    *entry += 1;
                }

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
        let _ = notify_rust::Notification::new().summary(title).body(&msg).icon("alarm-clock").timeout(notify_rust::Timeout::Milliseconds(5000)).show();
        self.timer_seconds = self.timer_mode.duration(&self.config);
        if let Some(task) = self.tasks.get(self.selected_task) {
            self.stats.task_timers.insert(task.id.clone(), TaskTimerState { remaining: self.timer_seconds, mode: self.timer_mode });
            self.save_stats();
        }
    }

    pub fn record_task_done(&mut self) {
        let hour_key = Local::now().format("%Y-%m-%d %H:00").to_string();
        let entry = self.stats.hourly_tasks_done.entry(hour_key).or_insert(0);
        *entry += 1;
        self.save_stats();
    }

    pub fn set_date_preset(&mut self, preset: DatePreset) {
        self.selected_date_preset = preset;
        let now = Local::now();
        match preset {
            DatePreset::Today => {
                self.input_due = now.format("%Y-%m-%d").to_string();
            }
            DatePreset::Tomorrow => {
                let tomorrow = now + chrono::Duration::days(1);
                self.input_due = tomorrow.format("%Y-%m-%d").to_string();
            }
            DatePreset::Custom => {
                // No cambiamos input_due automáticamente para dejar que el usuario escriba
            }
            DatePreset::None => {
                self.input_due = String::new();
            }
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
    pub fn mauve(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(203, 166, 247),
            Theme::Nord => Color::Rgb(180, 142, 173),
            Theme::Gruvbox => Color::Rgb(211, 134, 155),
            Theme::Dracula => Color::Rgb(189, 147, 249),
        }
    }
    pub fn red(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(243, 139, 168),
            Theme::Nord => Color::Rgb(191, 97, 106),
            Theme::Gruvbox => Color::Rgb(251, 73, 52),
            Theme::Dracula => Color::Rgb(255, 85, 85),
        }
    }
    pub fn green(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(166, 227, 161),
            Theme::Nord => Color::Rgb(163, 190, 140),
            Theme::Gruvbox => Color::Rgb(184, 187, 38),
            Theme::Dracula => Color::Rgb(80, 250, 123),
        }
    }
    pub fn peach(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(250, 179, 135),
            Theme::Nord => Color::Rgb(208, 135, 112),
            Theme::Gruvbox => Color::Rgb(254, 128, 25),
            Theme::Dracula => Color::Rgb(255, 184, 108),
        }
    }
    pub fn yellow(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(249, 226, 175),
            Theme::Nord => Color::Rgb(235, 203, 139),
            Theme::Gruvbox => Color::Rgb(250, 189, 47),
            Theme::Dracula => Color::Rgb(241, 250, 140),
        }
    }
    pub fn blue(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(137, 180, 250),
            Theme::Nord => Color::Rgb(129, 161, 193),
            Theme::Gruvbox => Color::Rgb(131, 165, 152),
            Theme::Dracula => Color::Rgb(139, 233, 253),
        }
    }
    pub fn text(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(205, 214, 244),
            Theme::Nord => Color::Rgb(236, 239, 244),
            Theme::Gruvbox => Color::Rgb(235, 219, 178),
            Theme::Dracula => Color::Rgb(248, 248, 242),
        }
    }
    pub fn subtext0(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(166, 173, 200),
            Theme::Nord => Color::Rgb(216, 222, 233),
            Theme::Gruvbox => Color::Rgb(168, 153, 132),
            Theme::Dracula => Color::Rgb(98, 114, 164),
        }
    }
    pub fn overlay0(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(108, 112, 134),
            Theme::Nord => Color::Rgb(76, 86, 106),
            Theme::Gruvbox => Color::Rgb(146, 131, 116),
            Theme::Dracula => Color::Rgb(68, 71, 90),
        }
    }
    pub fn surface0(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(49, 50, 68),
            Theme::Nord => Color::Rgb(59, 66, 82),
            Theme::Gruvbox => Color::Rgb(60, 56, 54),
            Theme::Dracula => Color::Rgb(40, 42, 54),
        }
    }
    pub fn base(theme: Theme) -> Color {
        match theme {
            Theme::CatppuccinMocha => Color::Rgb(30, 30, 46),
            Theme::Nord => Color::Rgb(46, 52, 64),
            Theme::Gruvbox => Color::Rgb(40, 40, 40),
            Theme::Dracula => Color::Rgb(40, 42, 54),
        }
    }
}

