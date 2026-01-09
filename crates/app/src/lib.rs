use man::ManPage;
use render::{ArgsInterpretation, ManRenderer, RenderError, classify_args};

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
    EnterHelp,
    ExitHelp,
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
    Help,
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
        topics: Vec<String>,
        section: Option<String>,
    },
    Help,
    Quit,
    Wipe,
    Empty,
    Unknown(String),
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
    status_message: Option<String>,
}

impl App {
    pub fn empty() -> Self {
        Self {
            tabs: Vec::new(),
            active: 0,
            mode: Mode::Normal,
            status_message: None,
        }
    }

    pub fn new(name: impl Into<String>, section: Option<String>) -> Self {
        Self {
            tabs: vec![ManPage::new(name, section)],
            active: 0,
            mode: Mode::Normal,
            status_message: None,
        }
    }

    pub fn has_tabs(&self) -> bool {
        !self.tabs.is_empty()
    }

    pub fn title(&self) -> String {
        let Some(page) = self.active_page() else {
            return "Manifold".to_string();
        };
        match page.section() {
            Some(section) => format!("{}({})", page.name(), section),
            None => page.name().to_string(),
        }
    }

    pub fn lines(&self) -> &[String] {
        self.active_page().map(ManPage::lines).unwrap_or(&[])
    }

    pub fn scroll(&self) -> usize {
        self.active_page().map(|page| page.scroll).unwrap_or(0)
    }

    pub fn mode(&self) -> &Mode {
        &self.mode
    }

    pub fn status_message(&self) -> Option<&str> {
        self.status_message.as_deref()
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.status_message = Some(message.into());
    }

    pub fn tabs(&self) -> &[ManPage] {
        &self.tabs
    }

