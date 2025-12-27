use app::App;
use platform::{Event, EventStream, KeyCode, PlatformEvent, TerminalContext};
use render::SystemManRenderer;
use std::error::Error;
use std::time::Duration;

fn main() -> Result<(), Box<dyn Error>> {
    let mut terminal = TerminalContext::new()?;
    let events = EventStream::new(Duration::from_millis(200));
    let renderer = SystemManRenderer::new();

    let mut app = App::new("open", Some("2".to_string()));

    let size = terminal.terminal_mut().size()?;
    let mut content_width = size.width.max(1);
    let mut content_height = ui::content_height(size.height);
    app.resize(&renderer, content_width, content_height)?;

    loop {
        terminal
            .terminal_mut()
            .draw(|frame| ui::draw(frame, &app))?;

        match events.next()? {
            PlatformEvent::Input(event) => match event {
                Event::Key(key) => match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Up => app.scroll_up(1),
                    KeyCode::Down => app.scroll_down(1, content_height),
                    KeyCode::PageUp => app.page_up(content_height),
                    KeyCode::PageDown => app.page_down(content_height),
                    KeyCode::Char('k') => app.scroll_up(1),
                    KeyCode::Char('j') => app.scroll_down(1, content_height),
                    _ => {}
                },
                Event::Resize(width, height) => {
                    content_width = width.max(1);
                    content_height = ui::content_height(height);
                    app.resize(&renderer, content_width, content_height)?;
                }
                _ => {}
            },
            PlatformEvent::Tick => {}
        }
    }

    Ok(())
}
