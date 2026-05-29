use ratatui::prelude::*;
use ratatui::widgets::{Block, Clear, List, ListItem, Paragraph, Wrap};

use crate::app::{App, EntryKind, FormField, Mode};
use crate::widgets::Widgets;

pub fn draw(f: &mut Frame, app: &mut App, widgets: &Widgets) {
    let area = f.area();
    let chunks = Layout::vertical([
        Constraint::Length(3), // banner
        Constraint::Length(3), // widget row
        Constraint::Length(1), // filter line
        Constraint::Min(1),    // list
        Constraint::Length(1), // footer
    ])
    .split(area);

    draw_banner(f, chunks[0], widgets);
    draw_widgets(f, chunks[1], widgets);
    draw_filter(f, chunks[2], app);
    draw_list(f, chunks[3], app);
    draw_footer(f, chunks[4], app);

    match app.mode {
        Mode::Form => draw_form(f, area, app),
        Mode::Confirm => draw_confirm(f, area, app),
        _ => {}
    }
    if app.show_help {
        draw_help(f, area);
    }
}

fn draw_banner(f: &mut Frame, area: Rect, widgets: &Widgets) {
    let block = Block::bordered();
    let inner = block.inner(area);
    f.render_widget(block, area);

    let host = Paragraph::new(widgets.hostname.clone())
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .alignment(Alignment::Center);
    f.render_widget(host, inner);

    let uptime = Paragraph::new(widgets.uptime_str())
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Right);
    f.render_widget(uptime, inner);
}

fn draw_widgets(f: &mut Frame, area: Rect, widgets: &Widgets) {
    let cols = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .split(area);

    draw_memory(f, cols[0], widgets);

    let cell = |title: &'static str, value: String, color: Color| {
        Paragraph::new(value)
            .alignment(Alignment::Center)
            .style(Style::default().fg(color))
            .block(Block::bordered().title(title))
    };
    f.render_widget(cell(" Bitcoin ", widgets.btc_str(), Color::Yellow), cols[1]);
    f.render_widget(cell(" Weather ", widgets.weather_str(), Color::Blue), cols[2]);
}

