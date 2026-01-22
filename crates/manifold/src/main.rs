use app::{Action, App, UpdateOutcome};
use clap::Parser;
use input::map_event;
use platform::{EventStream, PlatformEvent, TerminalContext};
use render::{ArgsInterpretation, SystemManRenderer, ValidationError, classify_args};
use std::error::Error;
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "manifold", about = "Tabbed CLI man page reader", version)]
struct Cli {
    #[arg(
        value_names = ["SECTION", "TOPIC"],
        num_args = 0..,
        help = "Man page to open (TOPIC or SECTION TOPIC)"
    )]
    args: Vec<String>,
}

type PageTopics = Vec<String>;
type PageSection = Option<String>;
type PageSelection = (PageTopics, PageSection);
const WIDTH_STEP: u16 = 5;
const MIN_CONTENT_WIDTH: u16 = 15;

fn resolve_initial_pages(args: &[String]) -> Result<Option<PageSelection>, ValidationError> {
    match args {
        [] => Ok(None),
        [topic] => Ok(Some((vec![topic.clone()], None))),
        _ => {
            let interpretation = classify_args(args)?;
            match interpretation {
                ArgsInterpretation::SectionAndPages { section, pages } => {
                    Ok((!pages.is_empty()).then_some((pages, Some(section))))
                }
                ArgsInterpretation::Pages(pages) => {
                    Ok((!pages.is_empty()).then_some((pages, None)))
                }
            }
        }
    }
}

fn min_content_width(terminal_width: u16) -> u16 {
    terminal_width.min(MIN_CONTENT_WIDTH)
}

fn clamp_content_width(width: u16, terminal_width: u16) -> u16 {
    let min_width = min_content_width(terminal_width);
    width.clamp(min_width, terminal_width)
}

fn apply_width_action(width: u16, terminal_width: u16, action: &Action) -> Option<u16> {
    match action {
        Action::DecreaseWidth => Some(clamp_content_width(
            width.saturating_sub(WIDTH_STEP),
            terminal_width,
        )),
        Action::IncreaseWidth => Some(clamp_content_width(
            width.saturating_add(WIDTH_STEP),
            terminal_width,
        )),
        _ => None,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let renderer = SystemManRenderer::new();

    let mut terminal = TerminalContext::new()?;
    let events = EventStream::new(Duration::from_millis(200));

    let size = terminal.terminal_mut().size()?;
    let mut terminal_width = size.width.max(1);
    let mut content_width = terminal_width;
    let mut content_height = ui::content_height(size.height);
    let initial_pages = resolve_initial_pages(&cli.args)?;
    let mut app = App::empty();
    if let Some((topics, section)) = initial_pages {
        app.open_pages(topics, section, &renderer, content_width, content_height)?;
    }
    app.resize_active(&renderer, content_width, content_height)?;

    loop {
        terminal
            .terminal_mut()
            .draw(|frame| ui::draw(frame, &app))?;

        match events.next()? {
            PlatformEvent::Input(event) => {
                if let Some(action) = map_event(event, app.mode()) {
                    if let Action::Resize(width, height) = action {
                        terminal_width = width.max(1);
                        content_width = clamp_content_width(content_width, terminal_width);
                        content_height = ui::content_height(height);
                    }
                    if let Some(updated_width) =
                        apply_width_action(content_width, terminal_width, &action)
                    {
                        content_width = updated_width;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamps_content_width_to_terminal_range() {
        assert_eq!(clamp_content_width(10, 50), 15);
        assert_eq!(clamp_content_width(80, 50), 50);
        assert_eq!(clamp_content_width(5, 5), 5);
    }

    #[test]
    fn applies_width_step_with_bounds() {
        assert_eq!(apply_width_action(20, 50, &Action::DecreaseWidth), Some(15));
        assert_eq!(apply_width_action(45, 50, &Action::IncreaseWidth), Some(50));
        assert_eq!(apply_width_action(20, 50, &Action::ScrollUp(1)), None);
    }
}
