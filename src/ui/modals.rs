use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Modifier, Style},
    widgets::{
        Block, BorderType, Borders, Paragraph, Clear,
        Table, Row, Cell, List, ListItem,
    },
    Frame,
};

use crate::app::{App, Palette, AppMode, InputField, DatePreset};
use crate::ui::centered_rect;

pub fn render_input_modal(app: &App, frame: &mut Frame) {
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

pub fn render_help_modal(app: &App, frame: &mut Frame) {
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
        ("[ / ]", app.translate("calendar_nav")),
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

pub fn render_settings_modal(app: &App, frame: &mut Frame) {
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

pub fn render_logout_confirm_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 20, frame.size());
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().title(app.translate("logout_confirm_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::red(app.config.theme))), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
    frame.render_widget(Paragraph::new(app.translate("logout_confirm_msg")).alignment(Alignment::Center), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app.config.theme))), chunks[2]);
}

pub fn render_confirm_modal(app: &App, frame: &mut Frame) {
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

pub fn render_list_selector(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 40, frame.size());
    frame.render_widget(Clear, area);
    let items: Vec<ListItem> = app.task_lists.iter().enumerate().map(|(i, l)| {
        let s = if i == app.selected_list_idx { Style::default().fg(Palette::base(app.config.theme)).bg(Palette::mauve(app.config.theme)) } else { Style::default().fg(Palette::text(app.config.theme)) };
        ListItem::new(format!("  {}  ", l.title)).style(s)
    }).collect();
    frame.render_widget(List::new(items).block(Block::default().title(app.translate("lists_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app.config.theme)))), area);
}
