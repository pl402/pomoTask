use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style, Color},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Gauge, List, ListItem, Paragraph, Sparkline, Clear,
        Table, Row, Cell,
    },
    Frame,
};
use chrono::{Local, Timelike};

use crate::app::{App, Palette, TimerMode, AppMode, InputField};

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
    }
}

fn render_left_panel(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let total = app.timer_mode.duration(&app.config);
    let progress = ((total - app.timer_seconds) as f64 / total as f64).min(1.0);
    let label = format!("{} - {:02}:{:02}", match app.timer_mode { TimerMode::Focus => app.translate("focus"), _ => app.translate("break") }, app.timer_seconds / 60, app.timer_seconds % 60);
    frame.render_widget(Gauge::default().block(Block::default().title(app.translate("timer")).borders(Borders::ALL).border_type(BorderType::Rounded)).gauge_style(Style::default().fg(if app.timer_mode == TimerMode::Focus { Palette::RED } else { Palette::GREEN }).bg(Palette::SURFACE0)).ratio(progress).label(label), chunks[0]);

    let header_cells = [" Tarea ", " 🕒 Creada ", " 📅 Venc. ", " 🍅 "].into_iter().map(|h| Cell::from(h).style(Style::default().fg(Palette::MAUVE).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = app.tasks.iter().enumerate().map(|(i, task)| {
        let is_selected = i == app.selected_task;
        let style = if is_selected { Style::default().fg(Palette::BASE).bg(Palette::MAUVE) } 
                    else if task.completed { Style::default().fg(Palette::OVERLAY0) }
                    else { Style::default().fg(Palette::TEXT) };

        let is_subtask = if let Some(pid) = &task.parent_id { app.tasks.iter().any(|t| &t.id == pid) } else { false };
        let indent = if is_subtask { "  ↳ " } else { "" };
        
        let title_content = format!("{}{}{}", indent, if task.completed { "󰄲 " } else { "󰄱 " }, task.title);
        
        // CONVERSIÓN A TIEMPO LOCAL
        let created_str = task.updated.with_timezone(&Local).format("%d/%m").to_string();
        let due_str = task.due.map(|d| d.with_timezone(&Local).format("%d/%m").to_string()).unwrap_or_else(|| "---".to_string());
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
        .constraints([Constraint::Length(7), Constraint::Min(0)])
        .split(area);

    let spark_data = app.get_sparkline_data();
    frame.render_widget(Sparkline::default().block(Block::default().title(app.translate("productivity")).borders(Borders::ALL).border_type(BorderType::Rounded)).data(&spark_data).style(Style::default().fg(Palette::PEACH)), chunks[0]);
    
    let mut info_lines = vec![];
    if let Some(task) = app.tasks.get(app.selected_task) {
        info_lines.push(Line::from(vec![Span::styled(task.title.clone(), Style::default().fg(Palette::MAUVE).add_modifier(Modifier::BOLD))]));
        info_lines.push(Line::from(vec![Span::styled(format!("🍅 Pomodoros: {}", task.pomodoros), Style::default().fg(Palette::PEACH))]));
        info_lines.push(Line::from(""));
        
        // CONVERSIÓN A TIEMPO LOCAL
        let due_str = task.due.map(|d| d.with_timezone(&Local).format("%Y-%m-%d").to_string()).unwrap_or_else(|| "---".to_string());
        info_lines.push(Line::from(vec![Span::styled(format!("{}: ", app.translate("due_date")), Style::default().fg(Palette::SUBTEXT0)), Span::raw(due_str)]));
        
        let created_str = task.updated.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string();
        info_lines.push(Line::from(vec![Span::styled(format!("{}: ", app.translate("created_date")), Style::default().fg(Palette::SUBTEXT0)), Span::raw(created_str)]));
        
        info_lines.push(Line::from(""));
        info_lines.push(Line::from(vec![Span::styled(format!("{}:", app.translate("notes")), Style::default().fg(Palette::SUBTEXT0))]));
        let notes = task.notes.as_deref().unwrap_or(app.translate("no_notes"));
        for line in notes.lines() { info_lines.push(Line::from(vec![Span::raw(format!("  {}", line))])); }
    } else { info_lines.push(Line::from(app.translate("no_task"))); }
    frame.render_widget(Paragraph::new(info_lines).wrap(ratatui::widgets::Wrap { trim: true }).block(Block::default().title(app.translate("info")).borders(Borders::ALL).border_type(BorderType::Rounded)), chunks[1]);
}

fn render_focus_mode(app: &App, frame: &mut Frame) {
    let area = frame.size();
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::RED));
    frame.render_widget(block, area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage(30), Constraint::Length(5), Constraint::Percentage(10), Constraint::Min(0)]).margin(5).split(area);
    let total = app.timer_mode.duration(&app.config);
    let progress = ((total - app.timer_seconds) as f64 / total as f64).min(1.0);
    let time_label = format!("{:02}:{:02}", app.timer_seconds / 60, app.timer_seconds % 60);
    frame.render_widget(Gauge::default().block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)).gauge_style(Style::default().fg(Palette::RED).bg(Palette::SURFACE0)).ratio(progress).label(time_label), chunks[1]);
    if let Some(task) = app.tasks.get(app.selected_task) {
        let now = Local::now();
        let seed = task.id.chars().map(|c| c as u32).sum::<u32>() + now.minute();
        let msg_key = format!("focus_msg_{}", seed % 10);
        let focus_text = vec![Line::from(vec![Span::styled(app.translate(&msg_key), Style::default().fg(Palette::TEXT)), Span::styled(task.title.clone(), Style::default().fg(Palette::MAUVE).add_modifier(Modifier::BOLD))]), Line::from(""), Line::from(vec![Span::styled(format!("🍅 {} completados hoy en esta tarea", task.pomodoros), Style::default().fg(Palette::PEACH))])];
        frame.render_widget(Paragraph::new(focus_text).alignment(Alignment::Center), chunks[3]);
    }
}