    pub fn search_query(&self) -> Option<&str> {
        self.active_page().and_then(ManPage::search_query)
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
        if self.status_message.is_some() && should_clear_status(&action) {
            self.status_message = None;
        }
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
            Action::EnterHelp => self.mode = Mode::Help,
            Action::ExitHelp => self.mode = Mode::Normal,
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
                    Mode::Help => String::new(),
                    Mode::Search { line, .. } => line,
                };
                let command = parse_command(&line);
                return self.execute_command(command, renderer, width, viewport_height);
            }
        }
        Ok(UpdateOutcome::Continue)
    }

    pub fn open_pages(
        &mut self,
        topics: Vec<String>,
        section: Option<String>,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<(), RenderError> {
        self.open_pages_internal(topics, section, renderer, width, viewport_height)
    }

    pub fn resize_active(
        &mut self,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<(), RenderError> {
        let Some(page) = self.active_page_mut() else {
            return Ok(());
        };
        page.ensure_render(renderer, width)?;
        self.clamp_scroll(viewport_height);
        Ok(())
    }

    pub fn scroll_up(&mut self, amount: usize) {
        let Some(page) = self.active_page_mut() else {
            return;
        };
        let current = page.scroll;
        page.scroll = current.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        let Some(page) = self.active_page_mut() else {
            return;
        };
        let next = (page.scroll + amount).min(max_scroll);
        page.scroll = next;
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
        if let Some(page) = self.active_page_mut() {
            page.scroll = 0;
        }
    }

    pub fn go_bottom(&mut self, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        if let Some(page) = self.active_page_mut() {
            page.scroll = max_scroll;
        }
    }

    pub fn clamp_scroll(&mut self, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        let Some(page) = self.active_page_mut() else {
            return;
        };
        if page.scroll > max_scroll {
            page.scroll = max_scroll;
        }
    }

    fn max_scroll(&self, viewport_height: usize) -> usize {
        let Some(page) = self.active_page() else {
            return 0;
        };
        let lines = page.line_count();
        if lines == 0 {
            return 0;
        }
        let visible = viewport_height.max(1);
        lines.saturating_sub(visible)
    }

    fn active_page(&self) -> Option<&ManPage> {
        self.tabs.get(self.active)
    }

    fn active_page_mut(&mut self) -> Option<&mut ManPage> {
        self.tabs.get_mut(self.active)
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
        let Some(page) = self.active_page() else {
            return;
        };
        let previous = page.search_query().map(|value| value.to_string());
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
        } else if let Some(page) = self.active_page_mut() {
            page.clear_search();
        }
        self.mode = Mode::Normal;
    }

    fn search_next(&mut self, viewport_height: usize) {
        let Some(page) = self.active_page_mut() else {
            return;
        };
        if let Some(line) = page.next_match_line() {
            self.center_on_line(line, viewport_height);
        }
    }

    fn search_prev(&mut self, viewport_height: usize) {
        let Some(page) = self.active_page_mut() else {
            return;
        };
        if let Some(line) = page.previous_match_line() {
            self.center_on_line(line, viewport_height);
        }
    }

    fn search_clear(&mut self) {
        if let Some(page) = self.active_page_mut() {
            page.clear_search();
        }
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
        if let Some(page) = self.active_page_mut() {
            page.ensure_render(renderer, width)?;
        }
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
        if let Some(page) = self.active_page_mut() {
            page.ensure_render(renderer, width)?;
        }
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
            ParsedCommand::Man { topics, section } => {
                self.open_pages_internal(topics, section, renderer, width, viewport_height)?;
                Ok(UpdateOutcome::Continue)
            }
            ParsedCommand::Help => {
                self.mode = Mode::Help;
                Ok(UpdateOutcome::Continue)
            }
            ParsedCommand::Quit => Ok(UpdateOutcome::Quit),
            ParsedCommand::Wipe => {
                if self.tabs.is_empty() {
                    return Ok(UpdateOutcome::Continue);
                }
                self.tabs.remove(self.active);
                if self.tabs.is_empty() {
                    self.active = 0;
                    return Ok(UpdateOutcome::Continue);
                }
                if self.active >= self.tabs.len() {
                    self.active = self.tabs.len() - 1;
                }
                if let Some(page) = self.active_page_mut() {
                    page.ensure_render(renderer, width)?;
                }
                self.clamp_scroll(viewport_height);
                Ok(UpdateOutcome::Continue)
            }
            ParsedCommand::Empty => Ok(UpdateOutcome::Continue),
            ParsedCommand::Unknown(command) => {
                self.status_message = Some(format!("Unknown command '{command}'"));
                Ok(UpdateOutcome::Continue)
            }
        }
    }

    fn apply_search(&mut self, line: &str, viewport_height: usize) {
        let Some(page) = self.active_page_mut() else {
            return;
        };
        let query = line.to_string();
        let start_line = page.scroll;
        page.update_search(Some(query), start_line);
        if let Some(match_line) = page.current_match_line() {
            self.center_on_line(match_line, viewport_height);
        }
    }

    fn center_on_line(&mut self, line: usize, viewport_height: usize) {
        let half = viewport_height / 2;
        let max_scroll = self.max_scroll(viewport_height);
        let desired = line.saturating_sub(half).min(max_scroll);
        if let Some(page) = self.active_page_mut() {
            page.scroll = desired;
        }
    }

    fn open_pages_internal(
        &mut self,
        topics: Vec<String>,
        section: Option<String>,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<(), RenderError> {
        let mut last_error = None;
        for topic in topics {
            self.tabs.push(ManPage::new(topic, section.clone()));
            self.active = self.tabs.len() - 1;
            if let Some(page) = self.active_page_mut()
                && let Err(err) = page.ensure_render(renderer, width)
            {
                self.tabs.remove(self.active);
                if self.active >= self.tabs.len() && !self.tabs.is_empty() {
                    self.active = self.tabs.len() - 1;
                }
                if let RenderError::CommandFailed(message) = err {
                    last_error = Some(message);
                    continue;
                }
                return Err(err);
            }
        }
        if let Some(message) = last_error {
            self.status_message = Some(message);
        }
        if !self.tabs.is_empty() {
            self.clamp_scroll(viewport_height);
        }
        Ok(())
    }
}

