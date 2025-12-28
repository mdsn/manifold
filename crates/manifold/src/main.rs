use app::{Action, App, UpdateOutcome};
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
    app.resize_active(&renderer, content_width, content_height)?;

    loop {
        terminal
            .terminal_mut()
            .draw(|frame| ui::draw(frame, &app))?;

        match events.next()? {
            PlatformEvent::Input(event) => {
                if let Some(action) = map_event(event, app.mode()) {
                    if let Action::Resize(width, height) = action {
                        content_width = width.max(1);
                        content_height = ui::content_height(height);
                    }
                    let outcome = app.update(action, &renderer, content_width, content_height)?;
                    if outcome == UpdateOutcome::Quit {
                        break;
                    }
                }
            }
            PlatformEvent::Tick => {}
        }
    }

    Ok(())
}
