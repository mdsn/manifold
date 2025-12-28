use app::{Action, Mode};
use platform::{Event, KeyCode};

pub fn map_event(event: Event, mode: &Mode) -> Option<Action> {
    match event {
        Event::Resize(width, height) => Some(Action::Resize(width, height)),
        Event::Key(code) => match mode {
            Mode::Normal => match code {
                KeyCode::Char('q') => Some(Action::Quit),
                KeyCode::Char('k') => Some(Action::ScrollUp(1)),
                KeyCode::Char('j') => Some(Action::ScrollDown(1)),
                KeyCode::Char('g') => Some(Action::GoTop),
                KeyCode::Char('G') => Some(Action::GoBottom),
                KeyCode::Char('H') => Some(Action::TabLeft),
                KeyCode::Char('L') => Some(Action::TabRight),
                KeyCode::Char(':') => Some(Action::EnterCommandMode),
                KeyCode::Up => Some(Action::ScrollUp(1)),
                KeyCode::Down => Some(Action::ScrollDown(1)),
                KeyCode::PageUp => Some(Action::PageUp),
                KeyCode::PageDown => Some(Action::PageDown),
                KeyCode::Esc => Some(Action::Quit),
                _ => None,
            },
            Mode::Command { .. } => match code {
                KeyCode::Esc | KeyCode::Ctrl('c') => Some(Action::CommandCancel),
                KeyCode::Enter => Some(Action::CommandSubmit),
                KeyCode::Backspace => Some(Action::CommandBackspace),
                KeyCode::Char(value) if value == ' ' || value.is_ascii_graphic() => {
                    Some(Action::CommandChar(value))
                }
                _ => None,
            },
        },
        Event::Unsupported => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_quit_keys() {
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('q')), &Mode::Normal),
            Some(Action::Quit)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Esc), &Mode::Normal),
            Some(Action::Quit)
        );
    }

    #[test]
    fn maps_scroll_keys() {
        assert_eq!(
            map_event(Event::Key(KeyCode::Up), &Mode::Normal),
            Some(Action::ScrollUp(1))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('k')), &Mode::Normal),
            Some(Action::ScrollUp(1))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Down), &Mode::Normal),
            Some(Action::ScrollDown(1))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('j')), &Mode::Normal),
            Some(Action::ScrollDown(1))
        );
    }

    #[test]
    fn maps_page_keys() {
        assert_eq!(
            map_event(Event::Key(KeyCode::PageUp), &Mode::Normal),
            Some(Action::PageUp)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::PageDown), &Mode::Normal),
            Some(Action::PageDown)
        );
    }

    #[test]
    fn maps_jump_keys() {
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('g')), &Mode::Normal),
            Some(Action::GoTop)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('G')), &Mode::Normal),
            Some(Action::GoBottom)
        );
    }

    #[test]
    fn maps_resize() {
        assert_eq!(
            map_event(Event::Resize(120, 40), &Mode::Normal),
            Some(Action::Resize(120, 40))
        );
    }

    #[test]
    fn ignores_unsupported() {
        assert_eq!(map_event(Event::Unsupported, &Mode::Normal), None);
    }

    #[test]
    fn maps_tab_and_command_keys() {
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('H')), &Mode::Normal),
            Some(Action::TabLeft)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('L')), &Mode::Normal),
            Some(Action::TabRight)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char(':')), &Mode::Normal),
            Some(Action::EnterCommandMode)
        );
    }

    #[test]
    fn maps_command_mode_keys() {
        let mode = Mode::Command {
            line: String::new(),
        };
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('a')), &mode),
            Some(Action::CommandChar('a'))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char(' ')), &mode),
            Some(Action::CommandChar(' '))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Backspace), &mode),
            Some(Action::CommandBackspace)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Enter), &mode),
            Some(Action::CommandSubmit)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Ctrl('c')), &mode),
            Some(Action::CommandCancel)
        );
    }
}
