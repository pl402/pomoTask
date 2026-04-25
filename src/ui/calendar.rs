use ratatui::{
    layout::{Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Table, Row, Cell,
    },
    Frame,
};
use chrono::{Local, NaiveDate, NaiveDateTime, Timelike};
use ratatui::layout::Constraint;

use crate::app::App;
use crate::ui::palette::Palette;

pub fn render_calendar(app: &App, frame: &mut Frame, area: Rect) {
    use chrono::Datelike;
    let year = app.calendar_date.year();
    let month = app.calendar_date.month();
    
    // Título del calendario
    let month_name = app.translate(&format!("month_{}", month));
    let mut title = match app.config.calendar_range {
        crate::app::CalendarRange::Month => format!(" {} {} ", month_name, year),
        crate::app::CalendarRange::Week => format!(" {} {} (Semana) ", month_name, year),
        crate::app::CalendarRange::Day => format!(" {} {} (Día) ", month_name, year),
    };
    
    // Agrupar estadísticas por fecha completa y hora
    let mut pending_counts = std::collections::HashMap::new();
    let mut done_counts: std::collections::HashMap<NaiveDate, i64> = std::collections::HashMap::new();
    let mut created_counts = std::collections::HashMap::new();
    let mut hourly_done = std::collections::HashMap::new(); // (Date, Hour) -> Count

    for task in &app.all_tasks {
        if !task.completed {
            if let Some(due) = task.due {
                let date: NaiveDate = due.date_naive();
                *pending_counts.entry(date).or_insert(0) += 1;
            }
        } else if let Some(comp_at) = task.completed_at {
            let dt_local = comp_at.with_timezone(&Local);
            let date: NaiveDate = dt_local.date_naive();
            let hour = dt_local.hour();
            *done_counts.entry(date).or_insert(0) += 1;
            *hourly_done.entry((date, hour)).or_insert(0) += 1;
        }
        let update_date = task.updated.with_timezone(&Local).date_naive();
        *created_counts.entry(update_date).or_insert(0) += 1;
    }

    for (key, count) in &app.stats.hourly_tasks_done {
        if let Ok(dt) = NaiveDateTime::parse_from_str(key, "%Y-%m-%d %H:00") {
            let date = dt.date();
            let hour = dt.hour();
            *done_counts.entry(date).or_insert(0) += *count as i64;
            *hourly_done.entry((date, hour)).or_insert(0) += *count as i64;
        }
    }

    let today = Local::now().date_naive();
    let mut rows = Vec::new();

    match app.config.calendar_range {
        crate::app::CalendarRange::Month => {
            let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
            let start_weekday = first_day.weekday().num_days_from_sunday();
            let days_in_month = if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
            }.signed_duration_since(first_day).num_days();

            // Header de días
            let day_headers = (0..7).map(|i| {
                let name = app.translate(&format!("day_{}", i));
                Cell::from(Line::from(name.chars().take(2).collect::<String>()).alignment(Alignment::Center)).style(Style::default().fg(Palette::subtext0(app)))
            }).collect::<Vec<_>>();
            rows.push(Row::new(day_headers));

            let mut current_day_num = 1;
            for week in 0..6 {
                let mut cells = Vec::new();
                let mut has_days = false;
                for day_of_week in 0..7 {
                    if (week == 0 && day_of_week < start_weekday) || current_day_num > days_in_month as u32 {
                        cells.push(Cell::from(""));
                    } else {
                        has_days = true;
                        let date = NaiveDate::from_ymd_opt(year, month, current_day_num).unwrap();
                        cells.push(render_day_cell(app, date, today, &pending_counts, &done_counts, &created_counts));
                        current_day_num += 1;
                    }
                }
                if has_days { rows.push(Row::new(cells).height(1)); }
            }
        },
        crate::app::CalendarRange::Week => {
            let start_of_week = app.calendar_date - chrono::Duration::days(app.calendar_date.weekday().num_days_from_sunday() as i64);
            
            // Header: Días de la semana
            let day_headers = std::iter::once(Cell::from("")).chain((0..7).map(|i| {
                let date = start_of_week + chrono::Duration::days(i as i64);
                let name = app.translate(&format!("day_{}", date.weekday().num_days_from_sunday()));
                Cell::from(Line::from(format!("{} {:02}", name.chars().take(2).collect::<String>(), date.day())).alignment(Alignment::Center)).style(Style::default().fg(Palette::subtext0(app)))
            })).collect::<Vec<_>>();
            rows.push(Row::new(day_headers));

            // Horas (mostramos bloque de 8 horas centrado en la hora actual o laborable)
            let current_hour = Local::now().hour();
            let start_h = current_hour.saturating_sub(4).min(16); // Mostrar rango de 8 horas
            
            for h in start_h..start_h+8 {
                let mut cells = vec![Cell::from(format!("{:02}:00", h)).style(Style::default().fg(Palette::overlay0(app)))];
                for i in 0..7 {
                    let date = start_of_week + chrono::Duration::days(i as i64);
                    let count = hourly_done.get(&(date, h as u32)).unwrap_or(&0);
                    let symbol = if *count > 0 { "█" } else { "·" };
                    let style = if *count > 0 { Style::default().fg(Palette::green(app)) } else { Style::default().fg(Palette::surface0(app)) };
                    cells.push(Cell::from(Line::from(symbol).alignment(Alignment::Center)).style(style));
                }
                rows.push(Row::new(cells));
            }
        },
        crate::app::CalendarRange::Day => {
            let date = app.calendar_date;
            let name = app.translate(&format!("day_{}", date.weekday().num_days_from_sunday()));
            title = format!(" {} {} ({}) ", name, date.format("%d %b"), year);
            
            // Vista detallada del día: lista vertical de horas con actividad
            let current_hour = Local::now().hour();
            let start_h = current_hour.saturating_sub(5).min(14);
            
            for h in start_h..start_h+10 {
                let count = hourly_done.get(&(date, h as u32)).unwrap_or(&0);
                let mut spans = vec![
                    Span::styled(format!("{:02}:00  ", h), Style::default().fg(Palette::subtext0(app))),
                ];
                
                if *count > 0 {
                    for _ in 0..(*count).min(5) {
                        spans.push(Span::styled("█", Style::default().fg(Palette::green(app))));
                    }
                    spans.push(Span::raw(format!(" ({} tareas)", count)));
                } else {
                    spans.push(Span::styled("····", Style::default().fg(Palette::surface0(app))));
                }
                
                rows.push(Row::new(vec![Cell::from(Line::from(spans))]));
            }
        }
    }

    let constraints = match app.config.calendar_range {
        crate::app::CalendarRange::Day => vec![Constraint::Percentage(100)],
        crate::app::CalendarRange::Week => vec![
            Constraint::Length(6), // Hora
            Constraint::Length(8), Constraint::Length(8), Constraint::Length(8),
            Constraint::Length(8), Constraint::Length(8), Constraint::Length(8), Constraint::Length(8)
        ],
        _ => vec![Constraint::Length(8); 7],
    };

    let table = Table::new(rows, constraints)
        .block(Block::default().title(title).title_alignment(Alignment::Center).borders(Borders::ALL).border_type(BorderType::Rounded).border_style(Style::default().fg(Palette::mauve(app))))
        .column_spacing(1);

    frame.render_widget(table, area);
}

