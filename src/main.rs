use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

use pomotask_cli::api::ApiClient;
use pomotask_cli::app::{App, AppMode, TaskList};
use pomotask_cli::events::{Event, EventHandler};
use pomotask_cli::handler::{handle_key_events, sync_all_lists, sync_tasks};
use pomotask_cli::ipc::execute_ipc_command;
use pomotask_cli::ui;

#[tokio::main]
async fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.contains(&"--version".to_string()) || args.contains(&"-v".to_string()) {
        println!("PomoTask-CLI Version: {}", env!("APP_VERSION"));
        return Ok(());
    }

    if let Some(pos) = args.iter().position(|a| a == "ipc" || a == "--ipc") {
        match execute_ipc_command(&args[pos..]).await {
            Ok(output) => {
                if !output.is_empty() {
                    println!("{}", output);
                }
                std::process::exit(0);
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
    }

    color_eyre::install()?;
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    // No capturamos el mouse: así el usuario conserva la selección/copiado nativo de su terminal.
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let mut events = EventHandler::new(Duration::from_millis(50));
    let mut clipboard = arboard::Clipboard::new().ok();

    terminal.draw(|f| ui::render(&mut app, f))?;
    let api_client = std::sync::Arc::new(ApiClient::new(events.sender()).await);
    sync_all_lists(&api_client, events.sender(), &mut app);

    while app.running {
        terminal.draw(|f| ui::render(&mut app, f))?;
        if let Some(event) = events.next().await {
            match event {
                Event::Tick => {
                    app.tick();
                    // Sincronización automática en segundo plano según el intervalo configurado.
                    if app.sync_due() {
                        sync_all_lists(&api_client, events.sender(), &mut app);
                    }
                }
                Event::Key(key) => {
                    handle_key_events(key, &mut app, &api_client, &events.sender()).await;
                    // Si el handler solicitó copiar texto, lo volcamos con el portapapeles persistente.
                    if let Some(text) = app.clipboard_request.take() {
                        if let Some(ref mut cb) = clipboard {
                            let _ = cb.set_text(text);
                        }
                    }
                }
                Event::NeedsAuth(url) => {
                    app.mode = AppMode::Auth;
                    app.auth_url = Some(url.clone());
                    app.loading = false;
                    let _ = open::that(&url);
                    if let Some(ref mut cb) = clipboard {
                        let _ = cb.set_text(url);
                    }
                }
                Event::ListsUpdate(mut lists) => {
                    let mut all_lists = vec![TaskList {
                        id: "@all".to_string(),
                        title: app.translate("list_all"),
                    }];
                    all_lists.append(&mut lists);
                    app.task_lists = all_lists;
                    if let Some(last_id) = &app.config.last_list_id {
                        if let Some(idx) = app.task_lists.iter().position(|l| &l.id == last_id) {
                            app.selected_list_idx = idx;
                        }
                    }
                    // Ya tenemos las listas: sincronizamos TODAS en segundo plano.
                    sync_all_lists(&api_client, events.sender(), &mut app);
                }
                Event::Sync => {
                    sync_tasks(&api_client, events.sender(), &mut app).await;
                }
                Event::SyncFailed => {
                    // Sin conexión: salimos de la pantalla de carga y mostramos la caché local.
                    app.loading = false;
                    app.rebuild_visible_tasks();
                    if app.mode == AppMode::Loading {
                        app.mode = AppMode::Timer;
                    }
                }
                Event::ApiUpdate(list_id, tasks) => {
                    app.creating_task_temp_id = None;
                    let mut tasks_with_stats = Vec::new();
                    for mut t in tasks {
                        t.pomodoros = *app.stats.task_pomodoros.get(&t.id).unwrap_or(&0);
                        tasks_with_stats.push(t);
                    }

                    // Siempre actualizamos la caché de ESA lista (aunque ya no la estemos viendo).
                    app.list_cache
                        .insert(list_id.clone(), tasks_with_stats.clone());
                    app.save_tasks_cache();

                    // Solo refrescamos la vista si la respuesta corresponde a la lista seleccionada
                    // actual; así una respuesta tardía de otra lista no pisa lo que estás viendo.
                    let current_id = app
                        .task_lists
                        .get(app.selected_list_idx)
                        .map(|l| l.id.clone())
                        .unwrap_or_default();
                    if list_id == current_id {
                        let old_mode = app.mode;
                        app.all_tasks = tasks_with_stats;
                        app.rebuild_visible_tasks();
                        if let Some(last_id) = &app.config.last_task_id {
                            if let Some(idx) = app.tasks.iter().position(|t| &t.id == last_id) {
                                app.selected_task = idx;
                            } else {
                                app.selected_task =
                                    app.selected_task.min(app.tasks.len().saturating_sub(1));
                            }
                        }
                        app.sync_active_timer_to_task();
                        app.loading = false;
                        if old_mode == AppMode::Auth || old_mode == AppMode::Loading {
                            if old_mode == AppMode::Auth {
                                app.mode = AppMode::AuthSuccess;
                                // ~3 s en pantalla (ticks de 50 ms); app.tick() hace la transición al temporizador.
                                app.auth_success_frames = 60;
                                let _ = notify_rust::Notification::new()
                                    .summary("PomoTask")
                                    .body(&app.translate("auth_success_msg"))
                                    .icon("emblem-success")
                                    .show();
                            } else {
                                app.mode = AppMode::Timer;
                            }
                        }
                    }
                }
                Event::ApiTaskCompleted(id, x, y, w)
                    if app.marking_done_task_id.as_ref() == Some(&id) =>
                {
                    app.marking_done_task_id = None;
                    app.start_completion_animation(id, x, y, w);
                }
                Event::ApiTaskFailed(id) => {
                    if app.marking_done_task_id.as_ref() == Some(&id) {
                        app.marking_done_task_id = None;
                    }
                    if app.creating_task_temp_id.as_ref() == Some(&id) {
                        app.creating_task_temp_id = None;
                    }
                }
                _ => {}
            }
        }
    }
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    std::process::exit(0);
}
