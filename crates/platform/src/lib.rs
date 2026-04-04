use crossterm::event::{self, Event as CrosstermEvent, KeyCode as CrosstermKeyCode};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, Stdout};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    Char(char),
    Ctrl(char),
    Up,
    Down,
    PageUp,
    PageDown,
    Enter,
    Backspace,
    Esc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Key(KeyCode),
    Resize(u16, u16),
    Unsupported,
}

pub struct TerminalContext {
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl TerminalContext {
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = Terminal::new(backend)?;
        Ok(Self { terminal })
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for TerminalContext {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
        let _ = self.terminal.show_cursor();
    }
}

pub struct EventStream;

impl EventStream {
    pub fn new() -> Self {
        Self
    }

    pub fn next(&self) -> io::Result<Event> {
        let event = event::read()?;
        Ok(map_crossterm_event(event))
    }
}

fn map_crossterm_event(event: CrosstermEvent) -> Event {
    match event {
        CrosstermEvent::Resize(width, height) => Event::Resize(width, height),
        CrosstermEvent::Key(key) => match key.code {
            CrosstermKeyCode::Char(value) => {
                if key.modifiers.contains(event::KeyModifiers::CONTROL) {
                    Event::Key(KeyCode::Ctrl(value))
                } else {
                    Event::Key(KeyCode::Char(value))
                }
            }
            CrosstermKeyCode::Up => Event::Key(KeyCode::Up),
            CrosstermKeyCode::Down => Event::Key(KeyCode::Down),
            CrosstermKeyCode::PageUp => Event::Key(KeyCode::PageUp),
            CrosstermKeyCode::PageDown => Event::Key(KeyCode::PageDown),
            CrosstermKeyCode::Enter => Event::Key(KeyCode::Enter),
            CrosstermKeyCode::Backspace => Event::Key(KeyCode::Backspace),
            CrosstermKeyCode::Esc => Event::Key(KeyCode::Esc),
            _ => Event::Unsupported,
        },
        _ => Event::Unsupported,
    }
}
