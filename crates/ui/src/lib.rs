use app::{App, Mode};
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.size();
    let chunks = layout(size);

    let tab_line = format_tabs(app);
    frame.render_widget(Paragraph::new(tab_line), chunks[0]);

    let text: Vec<Line> = app
        .lines()
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect();
    let paragraph = Paragraph::new(text).scroll((app.scroll() as u16, 0));
    frame.render_widget(paragraph, chunks[1]);

    let status = match app.mode() {
        Mode::Normal => format!("{}  line {}", app.title(), app.scroll() + 1),
        Mode::Command { line } => format!(":{line}"),
    };
    frame.render_widget(Paragraph::new(status), chunks[2]);

    if let Mode::Command { line } = app.mode() {
        let area = chunks[2];
        let mut cursor_x = area.x + 1 + line.len() as u16;
        let max_x = area.x + area.width.saturating_sub(1);
        if cursor_x > max_x {
            cursor_x = max_x;
        }
        frame.set_cursor(cursor_x, area.y);
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
