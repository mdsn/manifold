use app::{Action, App, UpdateOutcome};
use clap::Parser;
use input::map_event;
use platform::{Event, EventStream, TerminalContext};
use render::{
    ArgsInterpretation, ManRenderer, RenderError, SystemManRenderer, ValidationError, classify_args,
};
use std::error::Error;

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
const DEFAULT_CONTENT_WIDTH: u16 = 80;
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

fn default_content_width(terminal_width: u16) -> u16 {
    terminal_width.min(DEFAULT_CONTENT_WIDTH)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopOutcome {
    NoRedraw,
    Redraw,
    Quit,
}

fn handle_event(
    app: &mut App,
    renderer: &dyn ManRenderer,
    content_width: &mut u16,
    terminal_width: &mut u16,
    content_height: &mut usize,
    event: Event,
) -> Result<LoopOutcome, RenderError> {
    let Some(action) = map_event(event, app.mode()) else {
        return Ok(LoopOutcome::NoRedraw);
    };

    if let Action::Resize(width, height) = action {
        *terminal_width = width.max(1);
        *content_width = clamp_content_width(*content_width, *terminal_width);
        *content_height = ui::content_height(height);
    }
    if let Some(updated_width) = apply_width_action(*content_width, *terminal_width, &action) {
        *content_width = updated_width;
    }

    let outcome = app.update(action, renderer, *content_width, *content_height)?;
    if outcome == UpdateOutcome::Quit {
        return Ok(LoopOutcome::Quit);
    }

    Ok(LoopOutcome::Redraw)
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    let renderer = SystemManRenderer::new();

    let mut terminal = TerminalContext::new()?;
    let events = EventStream::new();

    let size = terminal.terminal_mut().size()?;
    let mut terminal_width = size.width.max(1);
    let mut content_width = default_content_width(terminal_width);
    let mut content_height = ui::content_height(size.height);
    let initial_pages = resolve_initial_pages(&cli.args)?;
    let mut app = App::empty();
    if let Some((topics, section)) = initial_pages {
        app.open_pages(topics, section, &renderer, content_width, content_height)?;
    }
    app.resize_active(&renderer, content_width, content_height)?;

    terminal
        .terminal_mut()
        .draw(|frame| ui::draw(frame, &app))?;

    loop {
        match handle_event(
            &mut app,
            &renderer,
            &mut content_width,
            &mut terminal_width,
            &mut content_height,
            events.next()?,
        )? {
            LoopOutcome::NoRedraw => {}
            LoopOutcome::Redraw => {
                terminal
                    .terminal_mut()
                    .draw(|frame| ui::draw(frame, &app))?;
            }
            LoopOutcome::Quit => break,
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestRenderer {
        lines: Vec<String>,
    }

    impl ManRenderer for TestRenderer {
        fn render(
            &self,
            _name: &str,
            _section: Option<&str>,
            _width: u16,
        ) -> Result<Vec<String>, RenderError> {
            Ok(self.lines.clone())
        }
    }

    fn make_app() -> (App, TestRenderer) {
        let renderer = TestRenderer {
            lines: (0..50).map(|idx| format!("line {idx}")).collect(),
        };
        let mut app = App::new("example", None);
        app.resize_active(&renderer, 80, 20).expect("render");
        (app, renderer)
    }

    #[test]
    fn clamps_content_width_to_terminal_range() {
        assert_eq!(clamp_content_width(10, 50), 15);
        assert_eq!(clamp_content_width(80, 50), 50);
        assert_eq!(clamp_content_width(5, 5), 5);
    }

    #[test]
    fn uses_default_content_width_with_terminal_cap() {
        assert_eq!(default_content_width(300), 80);
        assert_eq!(default_content_width(50), 50);
    }

    #[test]
    fn applies_width_step_with_bounds() {
        assert_eq!(apply_width_action(20, 50, &Action::DecreaseWidth), Some(15));
        assert_eq!(apply_width_action(45, 50, &Action::IncreaseWidth), Some(50));
        assert_eq!(apply_width_action(20, 50, &Action::ScrollUp(1)), None);
    }

    #[test]
    fn handled_input_requests_redraw() {
        let (mut app, renderer) = make_app();
        let mut terminal_width = 100;
        let mut content_width = 80;
        let mut content_height = 20;

        let outcome = handle_event(
            &mut app,
            &renderer,
            &mut content_width,
            &mut terminal_width,
            &mut content_height,
            Event::Key(platform::KeyCode::Down),
        )
        .expect("handled event");

        assert_eq!(outcome, LoopOutcome::Redraw);
        assert_eq!(app.scroll(), 1);
    }

    #[test]
    fn unsupported_input_does_not_request_redraw() {
        let (mut app, renderer) = make_app();
        let mut terminal_width = 100;
        let mut content_width = 80;
        let mut content_height = 20;

        let outcome = handle_event(
            &mut app,
            &renderer,
            &mut content_width,
            &mut terminal_width,
            &mut content_height,
            Event::Unsupported,
        )
        .expect("ignored event");

        assert_eq!(outcome, LoopOutcome::NoRedraw);
        assert_eq!(app.scroll(), 0);
    }

    #[test]
    fn resize_updates_dimensions_and_requests_redraw() {
        let (mut app, renderer) = make_app();
        let mut terminal_width = 100;
        let mut content_width = 80;
        let mut content_height = 20;

        let outcome = handle_event(
            &mut app,
            &renderer,
            &mut content_width,
            &mut terminal_width,
            &mut content_height,
            Event::Resize(60, 30),
        )
        .expect("resize event");

        assert_eq!(outcome, LoopOutcome::Redraw);
        assert_eq!(terminal_width, 60);
        assert_eq!(content_width, 60);
        assert_eq!(content_height, 28);
    }

    #[test]
    fn quit_requests_exit_without_redraw() {
        let mut app = App::empty();
        let renderer = TestRenderer { lines: Vec::new() };
        let mut terminal_width = 100;
        let mut content_width = 80;
        let mut content_height = 20;

        let outcome = handle_event(
            &mut app,
            &renderer,
            &mut content_width,
            &mut terminal_width,
            &mut content_height,
            Event::Key(platform::KeyCode::Char(':')),
        )
        .expect("enter command mode");
        assert_eq!(outcome, LoopOutcome::Redraw);

        let outcome = handle_event(
            &mut app,
            &renderer,
            &mut content_width,
            &mut terminal_width,
            &mut content_height,
            Event::Key(platform::KeyCode::Char('q')),
        )
        .expect("type q");
        assert_eq!(outcome, LoopOutcome::Redraw);

        let outcome = handle_event(
            &mut app,
            &renderer,
            &mut content_width,
            &mut terminal_width,
            &mut content_height,
            Event::Key(platform::KeyCode::Enter),
        )
        .expect("submit quit");

        assert_eq!(outcome, LoopOutcome::Quit);
    }
}
