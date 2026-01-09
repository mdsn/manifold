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

fn resolve_initial_pages(
    args: &[String],
) -> Result<Option<(Vec<String>, Option<String>)>, ValidationError> {
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

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let renderer = SystemManRenderer::new();

    let mut terminal = TerminalContext::new()?;
    let events = EventStream::new(Duration::from_millis(200));

    let size = terminal.terminal_mut().size()?;
    let mut content_width = size.width.max(1);
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
