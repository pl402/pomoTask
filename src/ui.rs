use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Clear,
        Table, Row, Cell, BarChart, BarGroup, Bar,
    },
    Frame,
};
use chrono::{Local, Timelike, Utc};

use crate::app::{App, Palette, TimerMode, AppMode, InputField, DatePreset};

pub fn render(app: &mut App, frame: &mut Frame) {
    match app.mode {
        AppMode::Loading => render_loading_screen(app, frame),
        AppMode::Timer => {
            if app.timer_active && !app.tasks.is_empty() {
                render_timer_mode(app, frame);
            } else {
                render_timer_screen(app, frame);
            }
        },
        AppMode::Auth => render_auth_screen(app, frame),
        AppMode::AuthSuccess => render_success_screen(app, frame),
        AppMode::ListSelector => {
            render_timer_screen(app, frame);
            render_list_selector(app, frame);
        }
        AppMode::Input | AppMode::SubtaskInput | AppMode::Edit => {
            render_timer_screen(app, frame);
            render_input_modal(app, frame);
        }
        AppMode::ConfirmComplete => {
            if app.timer_active {
                render_timer_mode(app, frame);
            } else {
                render_timer_screen(app, frame);
            }
            render_confirm_modal(app, frame);
        }
        AppMode::Help => {
            render_timer_screen(app, frame);
            render_help_modal(app, frame);
        }
        AppMode::Settings => {
            render_timer_screen(app, frame);
            render_settings_modal(app, frame);
        }
        AppMode::ConfirmLogout => {
            render_timer_screen(app, frame);
            render_logout_confirm_modal(app, frame);
        }
    }
    render_animation_layer(app, frame);
}

fn render_left_panel(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let total = app.timer_mode.duration(&app.config);
    let progress = ((total - app.timer_seconds) as f64 / total as f64).min(1.0);
    let label = format!("{} - {:02}:{:02}", match app.timer_mode { TimerMode::Focus => app.translate("focus"), _ => app.translate("break") }, app.timer_seconds / 60, app.timer_seconds % 60);
    frame.render_widget(Gauge::default().block(Block::default().title(app.translate("timer")).borders(Borders::ALL).border_type(BorderType::Rounded)).gauge_style(Style::default().fg(if app.timer_mode == TimerMode::Focus { Palette::red(app.config.theme) } else { Palette::green(app.config.theme) }).bg(Palette::surface0(app.config.theme))).ratio(progress).label(label), chunks[0]);

    let header_cells = [" Tarea ", " 🕒 Creada ", " 📅 Venc. ", " 🍅 "].into_iter().map(|h| Cell::from(h).style(Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = app.tasks.iter().enumerate().map(|(i, task)| {
        let is_selected = i == app.selected_task;
        let is_animating = app.animation.task_id.as_ref() == Some(&task.id);
        
        let mut style = if is_selected { Style::default().fg(Palette::base(app.config.theme)).bg(Palette::mauve(app.config.theme)) } 
                    else if task.completed { Style::default().fg(Palette::overlay0(app.config.theme)) }
                    else { Style::default().fg(Palette::text(app.config.theme)) };

        if is_animating {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }

        let is_subtask = if let Some(pid) = &task.parent_id { app.tasks.iter().any(|t| &t.id == pid) } else { false };
        let indent = if is_subtask { "  ↳ " } else { "" };
        
        let title_content = format!("{}{}{}", indent, if task.completed || is_animating { "󰄲 " } else { "󰄱 " }, task.title);
        
        // CONVERSIÓN A TIEMPO LOCAL
        let created_str = app.format_date(task.updated);
        let due_str = task.due.map(|d| app.format_due_date(d)).unwrap_or_else(|| "---".to_string());
        let pomodoros_str = if task.pomodoros > 0 { format!("{}", task.pomodoros) } else { "".to_string() };

        Row::new(vec![
            Cell::from(title_content),
            Cell::from(created_str),
            Cell::from(due_str),
            Cell::from(pomodoros_str),
        ]).style(style)
    }).collect();

    let list_title = if let Some(l) = app.task_lists.get(app.selected_list_idx) { 
        format!(" {} - {} ", app.translate("tasks"), l.title) 
    } else { app.translate("tasks") };

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(55),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ]
    )
    .header(header)
    .block(Block::default().title(list_title).borders(Borders::ALL).border_type(BorderType::Rounded))
    .highlight_symbol(">> ");

    frame.render_widget(table, chunks[1]);
}

