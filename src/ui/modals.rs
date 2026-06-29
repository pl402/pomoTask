use ratatui::{
    layout::{Constraint, Direction, Layout, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Paragraph, Clear,
        Table, Row, Cell, List, ListItem,
    },
    Frame,
};

use crate::app::{App, AppMode, InputField, DatePreset};
use crate::ui::palette::Palette;
use crate::ui::centered_rect;

pub fn render_input_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(70, 70, frame.size());
    frame.render_widget(Clear, area);
    let title_key = match app.mode { AppMode::SubtaskInput => "subtask_title", AppMode::Edit => "edit_title", _ => "input_title" };
    let mut modal_title = format!(" {} ", app.translate(title_key));
    if app.mode == AppMode::SubtaskInput {
        if let Some(parent) = app.tasks.get(app.selected_task) {
            modal_title = format!(" {} para {} ", app.translate("new_subtask"), parent.title);
        }
    }
    frame.render_widget(Block::default().title(modal_title).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::yellow(app))), area);

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

    // Caret visible solo en el campo enfocado.
    let caret = |focused: bool, text: &str| if focused { format!("{}▏", text) } else { text.to_string() };
    // Título del bloque con indicador ▸ cuando el campo está enfocado.
    let ftitle = |focused: bool, key: &str| if focused { format!(" ▸ {} ", app.translate(key)) } else { format!(" {} ", app.translate(key)) };

    let is_title = app.focused_input == InputField::Title;
    let title_style = if is_title { Style::default().fg(Palette::yellow(app)) } else { Style::default().fg(Palette::subtext0(app)) };
    let title_text = caret(is_title, &app.input_title);
    let title_width = chunks[0].width.max(3) - 2;
    let title_scroll = (title_text.chars().count() as u16).saturating_sub(title_width);
    frame.render_widget(Paragraph::new(title_text).scroll((0, title_scroll)).block(Block::default().title(ftitle(is_title, "field_title")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(title_style)), chunks[0]);

    let is_notes = app.focused_input == InputField::Notes;
    let notes_style = if is_notes { Style::default().fg(Palette::yellow(app)) } else { Style::default().fg(Palette::subtext0(app)) };
    let notes_text = caret(is_notes, &app.input_notes);
    let notes_width = chunks[1].width.max(3) - 2;
    let notes_scroll = (notes_text.chars().count() as u16).saturating_sub(notes_width);
    frame.render_widget(Paragraph::new(notes_text).scroll((0, notes_scroll)).block(Block::default().title(ftitle(is_notes, "field_notes")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(notes_style)), chunks[1]);

    let mut next_idx = 2;
    if app.mode == AppMode::Input {
        let is_list = app.focused_input == InputField::List;
        let list_style = if is_list { Style::default().fg(Palette::yellow(app)) } else { Style::default().fg(Palette::subtext0(app)) };
        let list_name = app.task_lists.get(app.input_list_idx).map(|l| l.title.as_str()).unwrap_or("---");
        let list_text = format!(" ← {} → ", list_name);
        frame.render_widget(Paragraph::new(list_text).alignment(Alignment::Center).block(Block::default().title(ftitle(is_list, "list_selection")).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(list_style)), chunks[next_idx]);
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

    let presets: Vec<(DatePreset, String)> = vec![
        (DatePreset::Today, app.translate("date_today")),
        (DatePreset::Tomorrow, app.translate("date_tomorrow")),
        (DatePreset::Custom, app.translate("date_custom")),
        (DatePreset::None, app.translate("date_none")),
    ];


    for (idx, (preset, label)) in presets.iter().enumerate() {
        let is_selected = app.selected_date_preset == *preset;
        let mut style = Style::default().fg(Palette::text(app));
        if is_selected {
            style = Style::default().fg(Palette::base(app)).bg(Palette::yellow(app));
        }
        let block = Block::default().borders(Borders::ALL).border_type(BorderType::Rounded).border_style(if is_selected && app.focused_input == InputField::Due { Style::default().fg(Palette::yellow(app)) } else { Style::default().fg(Palette::surface0(app)) });
        frame.render_widget(Paragraph::new(label.as_str()).alignment(Alignment::Center).style(style).block(block), preset_chunks[idx]);
    }

    let is_due = app.focused_input == InputField::Due;
    let due_style = if is_due { Style::default().fg(Palette::yellow(app)) } else { Style::default().fg(Palette::subtext0(app)) };
    let date_input_text = caret(is_due, &app.input_due);
    let due_width = chunks[next_idx].width.max(3) - 2;
    let due_scroll = (date_input_text.chars().count() as u16).saturating_sub(due_width);

    // Título de fecha: ▸ cuando enfocado + resolución de la fecha escrita (✓ legible / ✗ inválida).
    let prefix = if is_due { " ▸ " } else { " " };
    let mut due_title = vec![Span::raw(format!("{}{} ", prefix, app.translate("due_date")))];
    if app.input_due.trim().is_empty() {
        due_title.push(Span::styled(format!("{} ", app.translate("due_date_hint")), Style::default().fg(Palette::overlay0(app))));
    } else if let Some(dt) = App::parse_due_date(&app.input_due) {
        due_title.push(Span::styled(format!("✓ {} ", app.format_due_date(dt)), Style::default().fg(Palette::green(app))));
    } else {
        due_title.push(Span::styled(format!("✗ {} ", app.translate("date_invalid")), Style::default().fg(Palette::red(app))));
    }
    frame.render_widget(Paragraph::new(date_input_text).scroll((0, due_scroll)).block(Block::default().title(Line::from(due_title)).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(due_style)), chunks[next_idx]);
    next_idx += 1;
    
    let input_hint = if app.focused_input == InputField::Due {
        if app.config.language == crate::app::Language::Spanish { " ←→: Cambiar Fecha | ENTER: Guardar | ESC: Salir ".to_string() } else { " ←→: Change Date | ENTER: Save | ESC: Exit ".to_string() }
    } else if app.focused_input == InputField::List {
        if app.config.language == crate::app::Language::Spanish { " ←→: Cambiar Lista | ENTER: Guardar | ESC: Salir ".to_string() } else { " ←→: Change List | ENTER: Save | ESC: Exit ".to_string() }
    } else {
        app.translate("input_hint")
    };
    frame.render_widget(Paragraph::new(input_hint).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app))), chunks[next_idx]);
}

