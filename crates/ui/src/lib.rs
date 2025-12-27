use app::App;
use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::Line;
use ratatui::widgets::Paragraph;

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.size();
    let chunks = layout(size);

    let text: Vec<Line> = app
        .lines()
        .iter()
        .map(|line| Line::from(line.as_str()))
        .collect();
    let paragraph = Paragraph::new(text).scroll((app.scroll() as u16, 0));
    frame.render_widget(paragraph, chunks[0]);

    let status = format!("{}  line {}", app.title(), app.scroll() + 1);
    frame.render_widget(Paragraph::new(status), chunks[1]);
}

pub fn content_height(height: u16) -> usize {
    height.saturating_sub(1) as usize
}

fn layout(area: Rect) -> [Rect; 2] {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);
    [chunks[0], chunks[1]]
}
