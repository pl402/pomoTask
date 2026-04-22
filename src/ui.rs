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
use chrono::{Local, Timelike};

use crate::app::{App, Palette, TimerMode, AppMode, InputField, DatePreset};

pub fn render(app: &mut App, frame: &mut Frame) {
    match app.mode {
        AppMode::Loading => render_loading_screen(app, frame),
        AppMode::Timer => {
            if app.timer_active && app.timer_mode == TimerMode::Focus && !app.tasks.is_empty() {
                render_focus_mode(app, frame);
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
            render_timer_screen(app, frame);
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
    } else { app.translate("tasks").to_string() };

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
        let notes = task.notes.as_deref().unwrap_or(app.translate("no_notes"));
        for line in notes.lines() { info_lines.push(Line::from(vec![Span::raw(format!("  {}", line))])); }
    } else { info_lines.push(Line::from(app.translate("no_task"))); }
    frame.render_widget(Paragraph::new(info_lines).wrap(ratatui::widgets::Wrap { trim: true }).block(Block::default().title(app.translate("info")).borders(Borders::ALL).border_type(BorderType::Rounded)), chunks[2]);
}

fn render_focus_mode(app: &App, frame: &mut Frame) {
    let area = frame.size();
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::red(app.config.theme)));
    frame.render_widget(block, area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(30), Constraint::Length(5), Constraint::Percentage(10), Constraint::Min(0)]).margin(5).split(area);
    let total = app.timer_mode.duration(&app.config);
    let progress = ((total - app.timer_seconds) as f64 / total as f64).min(1.0);
    let time_label = format!("{:02}:{:02}", app.timer_seconds / 60, app.timer_seconds % 60);
    frame.render_widget(Gauge::default().block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)).gauge_style(Style::default().fg(Palette::red(app.config.theme)).bg(Palette::surface0(app.config.theme))).ratio(progress).label(time_label), chunks[1]);
    if let Some(task) = app.tasks.get(app.selected_task) {
        let now = Local::now();
        let seed = task.id.chars().map(|c| c as u32).sum::<u32>() + now.minute();
        let msg_key = format!("focus_msg_{}", seed % 10);
        let focus_text = vec![Line::from(vec![Span::styled(app.translate(&msg_key), Style::default().fg(Palette::text(app.config.theme))), Span::styled(task.title.clone(), Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD))]), Line::from(""), Line::from(vec![Span::styled(format!("🍅 {} completados hoy en esta tarea", task.pomodoros), Style::default().fg(Palette::peach(app.config.theme)))])];
        frame.render_widget(Paragraph::new(focus_text).alignment(Alignment::Center), chunks[3]);
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
    frame.render_widget(Block::default().title(app.translate(title_key)).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::yellow(app.config.theme))), area);
    
    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(3), // Título
        Constraint::Length(5), // Notas
        Constraint::Length(3), // Presets de Fecha
        Constraint::Length(3), // Input de Fecha
        Constraint::Min(0)     // Hint
    ]).margin(2).split(area);

    let title_style = if app.focused_input == InputField::Title { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::subtext0(app.config.theme)) };
    frame.render_widget(Paragraph::new(app.input_title.as_str()).block(Block::default().title(" Título ").borders(Borders::ALL).border_style(title_style)), chunks[0]);
    
    let notes_style = if app.focused_input == InputField::Notes { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::subtext0(app.config.theme)) };
    frame.render_widget(Paragraph::new(app.input_notes.as_str()).block(Block::default().title(" Notas ").borders(Borders::ALL).border_style(notes_style)), chunks[1]);

    // Botones de Fecha
    let preset_chunks = Layout::default().direction(Direction::Horizontal).constraints([
        Constraint::Percentage(33),
        Constraint::Percentage(33),
        Constraint::Percentage(33),
    ]).split(chunks[2]);

    let presets = [
        (DatePreset::Today, app.translate("date_today")),
        (DatePreset::Tomorrow, app.translate("date_tomorrow")),
        (DatePreset::Custom, app.translate("date_custom")),
    ];

    for (idx, (preset, label)) in presets.iter().enumerate() {
        let is_selected = app.selected_date_preset == *preset;
        let mut style = Style::default().fg(Palette::text(app.config.theme));
        if is_selected {
            style = Style::default().fg(Palette::base(app.config.theme)).bg(Palette::yellow(app.config.theme));
        }
        
        let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(if is_selected && app.focused_input == InputField::Due { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::surface0(app.config.theme)) });
        frame.render_widget(Paragraph::new(*label).alignment(Alignment::Center).style(style).block(block), preset_chunks[idx]);
    }

    let due_style = if app.focused_input == InputField::Due { Style::default().fg(Palette::yellow(app.config.theme)) } else { Style::default().fg(Palette::subtext0(app.config.theme)) };
    let date_input_text = if app.selected_date_preset == DatePreset::Custom { app.input_due.as_str() } else { app.input_due.as_str() }; // Mostrar siempre para feedback
    
    frame.render_widget(Paragraph::new(date_input_text).block(Block::default().title(format!(" {} {} ", app.translate("due_date"), app.translate("due_date_hint"))).borders(Borders::ALL).border_style(due_style)), chunks[3]);
    
    let input_hint = if app.focused_input == InputField::Due {
        if app.config.language == crate::app::Language::Spanish { " ←→: Cambiar Fecha | ENTER: Guardar | ESC: Salir " } else { " ←→: Change Date | ENTER: Save | ESC: Exit " }
    } else {
        app.translate("input_hint")
    };
    frame.render_widget(Paragraph::new(input_hint).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[4]);
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
        ("N", app.translate("new_task")),
        ("A", app.translate("new_subtask")),
        ("E", app.translate("edit_task")),
        ("C", app.translate("toggle_completed")),
        ("Tab", app.translate("change_list")),
        ("L", app.translate("lang")),
        ("S", app.translate("sync_manual")),
        (",", app.translate("settings_title")),
        ("Q", app.translate("quit")),
        ("?", app.translate("help_label")),
    ];

    let rows: Vec<Row> = keys.iter().map(|(k, v)| {
        Row::new(vec![
            Cell::from(Span::styled(format!(" {} ", k), Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD))),
            Cell::from(Span::raw(v.to_string())),
        ]).height(1)
    }).collect();

    let table = Table::new(rows, [Constraint::Percentage(30), Constraint::Percentage(70)])
        .block(block)
        .header(Row::new(vec![Cell::from(" Tecla "), Cell::from(" Acción ")]).style(Style::default().fg(Palette::subtext0(app.config.theme))));

    frame.render_widget(table, area);
    
    let hint_area = Rect { x: area.x, y: area.y + area.height - 2, width: area.width, height: 1 };
    frame.render_widget(Paragraph::new(app.translate("help_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), hint_area);
}

fn render_auth_screen(app: &App, frame: &mut Frame) {
    let area = frame.size();
    frame.render_widget(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(app.translate("title")), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)]).margin(2).split(area);
    frame.render_widget(Paragraph::new(app.translate("welcome")).alignment(Alignment::Center).style(Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD)), chunks[0]);
    let content = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Length(4), Constraint::Min(0)]).split(chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("login_msg")).alignment(Alignment::Center), content[0]);
    if let Some(url) = &app.auth_url {
        frame.render_widget(Paragraph::new(app.translate("auth_instructions")).alignment(Alignment::Center).style(Style::default().fg(Palette::subtext0(app.config.theme))), content[1]);
        frame.render_widget(Paragraph::new(url.as_str()).alignment(Alignment::Center).wrap(ratatui::widgets::Wrap { trim: true }).style(Style::default().fg(Palette::blue(app.config.theme)).add_modifier(Modifier::UNDERLINED)).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)), content[2]);
        let foot = if app.config.language == crate::app::Language::Spanish { "🌐 Navegador abierto | 📋 Copiado" } else { "🌐 Browser opened | 📋 Copied" };
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