pub fn render_help_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 70, frame.size());
    frame.render_widget(Clear, area);
    
    let block = Block::default()
        .title(format!(" {} ", app.translate("help_title")))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Palette::mauve(app)));
    
    let keys = vec![
        ("SPACE", app.translate("start_pause")),
        ("m", app.translate("switch_mode")),
        ("ENTER", app.translate("complete_task_hotkey")),
        ("j/k", app.translate("j_k_navigate_timer")),
        ("h/l", app.translate("h_l_change_list")),
        ("Tab", app.translate("change_list")),
        ("[ / ]", app.translate("calendar_nav")),
        ("N", app.translate("new_task")),
        ("A", app.translate("new_subtask")),
        ("E", app.translate("edit_task")),
        ("y", app.translate("copy_task_label")),
        ("M", app.translate("move_task_label")),
        ("C", app.translate("toggle_completed")),
        ("S", app.translate("sync_manual")),
        ("/", app.translate("search_label")),
        ("T", app.translate("stats_title")),
        (",", app.translate("settings_title")),
        ("Q", app.translate("quit")),
        ("?", app.translate("help_label")),
    ];

    let rows: Vec<Row> = keys.iter().map(|(k, v)| {
        Row::new(vec![
            Cell::from(*k).style(Style::default().fg(Palette::yellow(app)).add_modifier(Modifier::BOLD)),
            Cell::from(v.clone()).style(Style::default().fg(Palette::text(app))),
        ])
    }).collect();

    let table = Table::new(rows, [Constraint::Percentage(30), Constraint::Percentage(70)]).block(block);
    frame.render_widget(table, area);
}