fn render_right_panel(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Gráfico
            Constraint::Length(3),  // Resumen Actividad
            Constraint::Min(0),     // Info Tarea
        ])
        .split(area);

    // Preparar datos para BarChart (últimas 8 horas)
    let now = Local::now();
    let mut bars = Vec::new();
    let mut total_mins = 0;
    let mut total_tasks = 0;

    for i in (0..24).rev() {
        let hour_ago = now - chrono::Duration::hours(i);
        let key = hour_ago.format("%Y-%m-%d %H:00").to_string();
        let seconds = app.stats.hourly_seconds.get(&key).unwrap_or(&0);
        let tasks = app.stats.hourly_tasks_done.get(&key).unwrap_or(&0);
        
        let mins = *seconds / 60;
        total_mins += mins;
        total_tasks += tasks;

        if i < 8 { 
            let label = hour_ago.format("%Hh").to_string();
            let val = mins + (*tasks * 5);
            bars.push(Bar::default().value(val).label(label.into()).style(Style::default().fg(if *tasks > 0 { Palette::mauve(app.config.theme) } else { Palette::peach(app.config.theme) })));
        }
    }

    let barchart = BarChart::default()
        .block(Block::default().title(app.translate("productivity")).borders(Borders::ALL).border_type(BorderType::Rounded))
        .data(BarGroup::default().bars(&bars))
        .bar_width(5)
        .bar_gap(1);

    frame.render_widget(barchart, chunks[0]);

    // Render Resumen de actividad en su propio bloque
    let summary_line = Line::from(vec![
        Span::styled(" 🕒 ", Style::default().fg(Palette::green(app.config.theme))),
        Span::styled(format!("{} min", total_mins), Style::default().fg(Palette::green(app.config.theme)).add_modifier(Modifier::BOLD)),
        Span::raw("  "),
        Span::styled(" ✅ ", Style::default().fg(Palette::mauve(app.config.theme))),
        Span::styled(format!("{} tareas", total_tasks), Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD)),
    ]);

    let summary_title = if app.config.language == crate::app::Language::Spanish { " Resumen (24h) " } else { " Summary (24h) " };
    frame.render_widget(
        Paragraph::new(summary_line)
            .alignment(Alignment::Center)
            .block(Block::default().title(summary_title).borders(Borders::ALL).border_type(BorderType::Rounded)),
        chunks[1]
    );

    let mut info_lines = vec![];
    if let Some(task) = app.tasks.get(app.selected_task) {
        info_lines.push(Line::from(vec![Span::styled(task.title.clone(), Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD))]));
        info_lines.push(Line::from(vec![Span::styled(format!("🍅 Pomodoros: {}", task.pomodoros), Style::default().fg(Palette::peach(app.config.theme)))]));
        info_lines.push(Line::from(""));

        // CONVERSIÓN A TIEMPO LOCAL
        let due_str = task.due.map(|d| app.format_due_date(d)).unwrap_or_else(|| "---".to_string());
        info_lines.push(Line::from(vec![Span::styled(format!("{}: ", app.translate("due_date")), Style::default().fg(Palette::subtext0(app.config.theme))), Span::raw(due_str)]));

        let created_str = task.updated.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string();
        info_lines.push(Line::from(vec![Span::styled(format!("{}: ", app.translate("created_date")), Style::default().fg(Palette::subtext0(app.config.theme))), Span::raw(created_str)]));

        info_lines.push(Line::from(""));
        info_lines.push(Line::from(vec![Span::styled(format!("{}:", app.translate("notes")), Style::default().fg(Palette::subtext0(app.config.theme)))]));
        let no_notes = app.translate("no_notes");
        let notes = task.notes.as_deref().unwrap_or(&no_notes);
        for line in notes.lines() { info_lines.push(Line::from(vec![Span::raw(format!("  {}", line))])); }
    } else { info_lines.push(Line::from(app.translate("no_task"))); }
    frame.render_widget(Paragraph::new(info_lines).wrap(ratatui::widgets::Wrap { trim: true }).block(Block::default().title(app.translate("info")).borders(Borders::ALL).border_type(BorderType::Rounded)), chunks[2]);
}

