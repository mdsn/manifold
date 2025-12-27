pub use crossterm::event::{Event, KeyCode};
use crossterm::event::{self};
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;
use std::io::{self, Stdout};
use std::time::Duration;

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

pub enum PlatformEvent {
    Input(Event),
    Tick,
}

pub struct EventStream {
    tick_rate: Duration,
}

impl EventStream {
    pub fn new(tick_rate: Duration) -> Self {
        Self { tick_rate }
    }

    pub fn next(&self) -> io::Result<PlatformEvent> {
        if event::poll(self.tick_rate)? {
            Ok(PlatformEvent::Input(event::read()?))
        } else {
            Ok(PlatformEvent::Tick)
        }
    }
}
