use chrono::{DateTime, Utc, Local, Datelike, TimeZone};
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::fs;
use crate::ui::palette::{Theme, ThemeColors};

mod i18n;

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum TimerMode { Focus, ShortBreak, LongBreak }

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum CalendarView { Standard, Heatmap, Progress }

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum CalendarRange { Month, Week, Day }

/// Cuánta historia de estadísticas locales (registros horarios) se conserva al arrancar.
#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Default)]
pub enum StatsRetention { Month, #[default] Year, Forever }

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config { 
    pub focus_duration: u32, 
    pub short_break_duration: u32, 
    pub long_break_duration: u32, 
    pub language: Language,
    pub theme: Theme,
    pub custom_theme: Option<ThemeColors>,
    pub calendar_view: CalendarView,
    pub calendar_range: CalendarRange,
    pub last_list_id: Option<String>,
    pub last_task_id: Option<String>,
    pub show_completed: bool,
    #[serde(default)]
    pub stats_retention: StatsRetention,
}

impl Default for Config { 
    fn default() -> Self { 
        Self { 
            focus_duration: 25 * 60, 
            short_break_duration: 5 * 60, 
            long_break_duration: 15 * 60, 
            language: Language::Spanish,
            theme: Theme::CatppuccinMocha,
            custom_theme: None,
            calendar_view: CalendarView::Standard,
            calendar_range: CalendarRange::Month,
            last_list_id: None,
            last_task_id: None,
            show_completed: false,
            stats_retention: StatsRetention::Year,
        }
    }
}

impl TimerMode { pub fn duration(&self, config: &Config) -> u32 { match self { TimerMode::Focus => config.focus_duration, TimerMode::ShortBreak => config.short_break_duration, TimerMode::LongBreak => config.long_break_duration } } }

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct TaskTimerState { pub remaining: u32, pub mode: TimerMode }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub list_id: String, // Nuevo campo
    pub title: String, 
    pub completed: bool, 
    pub due: Option<DateTime<Utc>>, 
    pub updated: DateTime<Utc>, // Nuevo campo
    pub completed_at: Option<DateTime<Utc>>, // Nuevo campo
    pub notes: Option<String>, 
    pub parent_id: Option<String>,
    pub pomodoros: u64,
}

#[derive(Debug, Clone)]
pub struct TaskList { pub id: String, pub title: String }

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum Language { English, Spanish }

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum AppMode { Loading, Timer, Auth, AuthSuccess, ListSelector, Input, SubtaskInput, Edit, ConfirmComplete, Help, Settings, ConfirmLogout, Stats, Search }

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
    // Totales acumulados de por vida: NO se podan en la limpieza, así "Totales históricos"
    // sobrevive aunque se borren los registros horarios antiguos.
    #[serde(default)]
    pub lifetime_pomodoros: u64,
    #[serde(default)]
    pub lifetime_tasks_done: u64,
    #[serde(default)]
    pub lifetime_focus_seconds: u64,
    // Marca de que ya se sembraron los totales desde los mapas horarios (migración una sola vez).
    #[serde(default)]
    pub totals_migrated: bool,
}

#[derive(Debug, Clone)]
pub struct DayStats { pub label: String, pub pomodoros: u64, pub tasks_done: u64, pub focus_seconds: u64 }

/// Conserva solo las entradas cuya clave (fecha "YYYY-MM-DD HH:00") es >= `cutoff` ("YYYY-MM-DD").
/// La comparación léxica equivale a comparar fechas por el formato fijo ISO con ceros a la izquierda.
fn prune_before(map: &mut BTreeMap<String, u64>, cutoff: &str) {
    map.retain(|k, _| k.as_str() >= cutoff);
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
    pub all_tasks: Vec<Task>, // Nuevo campo para el calendario
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
    pub marking_done_task_id: Option<String>,
    pub creating_task_temp_id: Option<String>,
    pub calendar_date: chrono::NaiveDate,
    pub task_filter: String,
    pub moving_task_id: Option<String>,
    pub clipboard_request: Option<String>,
    pub copy_feedback_frames: u32,
    pub cleaning_frames: u32,
}