fn render_loading_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(40, 20, frame.size());
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = frames[app.spinner_frame % frames.len()];
    let loading = Paragraph::new(format!("{} {}", spinner, app.translate("loading_app"))).alignment(Alignment::Center).style(Style::default().fg(Palette::MAUVE)).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    frame.render_widget(loading, area);
}

fn render_input_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 60, frame.size());
    frame.render_widget(Clear, area);
    let title_key = match app.mode { AppMode::SubtaskInput => "subtask_title", AppMode::Edit => "edit_title", _ => "input_title" };
    frame.render_widget(Block::default().title(app.translate(title_key)).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::YELLOW)), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Length(5), Constraint::Length(3), Constraint::Min(0)]).margin(2).split(area);
    let title_style = if app.focused_input == InputField::Title { Style::default().fg(Palette::YELLOW) } else { Style::default().fg(Palette::SUBTEXT0) };
    frame.render_widget(Paragraph::new(app.input_title.as_str()).block(Block::default().title(" Título ").borders(Borders::ALL).border_style(title_style)), chunks[0]);
    let notes_style = if app.focused_input == InputField::Notes { Style::default().fg(Palette::YELLOW) } else { Style::default().fg(Palette::SUBTEXT0) };
    frame.render_widget(Paragraph::new(app.input_notes.as_str()).block(Block::default().title(" Notas ").borders(Borders::ALL).border_style(notes_style)), chunks[1]);
    let due_style = if app.focused_input == InputField::Due { Style::default().fg(Palette::YELLOW) } else { Style::default().fg(Palette::SUBTEXT0) };
    frame.render_widget(Paragraph::new(app.input_due.as_str()).block(Block::default().title(format!(" {} {} ", app.translate("due_date"), app.translate("due_date_hint"))).borders(Borders::ALL).border_style(due_style)), chunks[2]);
    frame.render_widget(Paragraph::new(app.translate("input_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::OVERLAY0)), chunks[3]);
}

fn render_header(app: &App, frame: &mut Frame, area: Rect) {
    let header_chunks = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(40), Constraint::Percentage(60)]).split(area);
    let title = Paragraph::new(app.translate("title")).style(Style::default().fg(Palette::MAUVE).add_modifier(Modifier::BOLD)).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    frame.render_widget(title, header_chunks[0]);
    let now = Local::now();
    let time_str = now.format("%Y-%m-%d %H:%M:%S").to_string();
    let spinner = if app.loading { let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]; frames[app.spinner_frame % frames.len()] } else { "✓" };
    let status = Paragraph::new(format!("{} | {}: {} | {}: {}", time_str, app.translate("sync"), spinner, app.translate("pomodoro_label"), app.session_pomodoros)).alignment(Alignment::Right).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    frame.render_widget(status, header_chunks[1]);
}

