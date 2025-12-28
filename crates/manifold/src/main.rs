use app::{Action, App};
use input::map_event;
use platform::{EventStream, PlatformEvent, TerminalContext};
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
            PlatformEvent::Input(event) => {
                if let Some(action) = map_event(event) {
                    match action {
                        Action::Quit => break,
                        Action::ScrollUp(amount) => app.scroll_up(amount),
                        Action::ScrollDown(amount) => app.scroll_down(amount, content_height),
                        Action::PageUp => app.page_up(content_height),
                        Action::PageDown => app.page_down(content_height),
                        Action::Resize(width, height) => {
                            content_width = width.max(1);
                            content_height = ui::content_height(height);
                            app.resize(&renderer, content_width, content_height)?;
                        }
                        Action::GoTop => app.go_top(),
                        Action::GoBottom => app.go_bottom(content_height),
                    }
                }
            }
            PlatformEvent::Tick => {}
        }
    }

    Ok(())
}
