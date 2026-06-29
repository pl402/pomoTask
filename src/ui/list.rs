use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    widgets::{
        Block, BorderType, Borders, Gauge, Table, Row, Cell, Paragraph, TableState,
    },
    Frame,
};

use chrono::Local;

use crate::app::{App, TimerMode};
use crate::ui::palette::Palette;

pub fn render_left_panel(app: &App, frame: &mut Frame, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let total = app.timer_mode.duration(&app.config);
    let progress = ((total - app.timer_seconds) as f64 / total as f64).min(1.0);
    let remaining_ratio = app.timer_seconds as f64 / total as f64;
    // Enfoque: color dinámico verde→amarillo→rojo según el tiempo restante. Descanso: verde.
    let gauge_fg = if app.timer_mode == TimerMode::Focus { Palette::timer_color(app, remaining_ratio) } else { Palette::green(app) };
    let label = format!("{} - {:02}:{:02}", match app.timer_mode { TimerMode::Focus => app.translate("focus"), _ => app.translate("break") }, app.timer_seconds / 60, app.timer_seconds % 60);
    frame.render_widget(Gauge::default().block(Block::default().title(format!(" {} ", app.translate("timer"))).borders(Borders::ALL).border_type(BorderType::Rounded)).gauge_style(Style::default().fg(gauge_fg).bg(Palette::surface0(app))).ratio(progress).label(label), chunks[0]);

    // En la vista "Todas" (@all) añadimos una columna que indica la lista de origen de cada tarea.
    let is_all_view = app.is_all_view();

    let header_titles: Vec<String> = if is_all_view {
        vec![format!(" {} ", app.translate("col_task")), format!(" 📋 {} ", app.translate("col_list")), format!(" 🕒 {} ", app.translate("col_created")), format!(" 📅 {} ", app.translate("col_due")), " 🍅 ".to_string()]
    } else {
        vec![format!(" {} ", app.translate("col_task")), format!(" 🕒 {} ", app.translate("col_created")), format!(" 📅 {} ", app.translate("col_due")), " 🍅 ".to_string()]
    };
    let header_cells = header_titles.into_iter().map(|h| Cell::from(h).style(Style::default().fg(Palette::mauve(app)).add_modifier(Modifier::BOLD)));
    let header = Row::new(header_cells).height(1).bottom_margin(0);

    let rows: Vec<Row> = app.tasks.iter().enumerate().map(|(i, task)| {
        let is_selected = i == app.selected_task;
        let is_animating = app.animation.task_id.as_ref() == Some(&task.id);
        
        let mut style = if is_selected { Style::default().fg(Palette::base(app)).bg(Palette::mauve(app)) } 
                    else if task.completed { Style::default().fg(Palette::overlay0(app)) }
                    else { Style::default().fg(Palette::text(app)) };

        if is_animating {
            style = style.add_modifier(Modifier::CROSSED_OUT);
        }

        let is_subtask = if let Some(pid) = &task.parent_id { app.tasks.iter().any(|t| &t.id == pid) } else { false };
        let indent = if is_subtask { "  ↳ " } else { "" };
        
        let is_marking = app.marking_done_task_id.as_ref() == Some(&task.id);
        let is_creating = app.creating_task_temp_id.as_ref() == Some(&task.id);
        let check_icon = if is_marking || is_creating {
            let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            format!("{} ", frames[app.spinner_frame % frames.len()])
        } else if task.completed || is_animating {
            "󰄲 ".to_string()
        } else {
            "󰄱 ".to_string()
        };

        let title_content = format!("{}{}{}", indent, check_icon, task.title);
        
        // CONVERSIÓN A TIEMPO LOCAL
        let created_str = app.format_date(task.updated);
        let due_str = task.due.map(|d| app.format_due_date(d)).unwrap_or_else(|| "---".to_string());
        let pomodoros_str = match task.pomodoros {
            0 => String::new(),
            n if n <= 4 => "🍅".repeat(n as usize),
            n => format!("🍅×{}", n),
        };

        // Color semántico de la fecha de vencimiento (solo si la fila no está seleccionada ni completada).
        let due_cell = {
            let mut c = Cell::from(due_str);
            if !is_selected && !task.completed {
                if let Some(d) = task.due {
                    let today = Local::now().date_naive();
                    let due_date = d.date_naive();
                    if due_date < today {
                        c = c.style(Style::default().fg(Palette::red(app)).add_modifier(Modifier::BOLD));
                    } else if due_date == today {
                        c = c.style(Style::default().fg(Palette::peach(app)));
                    }
                }
            }
            c
        };

        let mut cells = vec![Cell::from(title_content)];
        if is_all_view {
            cells.push(Cell::from(app.list_title_for(&task.list_id)));
        }
        cells.push(Cell::from(created_str));
        cells.push(due_cell);
        cells.push(Cell::from(pomodoros_str));

        Row::new(cells).style(style)
    }).collect();

    let mut list_title = if let Some(l) = app.task_lists.get(app.selected_list_idx) {
        format!(" {} - {} ", app.translate("tasks"), l.title)
    } else { format!(" {} ", app.translate("tasks")) };
    if !app.task_filter.is_empty() {
        list_title = format!("{}🔍 {} ", list_title, app.task_filter);
    }

    // Durante la carga con la lista aún vacía, NO bloqueamos con un spinner grande:
    // mostramos solo el panel con borde y dejamos el indicador "Cargando…" en la barra de estado.
    if app.loading && app.tasks.is_empty() && app.marking_done_task_id.is_none() && app.creating_task_temp_id.is_none() {
        let panel = Block::default().title(list_title).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app)));
        frame.render_widget(panel, chunks[1]);
        return;
    }

    if app.tasks.is_empty() {
        let msg_idx = (app.spinner_frame / 100) % 5;
        let empty_msg = app.translate(&format!("empty_list_{}", msg_idx));
        let empty_widget = Paragraph::new(format!("\n\n\n{}", empty_msg))
            .alignment(ratatui::layout::Alignment::Center)
            .style(Style::default().fg(Palette::overlay0(app)).add_modifier(Modifier::ITALIC))
            .block(Block::default().title(list_title).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app))));
        frame.render_widget(empty_widget, chunks[1]);
        return;
    }

    let mut state = TableState::default();
    state.select(Some(app.selected_task));

    let widths: Vec<Constraint> = if is_all_view {
        vec![
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
        ]
    } else {
        vec![
            Constraint::Percentage(55),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
            Constraint::Percentage(15),
        ]
    };

    let table = Table::new(rows, widths)
    .header(header)
    .block(Block::default().title(list_title).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app))))
    .highlight_symbol(">> ");

    frame.render_stateful_widget(table, chunks[1], &mut state);
}