fn render_big_clock(app: &App, frame: &mut Frame, area: Rect, color: ratatui::style::Color) {
    let now = Local::now();
    let time_str = now.format("%H:%M").to_string();
    let date_str = app.format_full_date(Utc::now());

    // Definición de números en bloques (5x3)
    let blocks = [
        ["┏━┓", "┃ ┃", "┃ ┃", "┃ ┃", "┗━┛"], // 0
        [" ┓ ", " ┃ ", " ┃ ", " ┃ ", " ┻ "], // 1
        ["┏━┓", "  ┃", "┏━┛", "┃  ", "┗━┛"], // 2
        ["┏━┓", "  ┃", " ━┫", "  ┃", "┗━┛"], // 3
        ["┃ ┃", "┃ ┃", "┗━┫", "  ┃", "  ┻"], // 4
        ["┏━┓", "┃  ", "┗━┓", "  ┃", "┗━┛"], // 5
        ["┏━┓", "┃  ", "┣━┓", "┃ ┃", "┗━┛"], // 6
        ["┏━┓", "  ┃", "  ┃", "  ┃", "  ┻"], // 7
        ["┏━┓", "┃ ┃", "┣━┫", "┃ ┃", "┗━┛"], // 8
        ["┏━┓", "┃ ┃", "┗━┫", "  ┃", "┗━┛"], // 9
        ["   ", " ⏺ ", "   ", " ⏺ ", "   "], // :
    ];

    let mut lines = vec![vec![]; 5];
    for c in time_str.chars() {
        let idx = match c {
            '0'..='9' => c as usize - '0' as usize,
            ':' => 10,
            _ => continue,
        };
        for i in 0..5 {
            lines[i].push(Span::styled(format!(" {} ", blocks[idx][i]), Style::default().fg(color).add_modifier(Modifier::BOLD)));
        }
    }

    let mut final_lines: Vec<Line> = lines.into_iter().map(Line::from).collect();
    final_lines.push(Line::from(""));
    final_lines.push(Line::from(Span::styled(date_str, Style::default().fg(color).add_modifier(Modifier::ITALIC))));

    frame.render_widget(Paragraph::new(final_lines).alignment(Alignment::Center), area);
}

fn render_timer_mode(app: &App, frame: &mut Frame) {
    let area = frame.size();
    let is_focus = app.timer_mode == TimerMode::Focus;
    let color = if is_focus { Palette::red(app.config.theme) } else { Palette::green(app.config.theme) };

    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(color));
    frame.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(10),
        Constraint::Length(5),
        Constraint::Length(4),
        Constraint::Min(0)
    ]).margin(5).split(area);

    render_big_clock(app, frame, chunks[0], color);

    let total = app.timer_mode.duration(&app.config);
    let progress = ((total - app.timer_seconds) as f64 / total as f64).min(1.0);
    let time_label = format!("{:02}:{:02}", app.timer_seconds / 60, app.timer_seconds % 60);

    frame.render_widget(Gauge::default().block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)).gauge_style(Style::default().fg(color).bg(Palette::surface0(app.config.theme))).ratio(progress).label(time_label), chunks[1]);

    if let Some(task) = app.tasks.get(app.selected_task) {
        let is_main_selected = app.focus_subtask_idx == 0;
        let msg = if is_focus {
            let now = Local::now();
            let seed = task.id.chars().map(|c| c as u32).sum::<u32>() + now.minute();
            let msg_key = format!("focus_msg_{}", seed % 10);
            app.translate(&msg_key)
        } else {
            app.translate("break")
        };

        let mut main_style = Style::default();
        if is_main_selected {
            main_style = main_style.bg(Palette::surface0(app.config.theme));
        }

        let focus_text = vec![
            Line::from(vec![
                Span::styled(msg, Style::default().fg(Palette::text(app.config.theme))),
                Span::styled(task.title.clone(), Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD))
            ]),
            Line::from(vec![
                Span::styled(format!("🍅 {} {}", task.pomodoros, app.translate("focus_completed_today")), Style::default().fg(Palette::peach(app.config.theme)))
            ])
        ];
        frame.render_widget(Paragraph::new(focus_text).alignment(Alignment::Center).wrap(ratatui::widgets::Wrap { trim: true }).style(main_style), chunks[2]);

        let subtasks: Vec<_> = app.tasks.iter().filter(|t| t.parent_id.as_ref() == Some(&task.id)).collect();
        if !subtasks.is_empty() {
             let mut items = vec![];
             for (idx, st) in subtasks.iter().enumerate() {
                 let is_selected = app.focus_subtask_idx == idx + 1;
                 let symbol = if st.completed { "✅" } else { "☐" };
                 let style = if is_selected { 
                     Style::default().fg(Palette::base(app.config.theme)).bg(Palette::yellow(app.config.theme))
                 } else { 
                     Style::default().fg(Palette::text(app.config.theme)) 
                 };
                 items.push(ListItem::new(format!(" {} {}", symbol, st.title)).style(style));
             }
             let list = List::new(items).block(Block::default().title(format!(" {} ", app.translate("new_subtask"))).borders(Borders::ALL).border_type(BorderType::Rounded));
             frame.render_widget(list, chunks[3]);
        }
    }
}

