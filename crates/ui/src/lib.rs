use app::{App, Mode};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use render::{StyledLine, TextStyle};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    if matches!(app.mode(), Mode::Help) {
        draw_help(frame, area);
        return;
    }

    let chunks = layout(area);

    let tab_line = format_tabs(app);
    frame.render_widget(Paragraph::new(tab_line), chunks[0]);

    if app.has_tabs() {
        let text: Vec<Line> = build_lines(app);
        let paragraph = Paragraph::new(text).scroll((app.scroll() as u16, 0));
        frame.render_widget(paragraph, chunks[1]);
    } else {
        draw_intro(frame, chunks[1]);
    }

    let viewport_height = content_height(area.height);
    let status = match app.mode() {
        Mode::Normal => status_line(app, viewport_height),
        Mode::Help => String::new(),
        Mode::Command { line } => format!(":{line}"),
        Mode::Search { line, .. } => format!("/{line}"),
    };
    frame.render_widget(Paragraph::new(status), chunks[2]);

    match app.mode() {
        Mode::Command { line } => set_prompt_cursor(frame, chunks[2], line),
        Mode::Search { line, .. } => set_prompt_cursor(frame, chunks[2], line),
        Mode::Normal | Mode::Help => {}
    }
}

pub fn content_height(height: u16) -> usize {
    height.saturating_sub(2) as usize
}

fn status_line(app: &App, viewport_height: usize) -> String {
    if let Some(message) = app.status_message() {
        return message.to_string();
    }
    if !app.has_tabs() {
        return String::new();
    }
    let line = app.scroll() + 1;
    let title = app.title();
    let total_lines = app.lines().len();
    let percent = percent_label(app.scroll(), total_lines, viewport_height);
    match percent {
        Some(label) => format!("{title}  line {line}  {label}"),
        None => format!("{title}  line {line}"),
    }
}

fn percent_label(scroll: usize, total_lines: usize, viewport_height: usize) -> Option<String> {
    if total_lines == 0 {
        return None;
    }
    let max_scroll = total_lines.saturating_sub(viewport_height.max(1));
    if scroll == 0 {
        return Some("Top".to_string());
    }
    if scroll >= max_scroll {
        return Some("Bot".to_string());
    }
    let percent = (scroll + 1) * 100 / total_lines.max(1);
    Some(format!("{percent}%"))
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
    app.lines()
        .iter()
        .map(|line| styled_line(line, app.search_query().filter(|query| !query.is_empty())))
        .collect()
}

fn styled_line(line: &StyledLine, query: Option<&str>) -> Line<'static> {
    let text = line.plain_text();
    let matches = query
        .map(|query| match_ranges(&text, query))
        .unwrap_or_default();
    let mut spans = Vec::new();
    let mut offset = 0usize;
    for source_span in line.spans() {
        let end = offset + source_span.content.len();
        let mut boundaries = vec![offset, end];
        for &(match_start, match_end) in &matches {
            if match_start > offset && match_start < end {
                boundaries.push(match_start);
            }
            if match_end > offset && match_end < end {
                boundaries.push(match_end);
            }
        }
        boundaries.sort_unstable();
        boundaries.dedup();
        for window in boundaries.windows(2) {
            let segment_start = window[0];
            let segment_end = window[1];
            let highlighted = matches
                .iter()
                .any(|&(start, end)| start <= segment_start && segment_end <= end);
            let mut style = ratatui_style(source_span.style);
            if highlighted {
                style = style.add_modifier(Modifier::REVERSED);
            }
            spans.push(Span::styled(
                source_span.content[segment_start - offset..segment_end - offset].to_string(),
                style,
            ));
        }
        offset = end;
    }
    Line::from(spans)
}

fn match_ranges(text: &str, query: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut offset = 0usize;
    while let Some(position) = text[offset..].find(query) {
        let start = offset + position;
        let end = start + query.len();
        ranges.push((start, end));
        offset = end;
    }
    ranges
}