fn should_clear_status(action: &Action) -> bool {
    !matches!(action, Action::Resize(_, _) | Action::Quit)
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
                    topics: vec![(*topic).to_string()],
                    section: None,
                },
                [_, ..] => {
                    let interpretation = classify_args(&args).unwrap_or_else(|_| {
                        ArgsInterpretation::Pages(
                            args.iter().map(|value| value.to_string()).collect(),
                        )
                    });
                    match interpretation {
                        ArgsInterpretation::SectionAndPages { section, pages } => {
                            if pages.is_empty() {
                                ParsedCommand::Unknown(command.to_string())
                            } else {
                                ParsedCommand::Man {
                                    topics: pages,
                                    section: Some(section),
                                }
                            }
                        }
                        ArgsInterpretation::Pages(pages) => {
                            if pages.is_empty() {
                                ParsedCommand::Unknown(command.to_string())
                            } else {
                                ParsedCommand::Man {
                                    topics: pages,
                                    section: None,
                                }
                            }
                        }
                    }
                }
                _ => ParsedCommand::Unknown(command.to_string()),
            }
        }
        "help" | "h" => ParsedCommand::Help,
        "quit" | "q" => ParsedCommand::Quit,
        "wipe" | "w" => ParsedCommand::Wipe,
        _ => ParsedCommand::Unknown(command.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::process::{Command, Stdio};

    fn man_available() -> bool {
        Command::new("man")
            .arg("-w")
            .arg("man")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

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

    struct FailingRenderer;

    impl ManRenderer for FailingRenderer {
        fn render(
            &self,
            _name: &str,
            _section: Option<&str>,
            _width: u16,
        ) -> Result<Vec<String>, RenderError> {
            Err(RenderError::CommandFailed(
                "No manual entry for seek".to_string(),
            ))
        }
    }

    #[test]
    fn parses_commands() {
        assert_eq!(
            parse_command("man ls"),
            ParsedCommand::Man {
                topics: vec!["ls".to_string()],
                section: None,
            }
        );
        if man_available() {
            assert_eq!(
                parse_command("man 2 read"),
                ParsedCommand::Man {
                    topics: vec!["read".to_string()],
                    section: Some("2".to_string()),
                }
            );
        }
        assert_eq!(parse_command("quit"), ParsedCommand::Quit);
        assert_eq!(parse_command("q"), ParsedCommand::Quit);
        assert_eq!(parse_command("wipe"), ParsedCommand::Wipe);
        assert_eq!(parse_command("w"), ParsedCommand::Wipe);
        assert_eq!(parse_command("help"), ParsedCommand::Help);
        assert_eq!(parse_command("h"), ParsedCommand::Help);
        assert_eq!(parse_command(""), ParsedCommand::Empty);
        assert_eq!(
            parse_command("bogus"),
            ParsedCommand::Unknown("bogus".to_string())
        );
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

    #[test]
    fn wipe_closes_active_tab_and_handles_empty() {
        let renderer = StubRenderer::new();
        let width: u16 = 80;
        let height: usize = 10;
        let mut app = App::empty();
        app.update(
            Action::Resize(width, height as u16),
            &renderer,
            width,
            height,
        )
        .unwrap();
        app.update(Action::EnterCommandMode, &renderer, width, height)
            .unwrap();
        for ch in "wipe".chars() {
            app.update(Action::CommandChar(ch), &renderer, width, height)
                .unwrap();
        }
        app.update(Action::CommandSubmit, &renderer, width, height)
            .unwrap();
        assert_eq!(app.tabs.len(), 0);

        let mut app = App::new("open", None);
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

        app.update(Action::EnterCommandMode, &renderer, width, height)
            .unwrap();
        for ch in "w".chars() {
            app.update(Action::CommandChar(ch), &renderer, width, height)
                .unwrap();
        }
        app.update(Action::CommandSubmit, &renderer, width, height)
            .unwrap();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active, 0);
        assert_eq!(app.title(), "open");
    }

    #[test]
    fn man_command_sets_status_on_missing_page() {
        let renderer = FailingRenderer;
        let width: u16 = 80;
        let height: usize = 10;
        let mut app = App::new("open", None);
        app.update(Action::EnterCommandMode, &renderer, width, height)
            .unwrap();
        for ch in "man seek".chars() {
            app.update(Action::CommandChar(ch), &renderer, width, height)
                .unwrap();
        }
        app.update(Action::CommandSubmit, &renderer, width, height)
            .unwrap();
        assert_eq!(app.tabs.len(), 1);
        assert_eq!(app.active, 0);
        assert_eq!(app.status_message(), Some("No manual entry for seek"));
    }
}
