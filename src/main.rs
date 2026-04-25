mod app;
mod ui;
mod events;
mod api;
mod handler;

use std::{io, time::Duration};
use ratatui::{backend::CrosstermBackend, Terminal};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use color_eyre::Result;

use crate::app::{App, AppMode, TaskList};
use crate::events::{Event, EventHandler};
use crate::api::ApiClient;
use crate::handler::{handle_key_events, sync_tasks};

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--version".to_string()) || args.contains(&"-v".to_string()) {
        println!("PomoTask-CLI Version: {}", env!("APP_VERSION"));
        return Ok(());
    }

    color_eyre::install()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut events = EventHandler::new(Duration::from_millis(50));
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
                    handle_key_events(key, &mut app, &api_client, &events.sender()).await;
                },
                Event::NeedsAuth(url) => {
                    app.mode = AppMode::Auth; app.auth_url = Some(url.clone()); app.loading = false;
                    let _ = open::that(&url);
                    if let Some(ref mut cb) = clipboard { let _ = cb.set_text(url); }
                }
                Event::ListsUpdate(mut lists) => { 
                    let mut all_lists = vec![TaskList { id: "@all".to_string(), title: app.translate("list_all") }];
                    all_lists.append(&mut lists);
                    app.task_lists = all_lists; 
                    if let Some(last_id) = &app.config.last_list_id { if let Some(idx) = app.task_lists.iter().position(|l| &l.id == last_id) { app.selected_list_idx = idx; } }
                    app.loading = false; sync_tasks(&api_client, events.sender(), &mut app).await; 
                }
                Event::Sync => {
                    sync_tasks(&api_client, events.sender(), &mut app).await;
                }
                Event::ApiUpdate(tasks) => {
                    app.creating_task_temp_id = None;
                    let old_mode = app.mode;
                    let mut tasks_with_stats = Vec::new();
                    for mut t in tasks { 
                        t.pomodoros = *app.stats.task_pomodoros.get(&t.id).unwrap_or(&0); 
                        tasks_with_stats.push(t); 
                    }
                    
                    // Guardar todas para el calendario
                    app.all_tasks = tasks_with_stats.clone();
                    
                    // Filtrar para la lista visual
                    let filtered_tasks = if app.config.show_completed {
                        tasks_with_stats
                    } else {
                        tasks_with_stats.into_iter().filter(|t| !t.completed).collect()
                    };
                    
                    app.tasks = app.organize_tasks_hierarchical(filtered_tasks);
                    if let Some(last_id) = &app.config.last_task_id { 
                        if let Some(idx) = app.tasks.iter().position(|t| &t.id == last_id) { 
                            app.selected_task = idx; 
                        } else {
                            app.selected_task = app.selected_task.min(app.tasks.len().saturating_sub(1));
                        }
                    }
                    app.sync_active_timer_to_task(); app.loading = false;
                    if old_mode == AppMode::Auth || old_mode == AppMode::Loading {
                        if old_mode == AppMode::Auth {
                            app.mode = AppMode::AuthSuccess;
                            let _ = notify_rust::Notification::new().summary("PomoTask").body(&app.translate("auth_success_msg")).icon("emblem-success").show();
                            let sender = events.sender(); tokio::spawn(async move { tokio::time::sleep(Duration::from_secs(3)).await; let _ = sender.send(Event::Tick); });
                        } else { app.mode = AppMode::Timer; }
                    }
                }
                Event::ApiTaskCompleted(id, x, y, w) => {
                    if app.marking_done_task_id.as_ref() == Some(&id) {
                        app.marking_done_task_id = None;
                        app.start_completion_animation(id, x, y, w);
                    }
                }
                Event::ApiTaskFailed(id) => {
                    if app.marking_done_task_id.as_ref() == Some(&id) {
                        app.marking_done_task_id = None;
                    }
                    if app.creating_task_temp_id.as_ref() == Some(&id) {
                        app.creating_task_temp_id = None;
                    }
                }
                _ => { if app.mode == AppMode::AuthSuccess && !app.task_lists.is_empty() { app.mode = AppMode::Timer; } }
            }
        }
    }
    disable_raw_mode()?; execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?; terminal.show_cursor()?; std::process::exit(0);
}