fn render_loading_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(40, 20, frame.size());
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = frames[app.spinner_frame % frames.len()];
    let loading = Paragraph::new(format!("{} {}", spinner, app.translate("loading_app"))).alignment(Alignment::Center).style(Style::default().fg(Palette::mauve(app.config.theme))).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    frame.render_widget(loading, area);
}

fn render_input_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 70, frame.size());
    frame.render_widget(Clear, area);
    let title_key = match app.mode { AppMode::SubtaskInput => "subtask_title", AppMode::Edit => "edit_title", _ => "input_title" };
    let mut modal_title = app.translate(title_key).to_string();
    if app.mode == AppMode::SubtaskInput {
        if let Some(parent) = app.tasks.get(app.selected_task) {
            modal_title = format!(" {} para {} ", app.translate("new_subtask"), parent.title);
        }
    }
    frame.render_widget(Block::default().title(modal_title).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::yellow(app.config.theme))), area);

    let mut constraints = vec![
        Constraint::Length(3), // Título
        Constraint::Length(5), // Notas
    ];
    if app.mode == AppMode::Input {
        constraints.push(Constraint::Length(3)); // Lista de Destino
    }
    constraints.extend_from_slice(&[
        Constraint::Length(3), // Presets de Fecha
        Constraint::Length(3), // Input de Fecha
        Constraint::Min(0)     // Hint
    ]);

    let chunks = Layout::default().direction(Direction::Vertical).constraints(constraints).margin(2).split(area);

    let title_style = if app.focused_input == InputField::Title { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::subtext0(app.config.theme)) };
    let title_width = chunks[0].width.max(3) - 2;
    let title_scroll = (app.input_title.chars().count() as u16).saturating_sub(title_width);
    frame.render_widget(Paragraph::new(app.input_title.as_str()).scroll((0, title_scroll)).block(Block::default().title(" Título ").borders(Borders::ALL).border_style(title_style)), chunks[0]);

    let notes_style = if app.focused_input == InputField::Notes { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::subtext0(app.config.theme)) };
    let notes_width = chunks[1].width.max(3) - 2;
    let notes_scroll = (app.input_notes.chars().count() as u16).saturating_sub(notes_width);
    frame.render_widget(Paragraph::new(app.input_notes.as_str()).scroll((0, notes_scroll)).block(Block::default().title(" Notas ").borders(Borders::ALL).border_style(notes_style)), chunks[1]);

    let mut next_idx = 2;
    if app.mode == AppMode::Input {
        let list_style = if app.focused_input == InputField::List { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::subtext0(app.config.theme)) };
        let list_name = app.task_lists.get(app.input_list_idx).map(|l| l.title.as_str()).unwrap_or("---");
        let list_text = format!(" ← {} → ", list_name);
        frame.render_widget(Paragraph::new(list_text).alignment(Alignment::Center).block(Block::default().title(app.translate("list_selection")).borders(Borders::ALL).border_style(list_style)), chunks[next_idx]);
        next_idx += 1;
    }

    // Botones de Fecha
    let preset_chunks = Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ]).split(chunks[next_idx]);
    next_idx += 1;

    let presets = [
        (DatePreset::Today, app.translate("date_today")),
        (DatePreset::Tomorrow, app.translate("date_tomorrow")),
        (DatePreset::Custom, app.translate("date_custom")),
        (DatePreset::None, app.translate("date_none")),
    ];

    for (idx, (preset, label)) in presets.iter().enumerate() {
        let is_selected = app.selected_date_preset == *preset;
        let mut style = Style::default().fg(Palette::text(app.config.theme));
        if is_selected {
            style = Style::default().fg(Palette::base(app.config.theme)).bg(Palette::yellow(app.config.theme));
        }
        let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(if is_selected && app.focused_input == InputField::Due { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::surface0(app.config.theme)) });
        frame.render_widget(Paragraph::new(label.as_str()).alignment(Alignment::Center).style(style).block(block), preset_chunks[idx]);
    }

    let due_style = if app.focused_input == InputField::Due { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::subtext0(app.config.theme)) };
    let date_input_text = if app.selected_date_preset == DatePreset::Custom { app.input_due.as_str() } else { app.input_due.as_str() }; 
    let due_width = chunks[next_idx].width.max(3) - 2;
    let due_scroll = (date_input_text.chars().count() as u16).saturating_sub(due_width);
    
    frame.render_widget(Paragraph::new(date_input_text).scroll((0, due_scroll)).block(Block::default().title(format!(" {} {} ", app.translate("due_date"), app.translate("due_date_hint"))).borders(Borders::ALL).border_style(due_style)), chunks[next_idx]);
    next_idx += 1;
    
    let input_hint = if app.focused_input == InputField::Due {
        if app.config.language == crate::app::Language::Spanish { " ←→: Cambiar Fecha | ENTER: Guardar | ESC: Salir ".to_string() } else { " ←→: Change Date | ENTER: Save | ESC: Exit ".to_string() }
    } else if app.focused_input == InputField::List {
        if app.config.language == crate::app::Language::Spanish { " ←→: Cambiar Lista | ENTER: Guardar | ESC: Salir ".to_string() } else { " ←→: Change List | ENTER: Save | ESC: Exit ".to_string() }
    } else {
        app.translate("input_hint")
    };
    frame.render_widget(Paragraph::new(input_hint).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[next_idx]);
}

