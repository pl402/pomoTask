mod app;
mod ui;
mod events;
mod api;

use std::{io, time::Duration};
use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use color_eyre::Result;
use chrono::{DateTime, Utc, TimeZone};

use crate::app::{App, TimerMode, AppMode, InputField, Task};
use crate::events::{Event, EventHandler};
use crate::api::ApiClient;

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut events = EventHandler::new(Duration::from_millis(250));
    let mut clipboard = arboard::Clipboard::new().ok();
    
    terminal.draw(|f| ui::render(&mut app, f))?;
    let api_client = std::sync::Arc::new(ApiClient::new(events.sender()).await);
    sync_tasks(&api_client, events.sender(), &mut app).await;

    while app.running {
        terminal.draw(|f| ui::render(&mut app, f))?;
        if let Some(event) = events.next().await {
            match event {
                Event::Tick => app.tick(),
                Event::Key(key) => {
                    // ESTRATEGIA: Match por Modo primero para evitar colisión de hotkeys
                    match app.mode {
                        AppMode::Loading => {}, // No hacemos nada en carga
                        
                        AppMode::Auth => {
                            match key.code {
                                KeyCode::Char('q') => app.running = false,
                                KeyCode::Enter if app.auth_url.is_none() => { sync_tasks(&api_client, events.sender(), &mut app).await; }
                                _ => {}
                            }
                        },

                        AppMode::AuthSuccess => {},

                        AppMode::ConfirmComplete => {
                            match key.code {
                                KeyCode::Esc => { app.mode = AppMode::Timer; }
                                KeyCode::Enter => {
                                    if let Some(task) = app.tasks.get(app.selected_task) {
                                        let task_id = task.id.clone();
                                        let is_completed = task.completed;
                                        let list_id = app.task_lists[app.selected_list_idx].id.clone();
                                        let api = api_client.clone();
                                        let sender = events.sender();
                                        app.loading = true;
                                        app.mode = AppMode::Timer;
                                        tokio::spawn(async move {
                                            if api.toggle_task_completion(&list_id, &task_id, !is_completed).await.is_ok() {
                                                let tasks = api.fetch_tasks(&list_id, true).await.unwrap_or_default(); 
                                                let _ = sender.send(Event::ApiUpdate(tasks));
                                            }
                                        });
                                    }
                                }
                                _ => {}
                            }
                        },

                        AppMode::ListSelector => {
                            match key.code {
                                KeyCode::Esc => { app.mode = AppMode::Timer; }
                                KeyCode::Enter => {
                                    app.mode = AppMode::Timer;
                                    save_selection(&mut app);
                                    sync_tasks(&api_client, events.sender(), &mut app).await;
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if !app.task_lists.is_empty() {
                                        app.selected_list_idx = (app.selected_list_idx + 1) % app.task_lists.len();
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if !app.task_lists.is_empty() {
                                        if app.selected_list_idx == 0 { app.selected_list_idx = app.task_lists.len() - 1; }
                                        else { app.selected_list_idx -= 1; }
                                    }
                                }
                                _ => {}
                            }
                        },

                        AppMode::Input | AppMode::SubtaskInput | AppMode::Edit => {
                            match key.code {
                                KeyCode::Tab => {
                                    app.focused_input = match app.focused_input {
                                        InputField::Title => InputField::Notes,
                                        InputField::Notes => InputField::Due,
                                        InputField::Due => InputField::Title,
                                    };
                                }
                                KeyCode::Esc => { app.mode = AppMode::Timer; clear_inputs(&mut app); }
                                KeyCode::Enter => {
                                    if !app.input_title.is_empty() && !app.task_lists.is_empty() {
                                        let title = app.input_title.clone();
                                        let notes = if app.input_notes.is_empty() { None } else { Some(app.input_notes.clone()) };
                                        let due = parse_due_date(&app.input_due);
                                        let list_id = app.task_lists[app.selected_list_idx].id.clone();
                                        let api = api_client.clone();
                                        let sender = events.sender();
                                        let mode = app.mode;
                                        let parent_id = if mode == AppMode::SubtaskInput { app.tasks.get(app.selected_task).map(|t| t.id.clone()) } else { None };
                                        let edit_id = app.editing_task_id.clone();
                                        app.loading = true; app.mode = AppMode::Timer; clear_inputs(&mut app);
                                        tokio::spawn(async move {
                                            let res = if mode == AppMode::Edit { api.update_task(&list_id, &edit_id.unwrap(), &title, notes, due).await } else { api.create_task(&list_id, &title, notes, due, parent_id).await };
                                            if res.is_ok() { if let Ok(tasks) = api.fetch_tasks(&list_id, true).await { let _ = sender.send(Event::ApiUpdate(tasks)); } }
                                        });
                                    }
                                }
                                KeyCode::Char(c) => {
                                    match app.focused_input {
                                        InputField::Title => app.input_title.push(c),
                                        InputField::Notes => app.input_notes.push(c),
                                        InputField::Due => app.input_due.push(c),
                                    }
                                }
                                KeyCode::Backspace => {
                                    match app.focused_input {
                                        InputField::Title => { app.input_title.pop(); }
                                        InputField::Notes => { app.input_notes.pop(); }
                                        InputField::Due => { app.input_due.pop(); }
                                    }
                                }
                                _ => {}
                            }
                        },

                        AppMode::Timer => {
                            match key.code {
                                KeyCode::Char('q') => app.running = false,
                                KeyCode::Char(' ') => app.toggle_timer(),
                                KeyCode::Char('r') => app.reset_timer(),
                                KeyCode::Char('n') => { app.mode = AppMode::Input; app.focused_input = InputField::Title; }
                                KeyCode::Char('a') => { if !app.tasks.is_empty() { app.mode = AppMode::SubtaskInput; app.focused_input = InputField::Title; } }
                                KeyCode::Char('e') => {
                                    if let Some(task) = app.tasks.get(app.selected_task) {
                                        app.mode = AppMode::Edit; app.editing_task_id = Some(task.id.clone()); app.input_title = task.title.clone(); app.input_notes = task.notes.clone().unwrap_or_default(); app.input_due = task.due.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(); app.focused_input = InputField::Title;
                                    }
                                }
                                KeyCode::Char('c') => {
                                    app.config.show_completed = !app.config.show_completed;
                                    app.save_config();
                                    sync_tasks(&api_client, events.sender(), &mut app).await;
                                }
                                KeyCode::Char('l') => app.toggle_language(),
                                KeyCode::Char('s') => sync_tasks(&api_client, events.sender(), &mut app).await,
                                KeyCode::Tab => { app.mode = AppMode::ListSelector; }
                                KeyCode::Enter => {
                                    if !app.tasks.is_empty() { app.mode = AppMode::ConfirmComplete; }
                                }
                                KeyCode::Down | KeyCode::Char('j') => {
                                    if !app.tasks.is_empty() {
                                        app.selected_task = (app.selected_task + 1) % app.tasks.len();
                                        sync_active_timer_to_task(&mut app);
                                        save_selection(&mut app);
                                    }
                                }
                                KeyCode::Up | KeyCode::Char('k') => {
                                    if !app.tasks.is_empty() {
                                        if app.selected_task == 0 { app.selected_task = app.tasks.len() - 1; }
                                        else { app.selected_task -= 1; }
                                        sync_active_timer_to_task(&mut app);
                                        save_selection(&mut app);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                },
                Event::NeedsAuth(url) => {
                    app.mode = AppMode::Auth; app.auth_url = Some(url.clone()); app.loading = false;
                    let _ = open::that(&url);
                    if let Some(ref mut cb) = clipboard { let _ = cb.set_text(url); }
                }
                Event::ListsUpdate(lists) => { 
                    app.task_lists = lists; 
                    if let Some(last_id) = &app.config.last_list_id { if let Some(idx) = app.task_lists.iter().position(|l| &l.id == last_id) { app.selected_list_idx = idx; } }
                    app.loading = false; sync_tasks(&api_client, events.sender(), &mut app).await; 
                }
                Event::ApiUpdate(tasks) => {
                    let old_mode = app.mode;
                    let mut tasks_with_stats = Vec::new();
                    for mut t in tasks { t.pomodoros = *app.stats.task_pomodoros.get(&t.id).unwrap_or(&0); tasks_with_stats.push(t); }
                    app.tasks = organize_tasks_hierarchical(tasks_with_stats);
                    if let Some(last_id) = &app.config.last_task_id { if let Some(idx) = app.tasks.iter().position(|t| &t.id == last_id) { app.selected_task = idx; } }
                    sync_active_timer_to_task(&mut app); app.loading = false;
                    if old_mode == AppMode::Auth || old_mode == AppMode::Loading {
                        if old_mode == AppMode::Auth {
                            app.mode = AppMode::AuthSuccess;
                            let _ = notify_rust::Notification::new().summary("PomoTask").body(app.translate("auth_success_msg")).icon("emblem-success").show();
                            let sender = events.sender(); tokio::spawn(async move { tokio::time::sleep(Duration::from_secs(3)).await; let _ = sender.send(Event::Tick); });
                        } else { app.mode = AppMode::Timer; }
                    }
                }
                _ => { if app.mode == AppMode::AuthSuccess && !app.task_lists.is_empty() { app.mode = AppMode::Timer; } }
            }
        }
    }
    disable_raw_mode()?; execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?; terminal.show_cursor()?; std::process::exit(0);
}

fn is_input_mode(app: &App) -> bool { matches!(app.mode, AppMode::Input | AppMode::SubtaskInput | AppMode::Edit) }
fn clear_inputs(app: &mut App) { app.input_title.clear(); app.input_notes.clear(); app.input_due.clear(); app.editing_task_id = None; }
fn parse_due_date(input: &str) -> Option<DateTime<Utc>> {
    let parts: Vec<&str> = input.split('-').collect();
    if parts.len() == 3 {
        let y: i32 = parts[0].parse().ok()?; let m: u32 = parts[1].parse().ok()?; let d: u32 = parts[2].parse().ok()?;
        Utc.with_ymd_and_hms(y, m, d, 12, 0, 0).single()
    } else { None }
}
fn save_selection(app: &mut App) {
    if let Some(list) = app.task_lists.get(app.selected_list_idx) { app.config.last_list_id = Some(list.id.clone()); }
    if let Some(task) = app.tasks.get(app.selected_task) { app.config.last_task_id = Some(task.id.clone()); }
    app.save_config();
}
fn sync_active_timer_to_task(app: &mut App) {
    if app.timer_active { return; }
    if let Some(task) = app.tasks.get(app.selected_task) {
        if let Some(state) = app.stats.task_timers.get(&task.id) { app.timer_seconds = state.remaining; app.timer_mode = state.mode; }
        else { app.timer_mode = TimerMode::Focus; app.timer_seconds = app.timer_mode.duration(&app.config); }
    }
}
async fn sync_tasks(api: &std::sync::Arc<ApiClient>, sender: tokio::sync::mpsc::UnboundedSender<Event>, app: &mut App) {
    if app.loading && app.mode != AppMode::Loading { return; }
    app.loading = true; let api = api.clone(); let show_comp = app.config.show_completed;
    if app.task_lists.is_empty() {
        tokio::spawn(async move {
            match api.fetch_task_lists().await {
                Ok(lists) => { let _ = sender.send(Event::ListsUpdate(lists)); }
                Err(_) => { let _ = sender.send(Event::ApiUpdate(Vec::new())); }
            }
        });
    } else {
        let list_id = app.task_lists[app.selected_list_idx].id.clone();
        tokio::spawn(async move {
            match api.fetch_tasks(&list_id, show_comp).await {
                Ok(tasks) => { let _ = sender.send(Event::ApiUpdate(tasks)); }
                Err(_) => { let _ = sender.send(Event::ApiUpdate(Vec::new())); }
            }
        });
    }
}

fn organize_tasks_hierarchical(tasks: Vec<Task>) -> Vec<Task> {
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
