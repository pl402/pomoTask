use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Bar, BarChart, BarGroup, Block, BorderType, Borders, Paragraph},
    Frame,
};

use crate::app::App;
use crate::ui::palette::Palette;

pub fn render_stats_screen(app: &App, frame: &mut Frame) {
    let area = frame.size();
    let block = Block::default()
        .title(format!(" 📊 {} ", app.translate("stats_title")))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Palette::mauve(app)));
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),  // Subtítulo
            Constraint::Min(8),     // Gráfica de barras (pomodoros/día)
            Constraint::Length(4),  // Totales históricos
            Constraint::Length(1),  // Pie
        ])
        .margin(2)
        .split(area);

    frame.render_widget(
        Paragraph::new(app.translate("stats_last_7_days"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Palette::subtext0(app)).add_modifier(Modifier::ITALIC)),
        chunks[0],
    );

    // Dos gráficas de barras lado a lado: pomodoros/día y tareas completadas/día (últimos 7).
    let days = app.stats_last_n_days(7);
    let charts = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    let pomo_bars: Vec<Bar> = days.iter().map(|d| {
        Bar::default()
            .value(d.pomodoros)
            .label(Line::from(d.label.clone()))
            .text_value(format!("{}", d.pomodoros))
            .style(Style::default().fg(Palette::red(app)))
            .value_style(Style::default().fg(Palette::base(app)).bg(Palette::red(app)))
    }).collect();
    frame.render_widget(
        BarChart::default()
            .block(Block::default()
                .title(format!(" 🍅 {} ", app.translate("stats_pomodoros_per_day")))
                .borders(Borders::ALL).border_type(BorderType::Rounded))
            .data(BarGroup::default().bars(&pomo_bars))
            .bar_width(3).bar_gap(1),
        charts[0],
    );

    let task_bars: Vec<Bar> = days.iter().map(|d| {
        Bar::default()
            .value(d.tasks_done)
            .label(Line::from(d.label.clone()))
            .text_value(format!("{}", d.tasks_done))
            .style(Style::default().fg(Palette::green(app)))
            .value_style(Style::default().fg(Palette::base(app)).bg(Palette::green(app)))
    }).collect();
    frame.render_widget(
        BarChart::default()
            .block(Block::default()
                .title(format!(" ✅ {} ", app.translate("stats_tasks_per_day")))
                .borders(Borders::ALL).border_type(BorderType::Rounded))
            .data(BarGroup::default().bars(&task_bars))
            .bar_width(3).bar_gap(1),
        charts[1],
    );

    // Totales históricos.
    let (total_pomo, total_tasks, total_focus) = app.stats_totals();
    let focus_hours = total_focus / 3600;
    let focus_mins = (total_focus % 3600) / 60;
    let totals = vec![
        Line::from(vec![
            Span::styled(format!("🍅 {}: ", app.translate("stats_total_pomodoros")), Style::default().fg(Palette::subtext0(app))),
            Span::styled(format!("{}", total_pomo), Style::default().fg(Palette::peach(app)).add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled(format!("✅ {}: ", app.translate("stats_total_tasks")), Style::default().fg(Palette::subtext0(app))),
            Span::styled(format!("{}", total_tasks), Style::default().fg(Palette::green(app)).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(vec![
            Span::styled(format!("⏱️ {}: ", app.translate("stats_total_focus")), Style::default().fg(Palette::subtext0(app))),
            Span::styled(format!("{}h {}m", focus_hours, focus_mins), Style::default().fg(Palette::blue(app)).add_modifier(Modifier::BOLD)),
            Span::raw("    "),
            Span::styled(format!("📆 {}: ", app.translate("stats_focus_today")), Style::default().fg(Palette::subtext0(app))),
            Span::styled(
                {
                    let today_focus = days.last().map(|d| d.focus_seconds).unwrap_or(0);
                    format!("{}h {}m", today_focus / 3600, (today_focus % 3600) / 60)
                },
                Style::default().fg(Palette::yellow(app)).add_modifier(Modifier::BOLD),
            ),
        ]),
    ];
    frame.render_widget(
        Paragraph::new(totals).alignment(Alignment::Center).block(
            Block::default()
                .title(format!(" {} ", app.translate("stats_totals")))
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded),
        ),
        chunks[2],
    );

    frame.render_widget(
        Paragraph::new(app.translate("stats_close_hint"))
            .alignment(Alignment::Center)
            .style(Style::default().fg(Palette::overlay0(app))),
        chunks[3],
    );
}