fn ratatui_style(source: TextStyle) -> Style {
    let mut style = Style::default();
    if source.bold {
        style = style.add_modifier(Modifier::BOLD);
    }
    if source.underlined {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    if source.italic {
        style = style.add_modifier(Modifier::ITALIC);
    }
    if source.reversed {
        style = style.add_modifier(Modifier::REVERSED);
    }
    style
}

fn draw_intro(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from("Manifold"),
        Line::from(""),
        Line::from("Type :man 2 open to open a man page."),
        Line::from("Press ? for help."),
    ];
    let height = lines.len() as u16;
    let rect = centered_rect(area, height);
    let paragraph = Paragraph::new(lines).alignment(Alignment::Center);
    frame.render_widget(paragraph, rect);
}

fn draw_help(frame: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from("Manifold Help"),
        Line::from(""),
        Line::from("Commands"),
        Line::from("  :man [SECTION] TOPIC   Open a man page"),
        Line::from("  :help, :h              Show this help"),
        Line::from("  :wipe, :w              Close current tab"),
        Line::from("  :quit, :q              Quit Manifold"),
        Line::from(""),
        Line::from("Keys"),
        Line::from("  j/k, Up/Down           Scroll line"),
        Line::from("  f/b, PageDown/PageUp   Forward/back a page"),
        Line::from("  d/u                    Half page down/up"),
        Line::from("  g/G                    Top/bottom"),
        Line::from("  H/L                    Previous/next tab"),
        Line::from("  /                      Search"),
        Line::from("  n/p                    Next/previous match"),
        Line::from("  -/+                    Narrow/widen text column"),
        Line::from("  ?                      Show help"),
        Line::from("  q                      Quit help"),
    ];
    let paragraph = Paragraph::new(lines).alignment(Alignment::Left);
    frame.render_widget(paragraph, area);
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
    frame.set_cursor_position((cursor_x, area.y));
}

#[cfg(test)]
mod tests {
    use super::*;
    use app::App;
    use render::{ManRenderer, RenderError, StyledLine, decode_terminal_text};

    struct TestRenderer {
        lines: Vec<String>,
    }

    impl ManRenderer for TestRenderer {
        fn render(
            &self,
            _name: &str,
            _section: Option<&str>,
            _width: u16,
        ) -> Result<Vec<StyledLine>, RenderError> {
            Ok(self.lines.iter().cloned().map(StyledLine::plain).collect())
        }
    }

    fn make_app(line_count: usize, viewport_height: usize) -> App {
        let lines = (0..line_count).map(|idx| format!("line {idx}")).collect();
        let renderer = TestRenderer { lines };
        let mut app = App::new("example", None);
        app.resize_active(&renderer, 80, viewport_height)
            .expect("render");
        app
    }

    #[test]
    fn status_line_shows_top_and_bottom_labels() {
        let viewport_height = 10;
        let mut app = make_app(100, viewport_height);
        assert_eq!(status_line(&app, viewport_height), "example  line 1  Top");
        app.go_bottom(viewport_height);
        assert_eq!(status_line(&app, viewport_height), "example  line 91  Bot");
    }

    #[test]
    fn status_line_shows_percentage_between_top_and_bottom() {
        let viewport_height = 10;
        let mut app = make_app(100, viewport_height);
        app.scroll_down(49, viewport_height);
        assert_eq!(status_line(&app, viewport_height), "example  line 50  50%");
    }

    #[test]
    fn search_highlighting_keeps_source_emphasis() {
        let lines = decode_terminal_text("a\u{1b}[1mb\u{1b}[0mc\n");
        let rendered = styled_line(&lines[0], Some("b"));
        let style = rendered.spans[1].style;
        assert!(style.add_modifier.contains(Modifier::BOLD));
        assert!(style.add_modifier.contains(Modifier::REVERSED));
    }
}
