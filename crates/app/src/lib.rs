use man::ManPage;
use render::{ManRenderer, RenderError};

#[derive(Debug)]
pub struct App {
    page: ManPage,
}

impl App {
    pub fn new(name: impl Into<String>, section: Option<String>) -> Self {
        Self {
            page: ManPage::new(name, section),
        }
    }

    pub fn title(&self) -> String {
        match self.page.section() {
            Some(section) => format!("{}({})", self.page.name(), section),
            None => self.page.name().to_string(),
        }
    }

    pub fn lines(&self) -> &[String] {
        self.page.lines()
    }

    pub fn scroll(&self) -> usize {
        self.page.scroll
    }

    pub fn ensure_render(
        &mut self,
        renderer: &dyn ManRenderer,
        width: u16,
    ) -> Result<(), RenderError> {
        self.page.ensure_render(renderer, width)
    }

    pub fn resize(
        &mut self,
        renderer: &dyn ManRenderer,
        width: u16,
        viewport_height: usize,
    ) -> Result<(), RenderError> {
        self.page.ensure_render(renderer, width)?;
        self.clamp_scroll(viewport_height);
        Ok(())
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.page.scroll = self.page.scroll.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        self.page.scroll = (self.page.scroll + amount).min(max_scroll);
    }

    pub fn page_up(&mut self, viewport_height: usize) {
        self.scroll_up(viewport_height);
    }

    pub fn page_down(&mut self, viewport_height: usize) {
        self.scroll_down(viewport_height, viewport_height);
    }

    pub fn clamp_scroll(&mut self, viewport_height: usize) {
        let max_scroll = self.max_scroll(viewport_height);
        if self.page.scroll > max_scroll {
            self.page.scroll = max_scroll;
        }
    }

    fn max_scroll(&self, viewport_height: usize) -> usize {
        let lines = self.page.line_count();
        if lines == 0 {
            return 0;
        }
        let visible = viewport_height.max(1);
        lines.saturating_sub(visible)
    }
}
