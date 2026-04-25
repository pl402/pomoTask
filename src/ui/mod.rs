pub mod calendar;
pub mod list;
pub mod timer;
pub mod modals;
pub mod palette;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph,
    },
    Frame,
};
use chrono::Local;

use crate::app::{App, AppMode};
use crate::ui::palette::Palette;
use self::calendar::render_calendar;
use self::timer::{render_timer_mode, render_timer_screen};
use self::modals::{
    render_input_modal, render_confirm_modal, render_help_modal, 
    render_settings_modal, render_logout_confirm_modal, render_list_selector
};

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

pub fn render_right_panel(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(12), // Calendario
            Constraint::Min(0),     // Info Tarea
        ])
        .split(area);

    render_calendar(app, frame, chunks[0]);

    let mut info_lines = vec![];
    if let Some(task) = app.tasks.get(app.selected_task) {
        info_lines.push(Line::from(vec![Span::styled(task.title.clone(), Style::default().fg(Palette::mauve(app)).add_modifier(Modifier::BOLD))]));
        info_lines.push(Line::from(vec![Span::styled(format!("🍅 Pomodoros: {}", task.pomodoros), Style::default().fg(Palette::peach(app)))]));
        info_lines.push(Line::from(""));

        // CONVERSIÓN A TIEMPO LOCAL
        let due_str = task.due.map(|d| app.format_due_date(d)).unwrap_or_else(|| "---".to_string());
        info_lines.push(Line::from(vec![Span::styled(format!("{}: ", app.translate("due_date")), Style::default().fg(Palette::subtext0(app))), Span::raw(due_str)]));

        let created_str = task.updated.with_timezone(&Local).format("%Y-%m-%d %H:%M").to_string();
        info_lines.push(Line::from(vec![Span::styled(format!("{}: ", app.translate("created_date")), Style::default().fg(Palette::subtext0(app))), Span::raw(created_str)]));

        info_lines.push(Line::from(""));
        info_lines.push(Line::from(vec![Span::styled(format!("{}:", app.translate("notes")), Style::default().fg(Palette::subtext0(app)))]));
        let no_notes = app.translate("no_notes");
        let notes = task.notes.as_deref().unwrap_or(&no_notes);
        for line in notes.lines() { info_lines.push(Line::from(vec![Span::raw(format!("  {}", line))])); }
    } else { info_lines.push(Line::from(app.translate("no_task"))); }
    frame.render_widget(Paragraph::new(info_lines).wrap(ratatui::widgets::Wrap { trim: true }).block(Block::default().title(format!(" {} ", app.translate("info"))).borders(Borders::ALL).border_type(BorderType::Rounded)), chunks[1]);
}

fn render_loading_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(40, 20, frame.size());
    let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let spinner = frames[app.spinner_frame % frames.len()];
    let loading = Paragraph::new(format!("{} {}", spinner, app.translate("loading_app"))).alignment(Alignment::Center).style(Style::default().fg(Palette::mauve(app))).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded));
    frame.render_widget(loading, area);
}

fn render_success_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 40, frame.size());
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::green(app)));
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(1), Constraint::Length(3), Constraint::Length(2), Constraint::Length(2)]).margin(2).split(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(app.translate("auth_success_title")).alignment(Alignment::Center).style(Style::default().fg(Palette::green(app)).add_modifier(Modifier::BOLD)), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("auth_success_msg")).alignment(Alignment::Center), chunks[2]);
    frame.render_widget(Paragraph::new(app.translate("auth_success_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::subtext0(app)).add_modifier(Modifier::ITALIC)), chunks[3]);
}

fn render_auth_screen(app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 70, frame.size());
    let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app)));
    frame.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(3), // Título
        Constraint::Min(0),    // Cuerpo
        Constraint::Length(3), // Botón/Status
        Constraint::Length(1), // Salir
    ]).margin(2).split(area);

    frame.render_widget(Paragraph::new(app.translate("welcome")).alignment(Alignment::Center).style(Style::default().fg(Palette::mauve(app)).add_modifier(Modifier::BOLD)), chunks[0]);

    let content = Layout::default().direction(Direction::Vertical).constraints([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(5),
        Constraint::Min(0),
    ]).split(chunks[1]);

    frame.render_widget(Paragraph::new(app.translate("login_msg")).alignment(Alignment::Center), content[0]);

    if let Some(url) = &app.auth_url {
        let url_str: String = url.as_str().to_string();
        let foot = if app.config.language == crate::app::Language::Spanish { "¡Link copiado al portapapeles!" } else { "Link copied to clipboard!" };
        frame.render_widget(Paragraph::new(format!("{}:", app.translate("auth_instructions"))).alignment(Alignment::Center).style(Style::default().fg(Palette::subtext0(app))), content[1]);
        frame.render_widget(Paragraph::new(url_str.as_str()).alignment(Alignment::Center).wrap(ratatui::widgets::Wrap { trim: true }).style(Style::default().fg(Palette::blue(app)).add_modifier(Modifier::UNDERLINED)).block(Block::default().borders(Borders::ALL).border_type(BorderType::Rounded)), content[2]);
        frame.render_widget(Paragraph::new(vec![Line::from(vec![Span::styled(foot, Style::default().fg(Palette::green(app)))]), Line::from(""), Line::from(vec![Span::styled(app.translate("auth_waiting"), Style::default().fg(Palette::peach(app)))])]).alignment(Alignment::Center), content[3]);
    } else { frame.render_widget(Paragraph::new(app.translate("login_btn")).alignment(Alignment::Center).style(Style::default().fg(Palette::green(app)).add_modifier(Modifier::REVERSED)), content[2]); }
    frame.render_widget(Paragraph::new(format!("'Q' -> {}", app.translate("quit"))).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app))), chunks[2]);
}

pub fn centered_rect(p_x: u16, p_y: u16, r: Rect) -> Rect {
    let v = Layout::default().direction(Direction::Vertical).constraints([Constraint::Percentage((100 - p_y) / 2), Constraint::Percentage(p_y), Constraint::Percentage((100 - p_y) / 2)]).split(r);
    Layout::default().direction(Direction::Horizontal).constraints([Constraint::Percentage((100 - p_x) / 2), Constraint::Percentage(p_x), Constraint::Percentage((100 - p_x) / 2)]).split(v[1])[1]
}

fn render_animation_layer(app: &mut App, frame: &mut Frame) {
    if let Some(_id) = &app.animation.task_id {
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
                let style = Style::default().fg(Palette::mauve(app)).add_modifier(Modifier::BOLD);
                if px < frame.size().width && py < frame.size().height {
                    frame.render_widget(Paragraph::new(p.char.to_string()).style(style), Rect { x: px as u16, y: py as u16, width: 1, height: 1 });
                }
            }
        }
    }
}
