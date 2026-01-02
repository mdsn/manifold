use app::{App, Mode};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.size();
    let chunks = layout(size);

    let tab_line = format_tabs(app);
    frame.render_widget(Paragraph::new(tab_line), chunks[0]);

    if app.has_tabs() {
        let text: Vec<Line> = build_lines(app);
        let paragraph = Paragraph::new(text).scroll((app.scroll() as u16, 0));
        frame.render_widget(paragraph, chunks[1]);
    } else {
        draw_intro(frame, chunks[1]);
    }

    let status = match app.mode() {
        Mode::Normal => {
            if app.has_tabs() {
                format!("{}  line {}", app.title(), app.scroll() + 1)
            } else {
                String::new()
            }
        }
        Mode::Command { line } => format!(":{line}"),
        Mode::Search { line, .. } => format!("/{line}"),
    };
    frame.render_widget(Paragraph::new(status), chunks[2]);

    match app.mode() {
        Mode::Command { line } => set_prompt_cursor(frame, chunks[2], line),
        Mode::Search { line, .. } => set_prompt_cursor(frame, chunks[2], line),
        Mode::Normal => {}
    }
}

pub fn content_height(height: u16) -> usize {
    height.saturating_sub(2) as usize
}

fn layout(area: Rect) -> [Rect; 3] {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    [chunks[0], chunks[1], chunks[2]]
}

fn format_tabs(app: &App) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::with_capacity(app.tabs().len());
    let active_style = Style::default().add_modifier(Modifier::REVERSED);
    for (index, page) in app.tabs().iter().enumerate() {
        let label = match page.section() {
            Some(section) => format!("{}({})", page.name(), section),
            None => page.name().to_string(),
        };
        let text = format!(" {} ", label);
        let span = if index == app.active_index() {
            Span::styled(text, active_style)
        } else {
            Span::raw(text)
        };
        spans.push(span);
    }
    Line::from(spans)
}

fn build_lines(app: &App) -> Vec<Line<'static>> {
    let Some(query) = app.search_query() else {
        return app
            .lines()
            .iter()
            .map(|line| Line::from(line.to_string()))
            .collect();
    };
    if query.is_empty() {
        return app
            .lines()
            .iter()
            .map(|line| Line::from(line.to_string()))
            .collect();
    }
    let highlight = Style::default().add_modifier(Modifier::REVERSED);
    app.lines()
        .iter()
        .map(|line| highlight_line(line, query, highlight))
        .collect()
}

fn highlight_line(line: &str, query: &str, style: Style) -> Line<'static> {
    let mut spans = Vec::new();
    let mut offset = 0;
    while let Some(pos) = line[offset..].find(query) {
        let start = offset + pos;
        let end = start + query.len();
        if start > offset {
            spans.push(Span::raw(line[offset..start].to_string()));
        }
        spans.push(Span::styled(line[start..end].to_string(), style));
        offset = end;
    }
    if spans.is_empty() {
        return Line::from(line.to_string());
    }
    if offset < line.len() {
        spans.push(Span::raw(line[offset..].to_string()));
    }
    Line::from(spans)
}

fn draw_intro(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from("Manifold"),
        Line::from(""),
        Line::from("Type :man 2 open to open a man page."),
    ];
    let height = lines.len() as u16;
    let rect = centered_rect(area, height);
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, rect);
}

fn centered_rect(area: Rect, height: u16) -> Rect {
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x: area.x,
        y,
        width: area.width,
        height,
    }
}

fn set_prompt_cursor(frame: &mut Frame, area: Rect, line: &str) {
    let mut cursor_x = area.x + 1 + line.len() as u16;
    let max_x = area.x + area.width.saturating_sub(1);
    if cursor_x > max_x {
        cursor_x = max_x;
    }
    frame.set_cursor(cursor_x, area.y);
}