impl App {
    pub fn new() -> Self {
        let config = Self::load_config().unwrap_or_default();
        let stats = Self::load_stats().unwrap_or_default();
        let cached_tasks = Self::load_tasks_cache();
        let mut app = Self {
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
            all_tasks: cached_tasks,
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
            marking_done_task_id: None,
            creating_task_temp_id: None,
            calendar_date: Local::now().date_naive(),
            task_filter: String::new(),
            moving_task_id: None,
            clipboard_request: None,
            copy_feedback_frames: 0,
            cleaning_frames: 0,
        };
        // Mostrar las tareas cacheadas de inmediato (se reemplazan al primer sync exitoso).
        app.rebuild_visible_tasks();
        // Si hay caché local, arrancamos ya en la vista normal (con "Cargando…" en el pie) en vez de
        // la pantalla de carga a pantalla completa, que da sensación de lentitud. Sin caché (primer
        // arranque) se mantiene la pantalla de carga.
        if !app.all_tasks.is_empty() {
            app.mode = AppMode::Timer;
        }
        // Sembrar los totales históricos desde los mapas horarios ANTES de podar (una sola vez).
        app.migrate_lifetime_totals();
        // Limpieza transparente de estadísticas antiguas al arrancar (solo local, no toca Google).
        if app.cleanup_old_stats() {
            app.cleaning_frames = 30; // ~1.5 s de aviso en la barra de estado
        }
        app
    }

    /// Inicializa los contadores acumulados de por vida a partir de los mapas horarios existentes.
    /// Se ejecuta una sola vez (controlado por `totals_migrated`) para no perder el histórico previo
    /// de usuarios que ya tenían datos antes de existir estos contadores.
    fn migrate_lifetime_totals(&mut self) {
        if !self.stats.totals_migrated {
            self.stats.lifetime_pomodoros = self.stats.hourly_pomodoros.values().sum();
            self.stats.lifetime_tasks_done = self.stats.hourly_tasks_done.values().sum();
            self.stats.lifetime_focus_seconds = self.stats.hourly_seconds.values().sum();
            self.stats.totals_migrated = true;
            self.save_stats();
        }
    }

    /// Elimina los registros horarios (pomodoros, segundos de enfoque y tareas completadas) más
    /// antiguos que el periodo de retención configurado. Es puramente local: NO borra nada de
    /// Google Tasks. Devuelve `true` si se eliminó algo (para mostrar el aviso "Limpiando…").
    pub fn cleanup_old_stats(&mut self) -> bool {
        let months = match self.config.stats_retention {
            StatsRetention::Month => 1,
            StatsRetention::Year => 12,
            StatsRetention::Forever => return false,
        };
        let cutoff = match Local::now().date_naive().checked_sub_months(chrono::Months::new(months)) {
            Some(d) => d.format("%Y-%m-%d").to_string(),
            None => return false,
        };
        // Las claves tienen formato fijo "YYYY-MM-DD HH:00", así que la comparación léxica equivale
        // a comparar fechas. Conservamos las entradas con fecha >= corte.
        let count = |s: &Stats| s.hourly_pomodoros.len() + s.hourly_seconds.len() + s.hourly_tasks_done.len();
        let before = count(&self.stats);
        prune_before(&mut self.stats.hourly_pomodoros, &cutoff);
        prune_before(&mut self.stats.hourly_seconds, &cutoff);
        prune_before(&mut self.stats.hourly_tasks_done, &cutoff);
        let removed = before != count(&self.stats);
        if removed { self.save_stats(); }
        removed
    }

    /// Reconstruye `tasks` (lista visual) desde `all_tasks` aplicando el filtro de completadas,
    /// el filtro de búsqueda por texto y la organización jerárquica.
    pub fn rebuild_visible_tasks(&mut self) {
        let filter = self.task_filter.to_lowercase();
        let filtered: Vec<Task> = self.all_tasks.iter()
            .filter(|t| self.config.show_completed || !t.completed)
            .filter(|t| filter.is_empty() || t.title.to_lowercase().contains(&filter))
            .cloned()
            .collect();
        self.tasks = Self::organize_tasks_hierarchical(filtered);
        if self.selected_task >= self.tasks.len() {
            self.selected_task = self.tasks.len().saturating_sub(1);
        }
    }