pub fn render_settings_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(62, 80, frame.size());
    frame.render_widget(Clear, area);

    let block = Block::default().title(format!(" {} ", app.translate("settings_title"))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::yellow(app)));
    frame.render_widget(block, area);

    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Min(0), Constraint::Length(1)]).margin(2).split(area);

    // (icono + etiqueta, valor) por índice lógico 0..8 (debe coincidir con el handler de teclado).
    let items: [(String, String); 9] = [
        (format!("⏱  {}", app.translate("settings_focus")), format!("{} min", app.config.focus_duration / 60)),
        (format!("☕  {}", app.translate("settings_short")), format!("{} min", app.config.short_break_duration / 60)),
        (format!("🌙  {}", app.translate("settings_long")), format!("{} min", app.config.long_break_duration / 60)),
        (format!("🌐  {}", app.translate("settings_lang")), match app.config.language { crate::app::Language::Spanish => "Español".to_string(), crate::app::Language::English => "English".to_string() }),
        (format!("🎨  {}", app.translate("settings_theme")), app.config.theme.name().to_string()),
        (format!("📅  {}", app.translate("settings_calendar_view")), match app.config.calendar_view {
            crate::app::CalendarView::Standard => app.translate("calendar_view_standard"),
            crate::app::CalendarView::Heatmap => app.translate("calendar_view_heatmap"),
            crate::app::CalendarView::Progress => app.translate("calendar_view_progress"),
        }),
        (format!("🔭  {}", app.translate("settings_calendar_range")), match app.config.calendar_range {
            crate::app::CalendarRange::Month => app.translate("calendar_range_month"),
            crate::app::CalendarRange::Week => app.translate("calendar_range_week"),
            crate::app::CalendarRange::Day => app.translate("calendar_range_day"),
        }),
        (format!("🧹  {}", app.translate("settings_retention")), match app.config.stats_retention {
            crate::app::StatsRetention::Month => app.translate("retention_month"),
            crate::app::StatsRetention::Year => app.translate("retention_year"),
            crate::app::StatsRetention::Forever => app.translate("retention_forever"),
        }),
        (format!("🚪  {}", app.translate("settings_logout")), String::new()),
    ];

    // Secciones (nombre, rango de índices).
    let groups: [(String, std::ops::Range<usize>); 4] = [
        (app.translate("settings_group_times"), 0..3),
        (app.translate("settings_group_appearance"), 3..7),
        (app.translate("settings_group_data"), 7..8),
        (app.translate("settings_group_account"), 8..9),
    ];

    let mut rows: Vec<Row> = Vec::new();
    for (gname, range) in groups.iter() {
        rows.push(Row::new(vec![
            Cell::from(format!("─ {} ─", gname)).style(Style::default().fg(Palette::overlay0(app)).add_modifier(Modifier::BOLD)),
            Cell::from(""),
        ]));
        for i in range.clone() {
            let (label, value) = &items[i];
            let selected = i == app.selected_settings_idx;
            // Afordancia ◄ ► en la fila seleccionada que tiene valor ajustable.
            let value_disp = if selected && !value.is_empty() { format!("◄ {} ►", value) } else { value.clone() };
            let style = if selected { Style::default().fg(Palette::base(app)).bg(Palette::yellow(app)) } else { Style::default().fg(Palette::text(app)) };
            rows.push(Row::new(vec![Cell::from(label.clone()), Cell::from(value_disp)]).style(style));
        }
    }

    let table = Table::new(rows, [Constraint::Percentage(60), Constraint::Percentage(40)]);
    frame.render_widget(table, chunks[0]);

    frame.render_widget(Paragraph::new(app.translate("settings_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app))), chunks[1]);
}

pub fn render_logout_confirm_modal(app: &App, frame: &mut Frame) {
    let area = centered_rect(50, 20, frame.size());
    frame.render_widget(Clear, area);
    frame.render_widget(Block::default().title(format!(" {} ", app.translate("logout_confirm_title"))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::red(app))), area);
    let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
    frame.render_widget(Paragraph::new(app.translate("logout_confirm_msg")).alignment(Alignment::Center), chunks[1]);
    frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app))), chunks[2]);
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
        
        frame.render_widget(Block::default().title(format!(" {} ", app.translate("confirm_title"))).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app))), area);
        let chunks = Layout::default().direction(Direction::Vertical).constraints([Constraint::Length(2), Constraint::Min(0), Constraint::Length(1)]).margin(1).split(area);
        frame.render_widget(Paragraph::new(msg).alignment(Alignment::Center).wrap(ratatui::widgets::Wrap { trim: true }), chunks[1]);
        frame.render_widget(Paragraph::new(app.translate("confirm_hint")).alignment(Alignment::Center).style(Style::default().fg(Palette::overlay0(app))), chunks[2]);
    }
}

pub fn render_list_selector(app: &App, frame: &mut Frame) {
    let area = centered_rect(60, 40, frame.size());
    frame.render_widget(Clear, area);
    let items: Vec<ListItem> = app.task_lists.iter().enumerate().map(|(i, l)| {
        let s = if i == app.selected_list_idx { Style::default().fg(Palette::base(app)).bg(Palette::mauve(app)) } else { Style::default().fg(Palette::text(app)) };
        ListItem::new(format!("  {}  ", l.title)).style(s)
    }).collect();
    // Cuando estamos moviendo una tarea, el selector elige la lista DESTINO.
    let title = if app.moving_task_id.is_some() {
        format!(" {} ", app.translate("move_to_title"))
    } else {
        format!(" {} ", app.translate("lists_title"))
    };
    frame.render_widget(List::new(items).block(Block::default().title(title).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app)))), area);
}
