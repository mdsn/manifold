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
    Suspend,
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

    /// Temporarily restore the caller's terminal, then stop this process.
    ///
    /// `SIGTSTP` returns here when the shell brings the process back to the
    /// foreground, at which point the TUI is reinitialized.
    #[cfg(unix)]
    pub fn suspend(&mut self) -> io::Result<()> {
        self.restore()?;

        // `raise` is safe to call with a valid signal number.  Unlike a
        // terminal-generated SIGTSTP, raw mode delivers Ctrl-Z as input, so
        // we explicitly ask the kernel to suspend this process.
        if unsafe { libc::raise(libc::SIGTSTP) } != 0 {
            let error = io::Error::last_os_error();
            self.activate()?;
            return Err(error);
        }

        self.activate()?;
        self.terminal.clear()?;
        Ok(())
    }

    #[cfg(not(unix))]
    pub fn suspend(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn activate(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        execute!(self.terminal.backend_mut(), EnterAlternateScreen)?;
        Ok(())
    }

    fn restore(&mut self) -> io::Result<()> {
        disable_raw_mode()?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}

impl Drop for TerminalContext {
    fn drop(&mut self) {
        let _ = self.restore();
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
                    if value.eq_ignore_ascii_case(&'z') {
                        Event::Suspend
                    } else {
                        Event::Key(KeyCode::Ctrl(value))
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_control_z_to_suspend() {
        let event = CrosstermEvent::Key(event::KeyEvent::new(
            CrosstermKeyCode::Char('z'),
            event::KeyModifiers::CONTROL,
        ));

        assert_eq!(map_crossterm_event(event), Event::Suspend);
    }
}