fn render_confirm_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 20, frame.size());
    frame.render_widget(Clear, area);
    let is_done = app.tasks.get(app.selected_task).map(|t| t.completed).unwrap_or(false);
    let msg_key = if is_done { "confirm_msg_undone" } else { "confirm_msg_done" };
    frame.render_widget(Block::default().title(app.translate("confirm_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app.config.theme))), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
    frame.render_widget(Paragraph::new(app.translate(msg_key)).alignment(Alignment::Center), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[2]);
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

fn render_settings_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 50, frame.size());
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(app.translate("settings_title"))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Palette::mauve(app.config.theme)));
    
    let settings = vec![
        (app.translate("settings_focus"), format!("{} min", app.config.focus_duration / 60)),
        (app.translate("settings_short"), format!("{} min", app.config.short_break_duration / 60)),
        (app.translate("settings_long"), format!("{} min", app.config.long_break_duration / 60)),
        (app.translate("settings_lang"), format!("{:?}", app.config.language)),
        (app.translate("settings_theme"), format!("{:?}", app.config.theme)),
        (app.translate("settings_logout"), "".to_string()),
    ];

    let items: Vec<ListItem> = settings.iter().enumerate().map(|(i, (label, value))| {
        let style = if i == app.selected_settings_idx {
            Style::default().fg(Palette::base(app.config.theme)).bg(Palette::mauve(app.config.theme))
        } else {
            Style::default().fg(Palette::text(app.config.theme))
        };
        
        let content = if value.is_empty() {
            label.to_string()
        } else {
            format!("{}: {}", label, value)
        };
        
        ListItem::new(format!("  {}  ", content)).style(style)
    }).collect();

    let list = List::new(items)
        .block(block);

    frame.render_widget(list, area);
    
    let hint_area = Rect { x: area.x, y: area.y + area.height - 2, width: area.width, height: 1 };
    frame.render_widget(
        Paragraph::new(app.translate("settings_hint"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Palette::overlay0(app.config.theme))), 
        hint_area
    );
}

fn render_logout_confirm_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 20, frame.size());
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().title(app.translate("logout_confirm_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::red(app.config.theme))), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
    frame.render_widget(Paragraph::new(app.translate("logout_confirm_msg")).alignment(Alignment::Center), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[2]);
}

fn render_animation_layer(app: &App, frame: &mut Frame) {
    if app.animation.task_id.is_none() { return; }

    let area = frame.size();
    let spawn_x = app.animation.spawn_x as f64;
    let spawn_y = app.animation.spawn_y as f64;

    for p in &app.animation.particles {
        let px = spawn_x + (p.x * 2.0); // Factor escala horizontal
        let py = spawn_y + (p.y * 1.0); // Factor escala vertical

        if px >= 0.0 && px < area.width as f64 && py >= 0.0 && py < area.height as f64 {
            let style = Style::default().fg(match p.char {
                '🍅' | '🚀' => Palette::red(app.config.theme),
                '✨' | '💎' => Palette::yellow(app.config.theme),
                '🎉' | '🌈' => Palette::mauve(app.config.theme),
                '⭐' => Palette::peach(app.config.theme),
                _ => Palette::blue(app.config.theme),
            });
            frame.render_widget(Paragraph::new(p.char.to_string()).style(style), Rect { x: px as u16, y: py as u16, width: 1, height: 1 });
        }
    }
}
