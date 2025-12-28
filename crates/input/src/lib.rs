use app::Action;
use platform::{Event, KeyCode};

pub fn map_event(event: Event) -> Option<Action> {
    match event {
        Event::Resize(width, height) => Some(Action::Resize(width, height)),
        Event::Key(code) => match code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('k') => Some(Action::ScrollUp(1)),
            KeyCode::Char('j') => Some(Action::ScrollDown(1)),
            KeyCode::Char('g') => Some(Action::GoTop),
            KeyCode::Char('G') => Some(Action::GoBottom),
            KeyCode::Up => Some(Action::ScrollUp(1)),
            KeyCode::Down => Some(Action::ScrollDown(1)),
            KeyCode::PageUp => Some(Action::PageUp),
            KeyCode::PageDown => Some(Action::PageDown),
            KeyCode::Esc => Some(Action::Quit),
            _ => None,
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
            map_event(Event::Key(KeyCode::Char('q'))),
            Some(Action::Quit)
        );
        assert_eq!(map_event(Event::Key(KeyCode::Esc)), Some(Action::Quit));
    }

    #[test]
    fn maps_scroll_keys() {
        assert_eq!(
            map_event(Event::Key(KeyCode::Up)),
            Some(Action::ScrollUp(1))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('k'))),
            Some(Action::ScrollUp(1))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Down)),
            Some(Action::ScrollDown(1))
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('j'))),
            Some(Action::ScrollDown(1))
        );
    }

    #[test]
    fn maps_page_keys() {
        assert_eq!(map_event(Event::Key(KeyCode::PageUp)), Some(Action::PageUp));
        assert_eq!(
            map_event(Event::Key(KeyCode::PageDown)),
            Some(Action::PageDown)
        );
    }

    #[test]
    fn maps_jump_keys() {
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('g'))),
            Some(Action::GoTop)
        );
        assert_eq!(
            map_event(Event::Key(KeyCode::Char('G'))),
            Some(Action::GoBottom)
        );
    }

    #[test]
    fn maps_resize() {
        assert_eq!(
            map_event(Event::Resize(120, 40)),
            Some(Action::Resize(120, 40))
        );
    }

    #[test]
    fn ignores_unsupported() {
        assert_eq!(map_event(Event::Unsupported), None);
    }
}
