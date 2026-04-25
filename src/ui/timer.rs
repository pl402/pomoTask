use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Gauge, Paragraph, ListItem, List
    },
    Frame,
};
use chrono::{Local, Timelike, Utc};

use crate::app::{App, Palette, TimerMode};
use crate::ui::list::render_left_panel;
use crate::ui::render_right_panel;

pub fn render_timer_screen(app: &mut App, frame: &mut Frame) {
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Min(0), Constraint::Length(1)]).split(frame.size());
    let content = Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage(60), Constraint::Percentage(40)]).split(chunks[0]);
    render_left_panel(app, frame, content[0]);
    render_right_panel(app, frame, content[1]);
    render_footer(app, frame, chunks[1]);
}

pub fn render_timer_mode(app: &App, frame: &mut Frame) {
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
                Span::styled(format!("{}: ", msg), Style::default().fg(Palette::text(app.config.theme))),
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

pub fn render_big_clock(app: &App, frame: &mut Frame, area: Rect, color: ratatui::style::Color) {
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

pub fn render_footer(app: &App, frame: &mut Frame, area: Rect) {
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H:%M").to_string();
    let spinner = if app.loading { let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]; frames[app.spinner_frame % frames.len()] } else { "✓" };
    
    let timer_label = format!("{:02}:{:02}", app.timer_seconds / 60, app.timer_seconds % 60);

    let left_spans = vec![
        Span::styled(format!(" {} ", app.translate("title")), Style::default().fg(Palette::mauve(app.config.theme)).add_modifier(Modifier::BOLD)),
        Span::styled(format!(" {} ", app.translate("footer_hint")), Style::default().fg(Palette::yellow(app.config.theme))),
    ];

    let right_spans = vec![
        Span::styled(format!(" {}: ", app.translate("timer_short")), Style::default().fg(Palette::subtext0(app.config.theme))),
        Span::styled(format!("{} ", timer_label), Style::default().fg(if app.timer_mode == TimerMode::Focus { Palette::red(app.config.theme) } else { Palette::green(app.config.theme) })),
        Span::styled(format!(" {}: ", app.translate("sync_short")), Style::default().fg(Palette::subtext0(app.config.theme))),
        Span::styled(format!("{} ", spinner), Style::default().fg(Palette::blue(app.config.theme))),
        Span::styled(format!(" {}: ", app.translate("pomodoro_short")), Style::default().fg(Palette::subtext0(app.config.theme))),
        Span::styled(format!("{} ", app.session_pomodoros), Style::default().fg(Palette::peach(app.config.theme))),
        Span::styled(format!(" {} | {} ", date_str, time_str), Style::default().fg(Palette::text(app.config.theme))),
    ];

    let footer_line = Line::from(left_spans);
    frame.render_widget(Paragraph::new(footer_line).alignment(Alignment::Left).style(Style::default().bg(Palette::surface0(app.config.theme))), area);
    
    let status_line = Line::from(right_spans);
    frame.render_widget(Paragraph::new(status_line).alignment(Alignment::Right), area);
}
