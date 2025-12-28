use man::ManPage;
use render::{ManRenderer, RenderError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ScrollUp(usize),
    ScrollDown(usize),
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    Resize(u16, u16),
    GoTop,
    GoBottom,
    TabLeft,
    TabRight,
    EnterCommandMode,
    CommandChar(char),
    CommandBackspace,
    CommandSubmit,
    CommandCancel,
    EnterSearchMode,
    SearchChar(char),
    SearchBackspace,
    SearchSubmit,
    SearchCancel,
    SearchNext,
    SearchPrev,
    SearchClear,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Command {
        line: String,
    },
    Search {
        line: String,
        previous: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ParsedCommand {
    Man {
        topic: String,
        section: Option<String>,
    },
    Quit,
    Wipe,
    Empty,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateOutcome {
    Continue,
    Quit,
}

#[derive(Debug)]
pub struct App {
    tabs: Vec<ManPage>,
    active: usize,
    mode: Mode,
}

impl App {
    pub fn new(name: impl Into<String>, section: Option<String>) -> Self {
        Self {
            tabs: vec![ManPage::new(name, section)],
            active: 0,
            mode: Mode::Normal,
        }
    }

    pub fn title(&self) -> String {
        match self.active_page().section() {
            Some(section) => format!("{}({})", self.active_page().name(), section),
            None => self.active_page().name().to_string(),
        }
    }

    pub fn lines(&self) -> &[String] {
        self.active_page().lines()
    }

    pub fn scroll(&self) -> usize {
        self.active_page().scroll
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn tabs(&self) -> &[ManPage] {
        &self.tabs
    }

    pub fn search_query(&self) -> Option<&str> {
        self.active_page().search_query()
    }

    pub fn active_index(&self) -> usize {
        self.active
    }

    pub fn update(
        &mut self,
        action: Action,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<UpdateOutcome, RenderError> {
        match action {
            Action::Quit => return Ok(UpdateOutcome::Quit),
            Action::ScrollUp(amount) => self.scroll_up(amount),
            Action::ScrollDown(amount) => self.scroll_down(amount, viewport_height),
            Action::PageUp => self.page_up(viewport_height),
            Action::PageDown => self.page_down(viewport_height),
            Action::HalfPageUp => self.half_page_up(viewport_height),
            Action::HalfPageDown => self.half_page_down(viewport_height),
            Action::Resize(_, _) => self.resize_active(renderer, width, viewport_height)?,
            Action::GoTop => self.go_top(),
            Action::GoBottom => self.go_bottom(viewport_height),
            Action::TabLeft => self.switch_tab_left(renderer, width, viewport_height)?,
            Action::TabRight => self.switch_tab_right(renderer, width, viewport_height)?,
            Action::EnterCommandMode => self.enter_command_mode(),
            Action::CommandChar(value) => self.command_char(value),
            Action::CommandBackspace => self.command_backspace(),
            Action::CommandCancel => self.mode = Mode::Normal,
            Action::EnterSearchMode => self.enter_search_mode(),
            Action::SearchChar(value) => self.search_char(value, viewport_height),
            Action::SearchBackspace => self.search_backspace(viewport_height),
            Action::SearchSubmit => self.search_submit(viewport_height),
            Action::SearchCancel => self.search_cancel(viewport_height),
            Action::SearchNext => self.search_next(viewport_height),
            Action::SearchPrev => self.search_prev(viewport_height),
            Action::SearchClear => self.search_clear(),
            Action::CommandSubmit => {
                let line = match std::mem::replace(&mut self.mode, Mode::Normal) {
                    Mode::Command { line } => line,
                    Mode::Normal => String::new(),
                    Mode::Search { line, .. } => line,
                };
                let command = parse_command(&line);
                return self.execute_command(command, renderer, width, viewport_height);
            }
        }
        Ok(UpdateOutcome::Continue)
    }

    pub fn resize_active(
        &mut self,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<(), RenderError> {
        self.active_page_mut().ensure_render(renderer, width)?;
        self.clamp_scroll(viewport_height);
        Ok(())
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let current = self.active_page().scroll;
        self.active_page_mut().scroll = current.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        let next = (self.active_page().scroll + amount).min(max_scroll);
        self.active_page_mut().scroll = next;
    }

    pub fn page_up(&mut self, viewport_height: usize) {
        self.scroll_up(viewport_height);
    }

    pub fn page_down(&mut self, viewport_height: usize) {
        self.scroll_down(viewport_height, viewport_height);
    }

    pub fn half_page_up(&mut self, viewport_height: usize) {
        let amount = (viewport_height / 2).max(1);
        self.scroll_up(amount);
    }

    pub fn half_page_down(&mut self, viewport_height: usize) {
        let amount = (viewport_height / 2).max(1);
        self.scroll_down(amount, viewport_height);
    }

    pub fn go_top(&mut self) {
        self.active_page_mut().scroll = 0;
    }

    pub fn go_bottom(&mut self, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        self.active_page_mut().scroll = max_scroll;
    }

    pub fn clamp_scroll(&mut self, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        if self.active_page().scroll > max_scroll {
            self.active_page_mut().scroll = max_scroll;
        }
    }

    fn max_scroll(&self, viewport_height: usize) -> usize {
        let lines = self.active_page().line_count();
        if lines == 0 {
            return 0;
        }
        let visible = viewport_height.max(1);
        lines.saturating_sub(visible)
    }

    fn active_page(&self) -> &ManPage {
        &self.tabs[self.active]
    }

    fn active_page_mut(&mut self) -> &mut ManPage {
        &mut self.tabs[self.active]
    }

    fn enter_command_mode(&mut self) {
        self.mode = Mode::Command {
            line: String::new(),
        };
    }

    fn command_char(&mut self, value: char) {
        if let Mode::Command { line } = &mut self.mode {
            line.push(value);
        }
    }

    fn command_backspace(&mut self) {
        if let Mode::Command { line } = &mut self.mode {
            line.pop();
        }
    }

    fn enter_search_mode(&mut self) {
        let previous = self
            .active_page()
            .search_query()
            .map(|value| value.to_string());
        self.mode = Mode::Search {
            line: String::new(),
            previous,
        };
    }

    fn search_char(&mut self, value: char, viewport_height: usize) {
        let query = match &mut self.mode {
            Mode::Search { line, .. } => {
                line.push(value);
                line.clone()
            }
            _ => return,
        };
        self.apply_search(&query, viewport_height);
    }

    fn search_backspace(&mut self, viewport_height: usize) {
        let query = match &mut self.mode {
            Mode::Search { line, .. } => {
                line.pop();
                line.clone()
            }
            _ => return,
        };
        self.apply_search(&query, viewport_height);
    }

    fn search_submit(&mut self, viewport_height: usize) {
        let query = match &self.mode {
            Mode::Search { line, .. } => line.clone(),
            _ => return,
        };
        self.apply_search(&query, viewport_height);
        self.mode = Mode::Normal;
    }

    fn search_cancel(&mut self, viewport_height: usize) {
        let previous = match &self.mode {
            Mode::Search { previous, .. } => previous.clone(),
            _ => return,
        };
        if let Some(prev) = previous {
            self.apply_search(&prev, viewport_height);
        } else {
            self.active_page_mut().clear_search();
        }
        self.mode = Mode::Normal;
    }

    fn search_next(&mut self, viewport_height: usize) {
        if let Some(line) = self.active_page_mut().next_match_line() {
            self.center_on_line(line, viewport_height);
        }
    }

    fn search_prev(&mut self, viewport_height: usize) {
        if let Some(line) = self.active_page_mut().previous_match_line() {
            self.center_on_line(line, viewport_height);
        }
    }

    fn search_clear(&mut self) {
        self.active_page_mut().clear_search();
    }

    fn switch_tab_left(
        &mut self,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<(), RenderError> {
        if self.tabs.is_empty() {
            return Ok(());
        }
        if self.active == 0 {
            self.active = self.tabs.len() - 1;
        } else {
            self.active -= 1;
        }
        self.active_page_mut().ensure_render(renderer, width)?;
        self.clamp_scroll(viewport_height);
        Ok(())
    }

    fn switch_tab_right(
        &mut self,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<(), RenderError> {
        if self.tabs.is_empty() {
            return Ok(());
        }
        self.active = (self.active + 1) % self.tabs.len();
        self.active_page_mut().ensure_render(renderer, width)?;
        self.clamp_scroll(viewport_height);
        Ok(())
    }

    fn execute_command(
        &mut self,
        command: ParsedCommand,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<UpdateOutcome, RenderError> {
        match command {
            ParsedCommand::Man { topic, section } => {
                self.tabs.push(ManPage::new(topic, section));
                self.active = self.tabs.len() - 1;
                self.active_page_mut().ensure_render(renderer, width)?;
                self.clamp_scroll(viewport_height);
                Ok(UpdateOutcome::Continue)
            }
            ParsedCommand::Quit => Ok(UpdateOutcome::Quit),
            ParsedCommand::Wipe => Ok(UpdateOutcome::Continue),
            ParsedCommand::Empty | ParsedCommand::Unknown => Ok(UpdateOutcome::Continue),
        }
    }

    fn apply_search(&mut self, line: &str, viewport_height: usize) {
        let query = line.to_string();
        let start_line = self.active_page().scroll;
        self.active_page_mut()
            .update_search(Some(query), start_line);
        if let Some(match_line) = self.active_page().current_match_line() {
            self.center_on_line(match_line, viewport_height);
        }
    }

    fn center_on_line(&mut self, line: usize, viewport_height: usize) {
        let half = viewport_height / 2;
        let max_scroll = self.max_scroll(viewport_height);
        let desired = line.saturating_sub(half).min(max_scroll);
        self.active_page_mut().scroll = desired;
    }
}

fn parse_command(line: &str) -> ParsedCommand {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ParsedCommand::Empty;
    }
    let mut parts = trimmed.split_whitespace();
    let command = match parts.next() {
        Some(value) => value,
        None => return ParsedCommand::Empty,
    };
    match command {
        "man" => {
            let args: Vec<&str> = parts.collect();
            match args.as_slice() {
                [topic] => ParsedCommand::Man {
                    topic: (*topic).to_string(),
                    section: None,
                },
                [section, topic] => ParsedCommand::Man {
                    topic: (*topic).to_string(),
                    section: Some((*section).to_string()),
                },
                _ => ParsedCommand::Unknown,
            }
        }
        "quit" | "q" => ParsedCommand::Quit,
        "wipe" | "w" => ParsedCommand::Wipe,
        _ => ParsedCommand::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    struct StubRenderer {
        calls: Cell<usize>,
    }

    impl StubRenderer {
        fn new() -> Self {
            Self {
                calls: Cell::new(0),
            }
        }
    }

    impl ManRenderer for StubRenderer {
        fn render(
            &self,
            name: &str,
            _section: Option<&str>,
            width: u16,
        ) -> Result<Vec<String>, RenderError> {
            let next = self.calls.get() + 1;
            self.calls.set(next);
            let line = format!("{name}:{width}");
            Ok(vec![line; 50])
        }
    }

    struct LinesRenderer {
        lines: Vec<String>,
    }

    impl LinesRenderer {
        fn new(lines: Vec<String>) -> Self {
            Self { lines }
        }
    }

    impl ManRenderer for LinesRenderer {
        fn render(
            &self,
            _name: &str,
            _section: Option<&str>,
            _width: u16,
        ) -> Result<Vec<String>, RenderError> {
            Ok(self.lines.clone())
        }
    }

    #[test]
    fn parses_commands() {
        assert_eq!(
            parse_command("man ls"),
            ParsedCommand::Man {
                topic: "ls".to_string(),
                section: None,
            }
        );
        assert_eq!(
            parse_command("man 2 read"),
            ParsedCommand::Man {
                topic: "read".to_string(),
                section: Some("2".to_string()),
            }
        );
        assert_eq!(parse_command("quit"), ParsedCommand::Quit);
        assert_eq!(parse_command("q"), ParsedCommand::Quit);
        assert_eq!(parse_command("wipe"), ParsedCommand::Wipe);
        assert_eq!(parse_command("w"), ParsedCommand::Wipe);
        assert_eq!(parse_command(""), ParsedCommand::Empty);
        assert_eq!(parse_command("bogus"), ParsedCommand::Unknown);
    }

    #[test]
    fn switches_tabs_and_preserves_scroll() {
        let renderer = StubRenderer::new();
        let mut app = App::new("open", None);
        let width: u16 = 80;
        let height: usize = 10;
        app.update(
            Action::Resize(width, height as u16),
            &renderer,
            width,
            height,
        )
        .unwrap();
        app.update(Action::ScrollDown(3), &renderer, width, height)
            .unwrap();
        app.update(Action::EnterCommandMode, &renderer, width, height)
            .unwrap();
        for ch in "man ls".chars() {
            app.update(Action::CommandChar(ch), &renderer, width, height)
                .unwrap();
        }
        app.update(Action::CommandSubmit, &renderer, width, height)
            .unwrap();
        assert_eq!(app.tabs.len(), 2);
        assert_eq!(app.active, 1);
        assert_eq!(app.scroll(), 0);

        app.update(Action::TabLeft, &renderer, width, height)
            .unwrap();
        assert_eq!(app.active, 0);
        assert_eq!(app.scroll(), 3);

        app.update(Action::TabRight, &renderer, width, height)
            .unwrap();
        assert_eq!(app.active, 1);
    }

    #[test]
    fn search_centers_and_navigates() {
        let mut lines = Vec::new();
        for idx in 0..40 {
            if idx == 10 || idx == 30 {
                lines.push(format!("foo line {idx}"));
            } else {
                lines.push(format!("line {idx}"));
            }
        }
        let renderer = LinesRenderer::new(lines);
        let mut app = App::new("open", None);
        let width: u16 = 80;
        let height: usize = 10;
        app.update(
            Action::Resize(width, height as u16),
            &renderer,
            width,
            height,
        )
        .unwrap();

        app.update(Action::EnterSearchMode, &renderer, width, height)
            .unwrap();
        for ch in "foo".chars() {
            app.update(Action::SearchChar(ch), &renderer, width, height)
                .unwrap();
        }
        app.update(Action::SearchSubmit, &renderer, width, height)
            .unwrap();
        assert_eq!(app.scroll(), 5);

        app.update(Action::SearchNext, &renderer, width, height)
            .unwrap();
        assert_eq!(app.scroll(), 25);

        app.update(Action::SearchClear, &renderer, width, height)
            .unwrap();
        let scroll = app.scroll();
        app.update(Action::SearchNext, &renderer, width, height)
            .unwrap();
        assert_eq!(app.scroll(), scroll);
    }
}
