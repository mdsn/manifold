use render::{ManRenderer, RenderError};

#[derive(Debug, Clone)]
pub struct RenderCache {
    pub width: u16,
    pub lines: Vec<String>,
}

impl RenderCache {
    pub fn empty() -> Self {
        Self {
            width: 0,
            lines: Vec::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ManPage {
    name: String,
    section: Option<String>,
    pub scroll: usize,
    cache: RenderCache,
}

impl ManPage {
    pub fn new(name: impl Into<String>, section: Option<String>) -> Self {
        Self {
            name: name.into(),
            section,
            scroll: 0,
            cache: RenderCache::empty(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn section(&self) -> Option<&str> {
        self.section.as_deref()
    }

    pub fn lines(&self) -> &[String] {
        &self.cache.lines
    }

    pub fn line_count(&self) -> usize {
        self.cache.lines.len()
    }

    pub fn ensure_render(
        &mut self,
        renderer: &dyn ManRenderer,
        width: u16,
    ) -> Result<(), RenderError> {
        let safe_width = width.max(1);
        if self.cache.width != safe_width || self.cache.lines.is_empty() {
            let lines = renderer.render(&self.name, self.section(), safe_width)?;
            self.cache = RenderCache {
                width: safe_width,
                lines,
            };
        }
        self.clamp_scroll();
        Ok(())
    }

    pub fn clamp_scroll(&mut self) {
        if self.cache.lines.is_empty() {
            self.scroll = 0;
            return;
        }
        let max_scroll = self.cache.lines.len().saturating_sub(1);
        if self.scroll > max_scroll {
            self.scroll = max_scroll;
        }
    }
}