fn draw_filter(f: &mut Frame, area: Rect, app: &App) {
    let (prefix, text, style) = match app.mode {
        Mode::Command => (":", app.cmdline.as_str(), Style::default().fg(Color::Magenta)),
        Mode::Search => ("> ", app.query.as_str(), Style::default().fg(Color::White)),
        _ => ("> ", app.query.as_str(), Style::default().fg(Color::DarkGray)),
    };
    let line = Line::from(vec![
        Span::styled(prefix, style.add_modifier(Modifier::BOLD)),
        Span::styled(text.to_string(), style),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn draw_list(f: &mut Frame, area: Rect, app: &mut App) {
    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|i| {
            let e = &app.config.entries[*i];
            if e.is_heading() {
                ListItem::new(Line::from(Span::styled(
                    format!("── {} ", e.heading_str()),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )))
            } else {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<22}", truncate(e.name_str(), 22)),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        e.description_str().to_string(),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]))
            }
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title(" Commands "))
        .highlight_style(
            Style::default()
                .add_modifier(Modifier::REVERSED)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▌ ");

    f.render_stateful_widget(list, area, &mut app.list_state);
}

fn draw_footer(f: &mut Frame, area: Rect, app: &App) {
    let text = if !app.status.is_empty() {
        app.status.clone()
    } else {
        match app.mode {
            Mode::Normal => {
                "j/k move · / search · Enter run · o new · c edit · dd del · ? help · q quit"
                    .to_string()
            }
            Mode::Search => "type to filter · Enter accept · Esc clear".to_string(),
            Mode::Command => "Enter to run command · Esc cancel".to_string(),
            Mode::Form => "j/k field · i/Enter edit · Ctrl-s/ZZ save · q/Esc cancel".to_string(),
            Mode::Confirm => "y confirm · n/Esc cancel".to_string(),
        }
    };
    f.render_widget(
        Paragraph::new(text).style(Style::default().fg(Color::DarkGray)),
        area,
    );
}

fn draw_form(f: &mut Frame, area: Rect, app: &App) {
    let rect = centered_rect(60, 60, area);
    f.render_widget(Clear, rect);

    let kind_label = match app.form.kind {
        EntryKind::Command => "[Command]  Heading ",
        EntryKind::Heading => " Command  [Heading]",
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(field_line(app, FormField::Type, "Type", kind_label.to_string()));
    let name_label = match app.form.kind {
        EntryKind::Heading => "Title",
        EntryKind::Command => "Name",
    };
    lines.push(field_line(app, FormField::Name, name_label, app.form.name.clone()));
    if app.form.kind == EntryKind::Command {
        lines.push(field_line(
            app,
            FormField::Description,
            "Description",
            app.form.description.clone(),
        ));
        lines.push(field_line(
            app,
            FormField::Command,
            "Command",
            app.form.command.clone(),
        ));
    }
    lines.push(Line::from(""));
    let hint = if app.form.editing {
        "editing — Esc to stop"
    } else {
        "i/Enter edit field · Ctrl-s save"
    };
    lines.push(Line::from(Span::styled(
        hint,
        Style::default().fg(Color::DarkGray),
    )));

    let p = Paragraph::new(lines)
        .block(
            Block::bordered()
                .title(format!(" {} ", app.form.title_str()))
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, rect);
}

fn field_line<'a>(app: &App, field: FormField, label: &'a str, value: String) -> Line<'a> {
    let focused = app.form.field == field;
    let editing = focused && app.form.editing;
    let label_style = if focused {
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let value_style = if editing {
        Style::default().add_modifier(Modifier::REVERSED)
    } else if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let marker = if focused { "› " } else { "  " };
    let shown = if editing {
        format!("{value}_")
    } else if value.is_empty() {
        "—".to_string()
    } else {
        value
    };
    Line::from(vec![
        Span::styled(marker, label_style),
        Span::styled(format!("{label:<12}"), label_style),
        Span::styled(shown, value_style),
    ])
}

fn draw_confirm(f: &mut Frame, area: Rect, app: &App) {
    let name = app
        .current_entry_index()
        .map(|i| {
            let e = &app.config.entries[i];
            if e.is_heading() {
                e.heading_str().to_string()
            } else {
                e.name_str().to_string()
            }
        })
        .unwrap_or_default();
    let rect = centered_rect(50, 20, area);
    f.render_widget(Clear, rect);
    let p = Paragraph::new(vec![
        Line::from(format!("Delete \"{name}\"?")),
        Line::from(""),
        Line::from(Span::styled(
            "y = yes    n / Esc = no",
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .alignment(Alignment::Center)
    .block(
        Block::bordered()
            .title(" Confirm delete ")
            .border_style(Style::default().fg(Color::Red)),
    );
    f.render_widget(p, rect);
}

fn draw_help(f: &mut Frame, area: Rect) {
    let rect = centered_rect(60, 70, area);
    f.render_widget(Clear, rect);
    let lines = vec![
        Line::from("Navigation"),
        Line::from("  j / k, ↓ / ↑    move (skips headings)"),
        Line::from("  gg / G          first / last"),
        Line::from("  Ctrl-d / Ctrl-u half page"),
        Line::from(""),
        Line::from("Actions"),
        Line::from("  Enter / l       run selected command"),
        Line::from("  /               search / filter"),
        Line::from("  o               new entry"),
        Line::from("  c               edit selected"),
        Line::from("  dd              delete selected"),
        Line::from("  q / :q          quit"),
        Line::from(""),
        Line::from(Span::styled(
            "press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];
    let p = Paragraph::new(lines).block(
        Block::bordered()
            .title(format!(" tuimenu v{} — Help ", env!("CARGO_PKG_VERSION")))
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(p, rect);
}

fn draw_memory(f: &mut Frame, area: Rect, widgets: &Widgets) {
    let block = Block::bordered().title(" Mem Used ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let mem_str = widgets.memory_str();
    let ratio = widgets.memory_ratio();
    let bar_color = if ratio < 0.85 { Color::Cyan } else { Color::Red };

    // Reserve space for " 12.3/16.0 GB" after the bar
    let label_len = mem_str.len() as u16 + 1;
    let bar_width = inner.width.saturating_sub(label_len) as usize;
    let filled = (ratio * bar_width as f64).round() as usize;
    let unfilled = bar_width.saturating_sub(filled);

    let line = Line::from(vec![
        Span::styled(
            "█".repeat(filled),
            Style::default().fg(bar_color).add_modifier(Modifier::DIM),
        ),
        Span::styled("░".repeat(unfilled), Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::styled(mem_str, Style::default().fg(Color::Gray)),
    ]);
    f.render_widget(Paragraph::new(line), inner);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
        t.push('…');
        t
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - percent_y) / 2),
        Constraint::Percentage(percent_y),
        Constraint::Percentage((100 - percent_y) / 2),
    ])
    .split(area);
    Layout::horizontal([
        Constraint::Percentage((100 - percent_x) / 2),
        Constraint::Percentage(percent_x),
        Constraint::Percentage((100 - percent_x) / 2),
    ])
    .split(vertical[1])[1]
}
