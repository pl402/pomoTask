use ratatui::{
    layout::{Rect, Alignment},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Table, Row, Cell,
    },
    Frame,
};
use chrono::{Local, Datelike, NaiveDate, NaiveDateTime};
use ratatui::layout::Constraint;

use crate::app::{App, Palette};

pub fn render_calendar(app: &App, frame: &mut Frame, area: Rect) {
    use chrono::Datelike;
    let year = app.calendar_date.year();
    let month = app.calendar_date.month();
    
    // Título del calendario
    let month_name = app.translate(&format!("month_{}", month));
    let title = format!(" {} {} ", month_name, year);
    
    // Agrupar tareas pendientes por día
    let mut pending_counts = std::collections::HashMap::new();
    // Agrupar tareas hechas por día (desde la lista actual)
    let mut done_counts = std::collections::HashMap::new();
    // Agrupar tareas creadas/actualizadas por día
    let mut created_counts = std::collections::HashMap::new();

    for task in &app.all_tasks {
        if !task.completed {
            if let Some(due) = task.due {
                let date = due.date_naive();
                if date.year() == year && date.month() == month {
                    *pending_counts.entry(date.day()).or_insert(0) += 1;
                }
            }
        } else if let Some(comp_at) = task.completed_at {
            let date = comp_at.with_timezone(&Local).date_naive();
            if date.year() == year && date.month() == month {
                *done_counts.entry(date.day()).or_insert(0) += 1;
            }
        }

        let update_date = task.updated.with_timezone(&Local).date_naive();
        if update_date.year() == year && update_date.month() == month {
            *created_counts.entry(update_date.day()).or_insert(0) += 1;
        }
    }

    // Complementar tareas hechas desde stats (pomodoros/sesiones)
    for (key, count) in &app.stats.hourly_tasks_done {
        if let Ok(dt) = NaiveDateTime::parse_from_str(key, "%Y-%m-%d %H:00") {
            if dt.year() == year && dt.month() == month {
                *done_counts.entry(dt.day()).or_insert(0) += *count;
            }
        }
    }

    let first_day = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let start_weekday = first_day.weekday().num_days_from_sunday(); // 0 = Dom, 6 = Sab
    
    let mut rows = Vec::new();
    
    // Header de días
    let day_headers = (0..7).map(|i| {
        let name = app.translate(&format!("day_{}", i));
        Cell::from(Line::from(name.chars().take(2).collect::<String>()).alignment(Alignment::Center))
            .style(Style::default().fg(Palette::subtext0(app.config.theme)))
    }).collect::<Vec<_>>();
    rows.push(Row::new(day_headers));

    let mut current_day = 1;
    let days_in_month = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap()
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap()
    }.signed_duration_since(first_day).num_days();

    let today = Local::now().date_naive();

    for week in 0..6 {
        let mut cells = Vec::new();
        let mut has_days = false;
        for day_of_week in 0..7 {
            if (week == 0 && day_of_week < start_weekday) || current_day > days_in_month as u32 {
                cells.push(Cell::from(""));
            } else {
                has_days = true;
                let pending = pending_counts.get(&current_day).unwrap_or(&0);
                let done = done_counts.get(&current_day).unwrap_or(&0);
                let created = created_counts.get(&current_day).unwrap_or(&0);
                
                let is_today = today.year() == year && today.month() == month && today.day() == current_day;
                
                let mut spans = vec![Span::raw(format!("{:2}", current_day))];
                
                // Semáforo: Rojo (Pendientes), Verde (Hechas), Azul (Creadas/Actualizadas)
                if *pending > 0 {
                    spans.push(Span::styled("█", Style::default().fg(Palette::red(app.config.theme))));
                } else {
                    spans.push(Span::raw(" "));
                }

                if *done > 0 {
                    spans.push(Span::styled("█", Style::default().fg(Palette::green(app.config.theme))));
                } else {
                    spans.push(Span::raw(" "));
                }

                if *created > 0 {
                    spans.push(Span::styled("█", Style::default().fg(Palette::blue(app.config.theme))));
                } else {
                    spans.push(Span::raw(" "));
                }

                let cell_style = if is_today {
                    Style::default().bg(Palette::surface0(app.config.theme)).add_modifier(Modifier::BOLD).add_modifier(Modifier::UNDERLINED)
                } else {
                    Style::default()
                };

                cells.push(Cell::from(Line::from(spans)).style(cell_style));
                current_day += 1;
            }
        }
        if has_days {
            rows.push(Row::new(cells).height(1));
        }
    }

    let table = Table::new(
        rows,
        [Constraint::Length(8); 7]
    )
    .block(Block::default()
        .title(title)
        .title_alignment(Alignment::Center)
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Palette::mauve(app.config.theme))))
    .column_spacing(1);

    frame.render_widget(table, area);
}