fn render_day_cell(
    app: &App, 
    date: NaiveDate, 
    today: NaiveDate,
    pending_counts: &std::collections::HashMap<NaiveDate, i64>,
    done_counts: &std::collections::HashMap<NaiveDate, i64>,
    created_counts: &std::collections::HashMap<NaiveDate, i64>,
) -> Cell<'static> {
    use chrono::Datelike;
    let is_today = date == today;
    let cell_style = if is_today {
        Style::default().bg(Palette::surface0(app)).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)
    } else {
        Style::default()
    };

    match app.config.calendar_view {
        crate::app::CalendarView::Standard => {
            let pending = pending_counts.get(&date).unwrap_or(&0);
            let done = done_counts.get(&date).unwrap_or(&0);
            let created = created_counts.get(&date).unwrap_or(&0);
            
            let mut spans = vec![Span::raw(format!("{:2}", date.day()))];
            if *pending > 0 { spans.push(Span::styled("█", Style::default().fg(Palette::red(app)))); } else { spans.push(Span::raw(" ")); }
            if *done > 0 { spans.push(Span::styled("█", Style::default().fg(Palette::green(app)))); } else { spans.push(Span::raw(" ")); }
            if *created > 0 { spans.push(Span::styled("█", Style::default().fg(Palette::blue(app)))); } else { spans.push(Span::raw(" ")); }
            Cell::from(Line::from(spans)).style(cell_style)
        },
        crate::app::CalendarView::Heatmap => {
            let done = *done_counts.get(&date).unwrap_or(&0);
            let intensity = if done == 0 { Palette::overlay0(app) }
                           else if done < 2 { Palette::blue(app) }
                           else if done < 5 { Palette::mauve(app) }
                           else { Palette::green(app) };
            
            let label = if done > 0 { format!("{:2}󰄲", date.day()) } else { format!("{:2} ", date.day()) };
            Cell::from(Span::styled(label, Style::default().fg(intensity))).style(cell_style)
        },
        crate::app::CalendarView::Progress => {
            let pending = *pending_counts.get(&date).unwrap_or(&0) as f32;
            let done = *done_counts.get(&date).unwrap_or(&0) as f32;
            let total = pending + done;
            
            let mut content = format!("{:2} ", date.day());
            let mut style = Style::default().fg(Palette::text(app));
            
            if total > 0.0 {
                let ratio = done / total;
                if ratio >= 1.0 { content.push('󰄲'); style = style.fg(Palette::green(app)); }
                else if ratio > 0.0 { content.push('󰄱'); style = style.fg(Palette::yellow(app)); }
                else { content.push(' '); style = style.fg(Palette::red(app)); }
            } else {
                content.push(' ');
            }
            Cell::from(Span::styled(content, style)).style(cell_style)
        }
    }
}
