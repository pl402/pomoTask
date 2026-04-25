use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{
        Block, BorderType, Borders, Gauge, Table, Row, Cell,
    },
    Frame,
};

use crate::app::{App, Palette, TimerMode};

pub fn render_left_panel(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let total = app.timer_mode.duration(&app.config);
    let progress = ((total - app.timer_seconds) as f64 / total as f64).min(1.0);
    let label = format!("{} - {:02}:{:02}", match app.timer_mode { TimerMode::Focus => app.translate("focus"), _ => app.translate("break") }, app.timer_seconds / 60, app.timer_seconds % 60);
    frame.render_widget(Gauge::default().block(Block::default().title(format!(" {} ", app.translate("timer"))).borders(Borders::ALL).border_type(BorderType::Rounded)).gauge_style(Style::default().fg(if app.timer_mode == TimerMode::Focus { Palette::red(app.config.theme) } else { Palette::green(app.config.theme) }).bg(Palette::surface0(app.config.theme))).ratio(progress).label(label), chunks[0]);

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
    } else { format!(" {} ", app.translate("tasks")) };

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