fn render_timer_screen(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Min(0), Constraint::Length(1)]).split(frame.size());
    let content = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(chunks[0]);
    render_left_panel(app, frame, content[0]);
    render_right_panel(app, frame, content[1]);
    render_footer(app, frame, chunks[1]);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H:%M").to_string();
    let spinner = if app.loading { let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]; frames[app.spinner_frame % frames.len()] } else { "✓" };
    
    let timer_label = format!("{:02}:{:02}", app.timer_seconds / 60, app.timer_seconds % 60);

    let left_spans = vec![
        Span::styled(app.translate("title").to_string(), Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} ", app.translate("footer_hint")), Style::default().fg(Palette::yellow(app.config.theme))),
    ];

    let right_spans = vec![
        Span::styled(format!(" {} ", app.translate("timer_short")), Style::default().fg(Palette::subtext0(app.config.theme))),
        Span::styled(format!("{} ", timer_label), Style::default().fg(if app.timer_mode == TimerMode::Focus { Palette::red(app.config.theme) } else { Palette::green(app.config.theme) })),
        Span::styled(format!(" {} ", app.translate("sync_short")), Style::default().fg(Palette::subtext0(app.config.theme))),
        Span::styled(format!("{} ", spinner), Style::default().fg(Palette::blue(app.config.theme))),
        Span::styled(format!(" {} ", app.translate("pomodoro_short")), Style::default().fg(Palette::subtext0(app.config.theme))),
        Span::styled(format!("{} ", app.session_pomodoros), Style::default().fg(Palette::peach(app.config.theme))),
        Span::styled(format!(" {} | {} ", date_str, time_str), Style::default().fg(Palette::text(app.config.theme))),
    ];

    let footer_line = Line::from(left_spans);
    frame.render_widget(Paragraph::new(footer_line).alignment(Alignment::Left).style(Style::default().bg(Palette::surface0(app.config.theme))), area);
    
    let status_line = Line::from(right_spans);
    frame.render_widget(Paragraph::new(status_line).alignment(Alignment::Right), area);
}