    pub fn calendar_next_month(&mut self) {
        let (mut year, mut month) = (self.calendar_date.year(), self.calendar_date.month());
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
        self.calendar_date = chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    }

    pub fn calendar_prev_month(&mut self) {
        let (mut year, mut month) = (self.calendar_date.year(), self.calendar_date.month());
        if month == 1 {
            year -= 1;
            month = 12;
        } else {
            month -= 1;
        }
        self.calendar_date = chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    }

    /// `true` cuando la lista seleccionada es la vista virtual "Todas" (@all).
    pub fn is_all_view(&self) -> bool {
        self.task_lists.get(self.selected_list_idx).is_some_and(|l| l.id == "@all")
    }

    /// Título de la lista real a la que pertenece una tarea (para mostrar su origen en la vista @all).
    pub fn list_title_for(&self, list_id: &str) -> String {
        self.task_lists.iter()
            .find(|l| l.id == list_id && l.id != "@all")
            .map(|l| l.title.clone())
            .unwrap_or_else(|| "---".to_string())
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

    /// Caché en disco de las últimas tareas sincronizadas (para mostrar algo sin red al arrancar).
    fn load_tasks_cache() -> Vec<Task> {
        let mut path = Self::get_config_dir();
        path.push("tasks_cache.json");
        fs::read_to_string(path).ok()
            .and_then(|data| serde_json::from_str(&data).ok())
            .unwrap_or_default()
    }

    pub fn save_tasks_cache(&self) {
        let mut path = Self::get_config_dir();
        path.push("tasks_cache.json");
        if let Ok(data) = serde_json::to_string(&self.all_tasks) { let _ = fs::write(path, data); }
    }

    pub fn save_stats(&self) {
        let mut path = Self::get_config_dir();
        path.push("stats.json");
        if let Ok(data) = serde_json::to_string_pretty(&self.stats) { let _ = fs::write(path, data); }
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
        if self.copy_feedback_frames > 0 { self.copy_feedback_frames -= 1; }
        if self.cleaning_frames > 0 { self.cleaning_frames -= 1; }
        
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
                    self.stats.lifetime_focus_seconds += 1;
                }

                if self.timer_seconds.is_multiple_of(5) {
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
            self.stats.lifetime_pomodoros += 1;
            if let Some(task) = self.tasks.get(self.selected_task) {
                let t_entry = self.stats.task_pomodoros.entry(task.id.clone()).or_insert(0);
                *t_entry += 1;
            }
            self.save_stats();
            // Técnica Pomodoro: descanso largo cada 4 pomodoros, corto el resto.
            self.timer_mode = if self.session_pomodoros.is_multiple_of(4) { TimerMode::LongBreak } else { TimerMode::ShortBreak };
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
        self.stats.lifetime_tasks_done += 1;
        self.save_stats();
    }

    /// Agrega estadísticas por día para los últimos `n` días (de más antiguo a más reciente).
    /// Devuelve `(etiqueta "dd/mm", pomodoros, tareas_completadas, segundos_de_enfoque)`.
    pub fn stats_last_n_days(&self, n: i64) -> Vec<DayStats> {
        let today = Local::now().date_naive();
        (0..n).rev().map(|i| {
            let date = today - chrono::Duration::days(i);
            let prefix = date.format("%Y-%m-%d").to_string();
            let sum = |map: &BTreeMap<String, u64>| -> u64 {
                map.iter().filter(|(k, _)| k.starts_with(&prefix)).map(|(_, v)| *v).sum()
            };
            DayStats {
                label: date.format("%d/%m").to_string(),
                pomodoros: sum(&self.stats.hourly_pomodoros),
                tasks_done: sum(&self.stats.hourly_tasks_done),
                focus_seconds: sum(&self.stats.hourly_seconds),
            }
        }).collect()
    }

    /// Totales históricos acumulados de por vida (no se ven afectados por la limpieza).
    pub fn stats_totals(&self) -> (u64, u64, u64) {
        (self.stats.lifetime_pomodoros, self.stats.lifetime_tasks_done, self.stats.lifetime_focus_seconds)
    }

    /// Arma el texto de una tarea (con su descripción, fecha y todas sus subtareas) en formato
    /// markdown listo para pegar. Usa `all_tasks` para incluir subtareas completadas aunque la
    /// vista las oculte.
    pub fn build_task_clipboard_text(&self, task_id: &str) -> String {
        let check = |c: bool| if c { "[x]" } else { "[ ]" };
        let mut out = String::new();

        let task = match self.all_tasks.iter().find(|t| t.id == task_id) {
            Some(t) => t,
            None => return out,
        };

        out.push_str(&format!("- {} {}\n", check(task.completed), task.title));
        if let Some(due) = task.due {
            out.push_str(&format!("    📅 {}\n", self.format_due_date(due)));
        }
        if let Some(notes) = &task.notes {
            for line in notes.lines() { out.push_str(&format!("    {}\n", line)); }
        }

        let mut children: Vec<&Task> = self.all_tasks.iter()
            .filter(|t| t.parent_id.as_deref() == Some(task_id))
            .collect();
        // Pendientes primero; dentro de cada grupo, las más recientes al final.
        children.sort_by(|a, b| a.completed.cmp(&b.completed).then(a.updated.cmp(&b.updated)));

        for child in children {
            out.push_str(&format!("  - {} {}\n", check(child.completed), child.title));
            if let Some(due) = child.due {
                out.push_str(&format!("      📅 {}\n", self.format_due_date(due)));
            }
            if let Some(notes) = &child.notes {
                for line in notes.lines() { out.push_str(&format!("      {}\n", line)); }
            }
        }

        out
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

    /// Cambia manualmente entre Enfoque → Descanso corto → Descanso largo (solo con el timer detenido).
    pub fn cycle_timer_mode(&mut self) {
        if self.timer_active { return; }
        self.timer_mode = match self.timer_mode {
            TimerMode::Focus => TimerMode::ShortBreak,
            TimerMode::ShortBreak => TimerMode::LongBreak,
            TimerMode::LongBreak => TimerMode::Focus,
        };
        self.timer_seconds = self.timer_mode.duration(&self.config);
    }
    pub fn reset_timer(&mut self) { 
        self.timer_active = false; 
        if let Some(task) = self.tasks.get(self.selected_task) { self.stats.task_timers.remove(&task.id); self.save_stats(); }
        self.timer_seconds = self.timer_mode.duration(&self.config); 
    }

    pub fn clear_inputs(&mut self) { 
        self.input_title.clear(); 
        self.input_notes.clear(); 
        self.input_due.clear(); 
        self.editing_task_id = None; 
        self.input_list_idx = 0; 
        self.confirming_task_id = None; 
    }

    pub fn parse_due_date(input: &str) -> Option<DateTime<Utc>> {
        let parts: Vec<&str> = input.split('-').collect();
        if parts.len() == 3 {
            let y: i32 = parts[0].parse().ok()?;
            let m: u32 = parts[1].parse().ok()?;
            let d: u32 = parts[2].parse().ok()?;
            // Google Tasks requiere que la hora sea exactamente 00:00:00Z.
            Utc.with_ymd_and_hms(y, m, d, 0, 0, 0).single()
        } else { None }
    }

    pub fn save_selection(&mut self) {
        if let Some(list) = self.task_lists.get(self.selected_list_idx) { 
            self.config.last_list_id = Some(list.id.clone()); 
        }
        if let Some(task) = self.tasks.get(self.selected_task) { 
            self.config.last_task_id = Some(task.id.clone()); 
        }
        self.save_config();
    }

    pub fn sync_active_timer_to_task(&mut self) {
        if self.timer_active { return; }
        if let Some(task) = self.tasks.get(self.selected_task) {
            if let Some(state) = self.stats.task_timers.get(&task.id) { 
                self.timer_seconds = state.remaining; 
                self.timer_mode = state.mode; 
            } else { 
                self.timer_mode = TimerMode::Focus; 
                self.timer_seconds = self.timer_mode.duration(&self.config); 
            }
        }
    }

    pub fn organize_tasks_hierarchical(tasks: Vec<Task>) -> Vec<Task> {
        let mut organized = Vec::new();
        let sort_criteria = |a: &Task, b: &Task| {
            if a.completed != b.completed { return a.completed.cmp(&b.completed); }
            if a.due != b.due {
                match (a.due, b.due) {
                    (Some(da), Some(db)) => {
                        let da: DateTime<Utc> = da;
                        let db: DateTime<Utc> = db;
                        return da.cmp(&db);
                    },
                    (Some(_), None) => return std::cmp::Ordering::Less,
                    (None, Some(_)) => return std::cmp::Ordering::Greater,
                    (None, None) => (),
                }
            }
            b.updated.cmp(&a.updated)
        };
        let mut top_level: Vec<_> = tasks.iter()
            .filter(|t| t.parent_id.is_none() || !tasks.iter().any(|p| Some(&p.id) == t.parent_id.as_ref()))
            .cloned().collect();
        top_level.sort_by(sort_criteria);
        for parent in top_level {
            let pid = parent.id.clone(); organized.push(parent);
            let mut children: Vec<_> = tasks.iter().filter(|t| t.parent_id.as_ref() == Some(&pid)).cloned().collect();
            children.sort_by(sort_criteria); organized.extend(children);
        }
        organized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: &str, parent: Option<&str>, completed: bool, due_day: Option<u32>) -> Task {
        Task {
            id: id.to_string(),
            list_id: "L1".to_string(),
            title: id.to_string(),
            completed,
            due: due_day.and_then(|d| Utc.with_ymd_and_hms(2026, 6, d, 0, 0, 0).single()),
            updated: Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).single().unwrap(),
            completed_at: None,
            notes: None,
            parent_id: parent.map(|p| p.to_string()),
            pomodoros: 0,
        }
    }

    #[test]
    fn parse_due_date_valida_fecha_correcta() {
        let dt = App::parse_due_date("2026-06-29").expect("debe parsear");
        assert_eq!(dt.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-06-29 00:00:00");
    }

    #[test]
    fn parse_due_date_rechaza_entradas_invalidas() {
        assert!(App::parse_due_date("").is_none());
        assert!(App::parse_due_date("2026-13-40").is_none()); // mes/día fuera de rango
        assert!(App::parse_due_date("hoy").is_none());
        assert!(App::parse_due_date("2026/06/29").is_none()); // separador incorrecto
    }

    #[test]
    fn organize_coloca_subtareas_bajo_su_padre() {
        let input = vec![
            task("child", Some("parent"), false, None),
            task("parent", None, false, None),
            task("solo", None, false, None),
        ];
        let out = App::organize_tasks_hierarchical(input);
        let ids: Vec<&str> = out.iter().map(|t| t.id.as_str()).collect();
        // El hijo debe aparecer inmediatamente después de su padre.
        let p = ids.iter().position(|&i| i == "parent").unwrap();
        assert_eq!(ids[p + 1], "child");
    }

    #[test]
    fn organize_ordena_completadas_al_final() {
        let input = vec![
            task("done", None, true, None),
            task("pending", None, false, None),
        ];
        let out = App::organize_tasks_hierarchical(input);
        assert_eq!(out.first().unwrap().id, "pending");
        assert_eq!(out.last().unwrap().id, "done");
    }

    #[test]
    fn prune_before_conserva_solo_lo_reciente() {
        let mut map = BTreeMap::new();
        map.insert("2025-01-15 10:00".to_string(), 3u64); // antiguo
        map.insert("2026-06-29 09:00".to_string(), 5u64); // reciente
        map.insert("2026-05-29 00:00".to_string(), 1u64); // justo en el corte
        prune_before(&mut map, "2026-05-29");
        assert!(!map.contains_key("2025-01-15 10:00"));   // eliminado
        assert!(map.contains_key("2026-06-29 09:00"));     // conservado
        assert!(map.contains_key("2026-05-29 00:00"));     // el día del corte se conserva
        assert_eq!(map.len(), 2);
    }

    #[test]
    fn organize_ordena_pendientes_por_fecha_de_vencimiento() {
        let input = vec![
            task("tarde", None, false, Some(20)),
            task("pronto", None, false, Some(5)),
            task("sin_fecha", None, false, None),
        ];
        let out = App::organize_tasks_hierarchical(input);
        let ids: Vec<&str> = out.iter().map(|t| t.id.as_str()).collect();
        // Con fecha antes que sin fecha; y la más próxima primero.
        assert_eq!(ids, vec!["pronto", "tarde", "sin_fecha"]);
    }
}

