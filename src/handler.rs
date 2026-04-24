use crossterm::event::{KeyCode, KeyEvent};
use std::time::Duration;
use crate::app::{App, AppMode, InputField, DatePreset, Theme};
use crate::events::Event;
use crate::api::ApiClient;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;
use chrono::{Local, Duration as ChronoDuration};

pub async fn handle_key_events(
    key: KeyEvent,
    app: &mut App,
    api_client: &Arc<ApiClient>,
    sender: &UnboundedSender<Event>,
) {
    match app.mode {
        AppMode::Loading => {},
        
        AppMode::Auth => {
            match key.code {
                KeyCode::Char('q') => app.running = false,
                KeyCode::Enter if app.auth_url.is_none() => { 
                    sync_tasks(api_client, sender.clone(), app).await; 
                }
                _ => {}
            }
        },

        AppMode::AuthSuccess => {},

        AppMode::ConfirmComplete => {
            match key.code {
                KeyCode::Esc => { 
                    app.mode = AppMode::Timer; 
                    app.confirming_task_id = None; 
                }
                KeyCode::Enter => {
                    let task_to_toggle = if let Some(id) = &app.confirming_task_id {
                        app.tasks.iter().find(|t| &t.id == id).cloned()
                    } else {
                        app.tasks.get(app.selected_task).cloned()
                    };

                    if let Some(mut task) = task_to_toggle {
                        let task_id = task.id.clone();
                        let is_completed = task.completed;
                        let task_list_id = task.list_id.clone();
                        let selected_list_id = app.task_lists[app.selected_list_idx].id.clone();
                        let api = api_client.clone();
                        let sender_clone = sender.clone();
                        let show_comp = app.config.show_completed;
                        let timer_active = app.timer_active;

                        let is_main_task = if let Some(current) = app.tasks.get(app.selected_task) { current.id == task_id } else { false };
                        if is_main_task && app.timer_active && !is_completed {
                            app.reset_timer();
                        }

                        if !is_completed && !app.timer_active {
                            task.completed = true;
                            let x = 3; 
                            let y = 1 + 5 + 1 + app.selected_task as u16; 
                            let w = task.title.len() as u16 + 5;
                            app.start_completion_animation(task_id.clone(), x, y, w);
                            app.record_task_done();
                        }

                        app.loading = true;
                        app.mode = AppMode::Timer;
                        app.confirming_task_id = None;
                        tokio::spawn(async move {
                            if !is_completed && !timer_active { tokio::time::sleep(Duration::from_millis(1500)).await; }
                            if api.toggle_task_completion(&task_list_id, &task_id, !is_completed).await.is_ok() {
                                if selected_list_id == "@all" {
                                    let _ = sender_clone.send(Event::Sync);
                                } else {
                                    let tasks = api.fetch_tasks(&selected_list_id, show_comp).await.unwrap_or_default(); 
                                    let _ = sender_clone.send(Event::ApiUpdate(tasks));
                                }
                            } else {
                                let _ = sender_clone.send(Event::Sync);
                            }
                        });
                    }
                }
                _ => {}
            }
        },

        AppMode::Help => {
            app.mode = AppMode::Timer;
        },

        AppMode::ConfirmLogout => {
            match key.code {
                KeyCode::Esc => { app.mode = AppMode::Settings; }
                KeyCode::Enter => {
                    app.logout();
                    app.running = false;
                }
                _ => {}
            }
        },

        AppMode::Settings => {
            match key.code {
                KeyCode::Esc => { app.mode = AppMode::Timer; }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.selected_settings_idx > 0 { app.selected_settings_idx -= 1; }
                    else { app.selected_settings_idx = 5; }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.selected_settings_idx < 5 { app.selected_settings_idx += 1; }
                    else { app.selected_settings_idx = 0; }
                }
                KeyCode::Left | KeyCode::Char('h') => {
                    match app.selected_settings_idx {
                        0 => { if app.config.focus_duration > 60 { app.config.focus_duration -= 60; } }
                        1 => { if app.config.short_break_duration > 60 { app.config.short_break_duration -= 60; } }
                        2 => { if app.config.long_break_duration > 60 { app.config.long_break_duration -= 60; } }
                        3 => { app.toggle_language(); }
                        4 => {
                            app.config.theme = match app.config.theme {
                                Theme::CatppuccinMocha => Theme::Dracula,
                                Theme::Nord => Theme::CatppuccinMocha,
                                Theme::Gruvbox => Theme::Nord,
                                Theme::Dracula => Theme::Gruvbox,
                            };
                        }
                        _ => {}
                    }
                    app.save_config();
                    app.timer_seconds = app.timer_mode.duration(&app.config);
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    match app.selected_settings_idx {
                        0 => { app.config.focus_duration += 60; }
                        1 => { app.config.short_break_duration += 60; }
                        2 => { app.config.long_break_duration += 60; }
                        3 => { app.toggle_language(); }
                        4 => {
                            app.config.theme = match app.config.theme {
                                Theme::CatppuccinMocha => Theme::Nord,
                                Theme::Nord => Theme::Gruvbox,
                                Theme::Gruvbox => Theme::Dracula,
                                Theme::Dracula => Theme::CatppuccinMocha,
                            };
                            app.save_config();
                        }
                        5 => {
                            if key.code == KeyCode::Enter || key.code == KeyCode::Right || key.code == KeyCode::Char('l') {
                                app.mode = AppMode::ConfirmLogout;
                            }
                        }
                        _ => {}
                    }
                    app.save_config();
                    app.timer_seconds = app.timer_mode.duration(&app.config);
                }
                KeyCode::Enter => {
                    if app.selected_settings_idx == 5 {
                        app.mode = AppMode::ConfirmLogout;
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
                    app.save_selection();
                    sync_tasks(api_client, sender.clone(), app).await;
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
                        InputField::Notes => {
                            if app.mode == AppMode::Input { InputField::List }
                            else { InputField::Due }
                        },
                        InputField::List => InputField::Due,
                        InputField::Due => InputField::Title,
                    };
                }
                KeyCode::Esc => { app.mode = AppMode::Timer; app.clear_inputs(); }
                KeyCode::Enter => {
                    if !app.input_title.is_empty() && !app.task_lists.is_empty() {
                        let title = app.input_title.clone();
                        let notes = if app.input_notes.is_empty() { None } else { Some(app.input_notes.clone()) };
                        let due = app.parse_due_date(&app.input_due);
                        let target_list_id = if app.mode == AppMode::Input {
                            app.task_lists[app.input_list_idx].id.clone()
                        } else if app.mode == AppMode::Edit {
                            app.tasks.get(app.selected_task).map(|t| t.list_id.clone()).unwrap_or_else(|| app.task_lists[app.selected_list_idx].id.clone())
                        } else {
                            app.task_lists[app.selected_list_idx].id.clone()
                        };
                        let api = api_client.clone();
                        let sender_clone = sender.clone();
                        let mode = app.mode;
                        let parent_id = if mode == AppMode::SubtaskInput { 
                            app.tasks.get(app.selected_task).map(|t| {
                                t.parent_id.clone().unwrap_or(t.id.clone())
                            })
                        } else { None };
                        let edit_id = app.editing_task_id.clone();
                        app.loading = true; app.mode = AppMode::Timer; app.clear_inputs();
                        tokio::spawn(async move {
                            let res = if mode == AppMode::Edit { api.update_task(&target_list_id, &edit_id.unwrap(), &title, notes, due).await } else { api.create_task(&target_list_id, &title, notes, due, parent_id).await };
                            if res.is_ok() { let _ = sender_clone.send(Event::Sync); }
                        });
                    }
                }
                KeyCode::Left | KeyCode::Char('h') if app.focused_input == InputField::Due => {
                    app.selected_date_preset = match app.selected_date_preset {
                        DatePreset::Today => DatePreset::None,
                        DatePreset::Tomorrow => DatePreset::Today,
                        DatePreset::Custom => DatePreset::Tomorrow,
                        DatePreset::None => DatePreset::Custom,
                    };
                    app.set_date_preset(app.selected_date_preset);
                }
                KeyCode::Left | KeyCode::Char('h') if app.focused_input == InputField::List => {
                    if app.input_list_idx > 1 || (app.input_list_idx > 0 && app.task_lists[0].id != "@all") {
                        app.input_list_idx -= 1;
                    } else {
                        app.input_list_idx = app.task_lists.len() - 1;
                    }
                }
                KeyCode::Right | KeyCode::Char('l') if app.focused_input == InputField::Due => {
                    app.selected_date_preset = match app.selected_date_preset {
                        DatePreset::Today => DatePreset::Tomorrow,
                        DatePreset::Tomorrow => DatePreset::Custom,
                        DatePreset::Custom => DatePreset::None,
                        DatePreset::None => DatePreset::Today,
                    };
                    app.set_date_preset(app.selected_date_preset);
                }
                KeyCode::Right | KeyCode::Char('l') if app.focused_input == InputField::List => {
                    app.input_list_idx = (app.input_list_idx + 1) % app.task_lists.len();
                    if app.input_list_idx == 0 && app.task_lists[0].id == "@all" {
                        app.input_list_idx = 1;
                    }
                }
                KeyCode::Char(c) => {
                    match app.focused_input {
                        InputField::Title => app.input_title.push(c),
                        InputField::Notes => app.input_notes.push(c),
                        InputField::Due => {
                            app.selected_date_preset = DatePreset::Custom;
                            app.input_due.push(c);
                        }
                        InputField::List => {}
                    }
                }
                KeyCode::Backspace => {
                    match app.focused_input {
                        InputField::Title => { app.input_title.pop(); }
                        InputField::Notes => { app.input_notes.pop(); }
                        InputField::Due => { 
                            app.selected_date_preset = DatePreset::Custom;
                            app.input_due.pop(); 
                        }
                        InputField::List => {}
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
                KeyCode::Char('n') => { 
                    app.mode = AppMode::Input; 
                    app.focused_input = InputField::Title;
                    app.set_date_preset(DatePreset::Today);
                    if app.task_lists[app.selected_list_idx].id == "@all" {
                        app.input_list_idx = 1; // Primera lista real
                    } else {
                        app.input_list_idx = app.selected_list_idx;
                    }
                }
                KeyCode::Char('a') => { 
                    if !app.tasks.is_empty() && app.task_lists[app.selected_list_idx].id != "@all" { 
                        app.mode = AppMode::SubtaskInput; 
                        app.focused_input = InputField::Title;
                        app.set_date_preset(DatePreset::Today);
                    } 
                }
                KeyCode::Char('e') if !app.timer_active => {
                    if let Some(task) = app.tasks.get(app.selected_task) {
                        app.mode = AppMode::Edit; 
                        app.editing_task_id = Some(task.id.clone()); 
                        app.input_title = task.title.clone(); 
                        app.input_notes = task.notes.clone().unwrap_or_default(); 
                        
                        let date_str = task.due.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default();
                        app.input_due = date_str.clone();
                        
                        let now = Local::now().format("%Y-%m-%d").to_string();
                        let tomorrow = (Local::now() + ChronoDuration::days(1)).format("%Y-%m-%d").to_string();
                        
                        if date_str == now { app.selected_date_preset = DatePreset::Today; }
                        else if date_str == tomorrow { app.selected_date_preset = DatePreset::Tomorrow; }
                        else if date_str.is_empty() { app.selected_date_preset = DatePreset::None; }
                        else { app.selected_date_preset = DatePreset::Custom; }
                        
                        app.focused_input = InputField::Title;
                    }
                }
                KeyCode::Char('c') => {
                    app.config.show_completed = !app.config.show_completed;
                    app.save_config();
                    sync_tasks(api_client, sender.clone(), app).await;
                }
                KeyCode::Char('s') => sync_tasks(api_client, sender.clone(), app).await,
                KeyCode::Char('?') => { app.mode = AppMode::Help; }
                KeyCode::Char(',') => { app.mode = AppMode::Settings; }
                KeyCode::Char('[') => app.calendar_prev_month(),
                KeyCode::Char(']') => app.calendar_next_month(),
                KeyCode::Tab if !app.timer_active => { app.mode = AppMode::ListSelector; }
                KeyCode::Left | KeyCode::Char('h') if !app.timer_active => {
                    if app.selected_list_idx == 0 { app.selected_list_idx = app.task_lists.len() - 1; }
                    else { app.selected_list_idx -= 1; }
                    app.save_selection();
                    sync_tasks(api_client, sender.clone(), app).await;
                }
                KeyCode::Right | KeyCode::Char('l') if !app.timer_active => {
                    app.selected_list_idx = (app.selected_list_idx + 1) % app.task_lists.len();
                    app.save_selection();
                    sync_tasks(api_client, sender.clone(), app).await;
                }
                KeyCode::Enter => {
                    if app.timer_active {
                        if let Some(task) = app.tasks.get(app.selected_task) {
                            if app.focus_subtask_idx == 0 {
                                app.confirming_task_id = Some(task.id.clone());
                                app.mode = AppMode::ConfirmComplete;
                            } else {
                                let subtasks: Vec<_> = app.tasks.iter().filter(|t| t.parent_id.as_ref() == Some(&task.id)).collect();
                                if let Some(st) = subtasks.get(app.focus_subtask_idx - 1) {
                                    app.confirming_task_id = Some(st.id.clone());
                                    app.mode = AppMode::ConfirmComplete;
                                }
                            }
                        }
                    } else if !app.tasks.is_empty() { 
                        app.mode = AppMode::ConfirmComplete; 
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.timer_active {
                        if let Some(task) = app.tasks.get(app.selected_task) {
                            let subtasks_count = app.tasks.iter().filter(|t| t.parent_id.as_ref() == Some(&task.id)).count();
                            app.focus_subtask_idx = (app.focus_subtask_idx + 1) % (subtasks_count + 1);
                        }
                    } else if !app.tasks.is_empty() {
                        app.selected_task = (app.selected_task + 1) % app.tasks.len();
                        if let Some(task) = app.tasks.get(app.selected_task) {
                            if let Some(due) = task.due {
                                app.calendar_date = due.date_naive();
                            }
                        }
                        app.sync_active_timer_to_task();
                        app.save_selection();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.timer_active {
                        if let Some(task) = app.tasks.get(app.selected_task) {
                            let subtasks_count = app.tasks.iter().filter(|t| t.parent_id.as_ref() == Some(&task.id)).count();
                            if app.focus_subtask_idx == 0 { app.focus_subtask_idx = subtasks_count; }
                            else { app.focus_subtask_idx -= 1; }
                        }
                    } else if !app.tasks.is_empty() {
                        if app.selected_task == 0 { app.selected_task = app.tasks.len() - 1; }
                        else { app.selected_task -= 1; }
                        if let Some(task) = app.tasks.get(app.selected_task) {
                            if let Some(due) = task.due {
                                app.calendar_date = due.date_naive();
                            }
                        }
                        app.sync_active_timer_to_task();
                        app.save_selection();
                    }
                }
                _ => {}
            }
        }
    }
}

pub async fn sync_tasks(api: &Arc<ApiClient>, sender: UnboundedSender<Event>, app: &mut App) {
    app.loading = true; let api = api.clone();
    if app.task_lists.is_empty() {
        tokio::spawn(async move {
            match api.fetch_task_lists().await {
                Ok(lists) => { let _ = sender.send(Event::ListsUpdate(lists)); }
                Err(_) => { let _ = sender.send(Event::ApiUpdate(Vec::new())); }
            }
        });
    } else {
        let list_id = app.task_lists[app.selected_list_idx].id.clone();
        if list_id == "@all" {
            let other_lists: Vec<String> = app.task_lists.iter().filter(|l| l.id != "@all").map(|l| l.id.clone()).collect();
            tokio::spawn(async move {
                let mut all_tasks = Vec::new();
                for id in other_lists {
                    if let Ok(tasks) = api.fetch_tasks(&id, true).await { // Siempre traer completadas
                        all_tasks.extend(tasks);
                    }
                }
                let _ = sender.send(Event::ApiUpdate(all_tasks));
            });
        } else {
            tokio::spawn(async move {
                match api.fetch_tasks(&list_id, true).await { // Siempre traer completadas
                    Ok(tasks) => { let _ = sender.send(Event::ApiUpdate(tasks)); }
                    Err(_) => { /* No enviar nada para no vaciar la lista actual en caso de error de red */ }
                }
            });
        }
    }
}