fn render_help_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.size());
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(app.translate("help_title"))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Palette::mauve(app.config.theme)));
    
    let keys = vec![
        ("SPACE", app.translate("start_pause")),
        ("ENTER", app.translate("complete_task_hotkey")),
        ("j/k", app.translate("j_k_navigate_timer")),
        ("h/l", app.translate("h_l_change_list")),
        ("Tab", app.translate("change_list")),
        ("N", app.translate("new_task")),
        ("A", app.translate("new_subtask")),
        ("E", app.translate("edit_task")),
        ("C", app.translate("toggle_completed")),
        ("S", app.translate("sync_manual")),
        (",", app.translate("settings_title")),
        ("Q", app.translate("quit")),
        ("?", app.translate("help_label")),
    ];

    let rows: Vec<Row> = keys.iter().map(|(k, v)| {
        Row::new(vec![
            Cell::from(*k).style(Style::default().fg(Palette::yellow(app.config.theme)).add_modifier(Modifier::BOLD)),
            Cell::from(v.clone()).style(Style::default().fg(Palette::text(app.config.theme))),
        ])
    }).collect();

    let table = Table::new(rows, [Constraint::Percentage(30), Constraint::Percentage(70)]).block(block);
    frame.render_widget(table, area);
}

fn render_settings_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 50, frame.size());
    frame.render_widget(Clear, area);
    
    let block = Block::default().title(app.translate("settings_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::yellow(app.config.theme)));
    frame.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Min(0), Constraint::Length(1)]).margin(2).split(area);

    let settings = vec![
        (app.translate("settings_focus"), format!("{} min", app.config.focus_duration / 60)),
        (app.translate("settings_short"), format!("{} min", app.config.short_break_duration / 60)),
        (app.translate("settings_long"), format!("{} min", app.config.long_break_duration / 60)),
        (app.translate("settings_lang"), match app.config.language { crate::app::Language::Spanish => "Español".to_string(), crate::app::Language::English => "English".to_string() }),
        (app.translate("settings_theme"), format!("{:?}", app.config.theme)),
        (app.translate("settings_logout"), "".to_string()),
    ];

    let rows: Vec<Row> = settings.iter().enumerate().map(|(i, (k, v))| {
        let style = if i == app.selected_settings_idx { Style::default().fg(Palette::base(app.config.theme)).bg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::text(app.config.theme)) };
        Row::new(vec![Cell::from(k.clone()), Cell::from(v.clone())]).style(style)
    }).collect();

    let table = Table::new(rows, [Constraint::Percentage(60), Constraint::Percentage(40)]);
    frame.render_widget(table, chunks[0]);

    frame.render_widget(Paragraph::new(app.translate("settings_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[1]);
}

fn render_logout_confirm_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 20, frame.size());
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().title(app.translate("logout_confirm_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::red(app.config.theme))), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
    frame.render_widget(Paragraph::new(app.translate("logout_confirm_msg")).alignment(Alignment::Center), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[2]);
}

fn render_confirm_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 20, frame.size());
    frame.render_widget(Clear, area);
    
    let task = if let Some(id) = &app.confirming_task_id {
        app.tasks.iter().find(|t| &t.id == id).cloned()
    } else {
        app.tasks.get(app.selected_task).cloned()
    };

    if let Some(t) = task {
        let is_done = t.completed;
        let msg_key = if is_done { "confirm_msg_undone" } else { "confirm_msg_done" };
        let msg = app.translate(msg_key).replace("{}", &t.title);
        
        frame.render_widget(Block::default().title(app.translate("confirm_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app.config.theme))), area);
        let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
        frame.render_widget(Paragraph::new(msg).alignment(Alignment::Center).wrap(ratatui::widgets::Wrap { trim: true }), chunks[1]);
        frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[2]);
    }
}

fn render_success_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 40, frame.size());
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::green(app.config.theme)));
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Length(3), Constraint::Length(2), Constraint::Length(2)]).margin(2).split(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(app.translate("auth_success_title")).alignment(Alignment::Center).style(Style::default().fg(Palette::green(app.config.theme)).add_modifier(Modifier::BOLD)), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("auth_success_msg")).alignment(Alignment::Center), chunks[2]);
    frame.render_widget(Paragraph::new(app.translate("auth_success_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::subtext0(app.config.theme)).add_modifier(Modifier::ITALIC)), chunks[3]);
}

fn render_auth_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 70, frame.size());
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app.config.theme)));
    frame.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(3), // Título
        Constraint::Min(0),    // Cuerpo
        Constraint::Length(3), // Botón/Status
        Constraint::Length(1), // Salir
    ]).margin(2).split(area);

    frame.render_widget(Paragraph::new(app.translate("welcome")).alignment(Alignment::Center).style(Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD)), chunks[0]);

    let content = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(5),
        Constraint::Min(0),
    ]).split(chunks[1]);

    frame.render_widget(Paragraph::new(app.translate("login_msg")).alignment(Alignment::Center), content[0]);

    if let Some(url) = &app.auth_url {
        let foot = if app.config.language == crate::app::Language::Spanish { "¡Link copiado al portapapeles!" } else { "Link copied to clipboard!" };
        frame.render_widget(Paragraph::new(app.translate("auth_instructions")).alignment(Alignment::Center).style(Style::default().fg(Palette::subtext0(app.config.theme))), content[1]);
        frame.render_widget(Paragraph::new(url.as_str()).alignment(Alignment::Center).wrap(ratatui::widgets::Wrap { trim: true }).style(Style::default().fg(Palette::blue(app.config.theme)).add_modifier(Modifier::UNDERLINED)).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)), content[2]);
        frame.render_widget(Paragraph::new(vec![Line::from(vec![Span::styled(foot, Style::default().fg(Palette::green(app.config.theme)))]), Line::from(""), Line::from(vec![Span::styled(app.translate("auth_waiting"), Style::default().fg(Palette::peach(app.config.theme)))])]).alignment(Alignment::Center), content[3]);
    } else { frame.render_widget(Paragraph::new(app.translate("login_btn")).alignment(Alignment::Center).style(Style::default().fg(Palette::green(app.config.theme)).add_modifier(Modifier::REVERSED)), content[2]); }
    frame.render_widget(Paragraph::new(format!("'Q' -> {}", app.translate("quit"))).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[2]);
}

