use app::{Action, App, UpdateOutcome};
use clap::Parser;
use input::map_event;
use platform::{EventStream, PlatformEvent, TerminalContext};
use render::{ArgsInterpretation, ManRenderer, SystemManRenderer, ValidationError, classify_args};
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

fn resolve_initial_page(
    args: &[String],
) -> Result<Option<(String, Option<String>)>, ValidationError> {
    match args {
        [] => Ok(None),
        [topic] => Ok(Some((topic.clone(), None))),
        _ => {
            let interpretation = classify_args(args)?;
            match interpretation {
                ArgsInterpretation::SectionAndPages { section, pages } => {
                    Ok(pages.first().map(|page| (page.clone(), Some(section))))
                }
                ArgsInterpretation::Pages(pages) => {
                    Ok(pages.first().map(|page| (page.clone(), None)))
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let renderer = SystemManRenderer::new();

    let initial_page = resolve_initial_page(&cli.args)?;
    if let Some((topic, section)) = &initial_page {
        if let Err(err) = renderer.render(topic, section.as_deref(), 80) {
            if let render::RenderError::CommandFailed(message) = err {
                eprintln!("{message}");
                return Ok(());
            }
            return Err(err.into());
        }
    }

    let mut terminal = TerminalContext::new()?;
    let events = EventStream::new(Duration::from_millis(200));

    let mut app = match initial_page {
        Some((topic, section)) => App::new(topic, section),
        None => App::empty(),
    };

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