fn render_timer_screen(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)]).split(frame.size());
    render_header(app, frame, chunks[0]);
    let content = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(chunks[1]);
    render_left_panel(app, frame, content[0]);
    render_right_panel(app, frame, content[1]);
    render_footer(app, frame, chunks[2]);
}

fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let keys = vec![("SPACE", app.translate("start_pause")), ("ENTER", app.translate("complete_task_hotkey")), ("N", app.translate("new_task")), ("A", app.translate("new_subtask")), ("E", app.translate("edit_task")), ("C", app.translate("toggle_completed")), ("Tab", app.translate("change_list")), ("L", app.translate("lang")), ("Q", app.translate("quit"))];
    let spans: Vec<Span> = keys.iter().flat_map(|(k, v)| vec![Span::styled(format!(" {} ", k), Style::default().fg(Palette::MAUVE).add_modifier(Modifier::REVERSED)), Span::styled(format!(" {}  ", v), Style::default().fg(Palette::TEXT))]).collect();
    frame.render_widget(Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)), area);
}

fn render_auth_screen(app: &App, frame: &mut Frame) {
    let area = frame.size();
    frame.render_widget(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).title(app.translate("title")), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(3)]).margin(2).split(area);
    frame.render_widget(Paragraph::new(app.translate("welcome")).alignment(Alignment::Center).style(Style::default().fg(Palette::MAUVE).add_modifier(Modifier::BOLD)), chunks[0]);
    let content = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Length(2), Constraint::Length(4), Constraint::Min(0)]).split(chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("login_msg")).alignment(Alignment::Center), content[0]);
    if let Some(url) = &app.auth_url {
        frame.render_widget(Paragraph::new(app.translate("auth_instructions")).alignment(Alignment::Center).style(Style::default().fg(Palette::SUBTEXT0)), content[1]);
        frame.render_widget(Paragraph::new(url.as_str()).alignment(Alignment::Center).wrap(ratatui::widgets::Wrap { trim: true }).style(Style::default().fg(Palette::BLUE).add_modifier(Modifier::UNDERLINED)).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)), content[2]);
        let foot = if app.config.language == crate::app::Language::Spanish { "🌐 Navegador abierto | 📋 Copiado" } else { "🌐 Browser opened | 📋 Copied" };
        frame.render_widget(Paragraph::new(vec![Line::from(vec![Span::styled(foot, Style::default().fg(Palette::GREEN))]), Line::from(""), Line::from(vec![Span::styled(app.translate("auth_waiting"), Style::default().fg(Palette::PEACH))])]).alignment(Alignment::Center), content[3]);
    } else { frame.render_widget(Paragraph::new(app.translate("login_btn")).alignment(Alignment::Center).style(Style::default().fg(Palette::GREEN).add_modifier(Modifier::REVERSED)), content[2]); }
    frame.render_widget(Paragraph::new(format!("'Q' -> {}", app.translate("quit"))).alignment(Alignment::Center).style(Style::default().fg(Palette::OVERLAY0)), chunks[2]);
}

fn render_list_selector(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 40, frame.size());
    frame.render_widget(Clear, area);
    let items: Vec<ListItem> = app.task_lists.iter().enumerate().map(|(i, l)| {
        let s = if i == app.selected_list_idx { Style::default().fg(Palette::BASE).bg(Palette::MAUVE) } else { Style::default().fg(Palette::TEXT) };
        ListItem::new(format!("  {}  ", l.title)).style(s)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(app.translate("lists_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::MAUVE))), area);
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
    frame.render_widget(Block::default().title(app.translate("confirm_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::MAUVE)), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
    frame.render_widget(Paragraph::new(app.translate(msg_key)).alignment(Alignment::Center), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::OVERLAY0)), chunks[2]);
}

fn render_success_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 40, frame.size());
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::GREEN));
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Length(3), Constraint::Length(2), Constraint::Length(2)]).margin(2).split(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(app.translate("auth_success_title")).alignment(Alignment::Center).style(Style::default().fg(Palette::GREEN).add_modifier(Modifier::BOLD)), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("auth_success_msg")).alignment(Alignment::Center), chunks[2]);
    frame.render_widget(Paragraph::new(app.translate("auth_success_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::SUBTEXT0).add_modifier(Modifier::ITALIC)), chunks[3]);
}