fn render_list_selector(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 40, frame.size());
    frame.render_widget(Clear, area);
    let items: Vec<ListItem> = app.task_lists.iter().enumerate().map(|(i, l)| {
        let s = if i == app.selected_list_idx { Style::default().fg(Palette::base(app.config.theme)).bg(Palette::mauve(app.config.theme)) } else { Style::default().fg(Palette::text(app.config.theme)) };
        ListItem::new(format!("  {}  ", l.title)).style(s)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(app.translate("lists_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app.config.theme)))), area);
}

fn centered_rect(p_x: u16, p_y: u16, r: Rect) -> Rect {
    let v = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage((100 - p_y) / 2), Constraint::Percentage(p_y), Constraint::Percentage((100 - p_y) / 2)]).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage((100 - p_x) / 2), Constraint::Percentage(p_x), Constraint::Percentage((100 - p_x) / 2)]).split(v[1])[1]
}

fn render_animation_layer(app: &mut App, frame: &mut Frame) {
    if let Some(id) = &app.animation.task_id {
        let mut particles = app.animation.particles.clone();
        for p in &mut particles {
            p.x += p.vx; p.y += p.vy; p.life -= 0.05;
        }
        particles.retain(|p| p.life > 0.0);
        app.animation.particles = particles;
        app.animation.progress += 0.02;

        if app.animation.progress >= 1.0 { app.animation.task_id = None; }
        else {
            for p in &app.animation.particles {
                let (px, py) = (p.x as u16, p.y as u16);
                let style = Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD);
                if px < frame.size().width && py < frame.size().height {
                    frame.render_widget(Paragraph::new(p.char.to_string()).style(style), Rect { x: px as u16, y: py as u16, width: 1, height: 1 });
                }
            }
        }
    }
}
