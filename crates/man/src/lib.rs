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
    search_query: Option<String>,
    search_matches: Vec<SearchMatch>,
    search_index: Option<usize>,
}

impl ManPage {
    pub fn new(name: impl Into<String>, section: Option<String>) -> Self {
        Self {
            name: name.into(),
            section,
            scroll: 0,
            cache: RenderCache::empty(),
            search_query: None,
            search_matches: Vec::new(),
            search_index: None,
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

    pub fn search_query(&self) -> Option<&str> {
        self.search_query.as_deref()
    }

    pub fn search_matches(&self) -> &[SearchMatch] {
        &self.search_matches
    }

    pub fn search_index(&self) -> Option<usize> {
        self.search_index
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
        if self.search_query.is_some() {
            self.refresh_search(self.scroll);
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

    pub fn update_search(&mut self, query: Option<String>, start_line: usize) {
        let Some(query) = query else {
            self.clear_search();
            return;
        };
        if query.is_empty() {
            self.clear_search();
            return;
        }
        self.search_query = Some(query);
        self.refresh_search(start_line);
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_matches.clear();
        self.search_index = None;
    }

    pub fn next_match_line(&mut self) -> Option<usize> {
        let count = self.search_matches.len();
        if count == 0 {
            self.search_index = None;
            return None;
        }
        let next = match self.search_index {
            Some(index) => (index + 1) % count,
            None => 0,
        };
        self.search_index = Some(next);
        Some(self.search_matches[next].line)
    }

    pub fn previous_match_line(&mut self) -> Option<usize> {
        let count = self.search_matches.len();
        if count == 0 {
            self.search_index = None;
            return None;
        }
        let next = match self.search_index {
            Some(index) => (index + count - 1) % count,
            None => 0,
        };
        self.search_index = Some(next);
        Some(self.search_matches[next].line)
    }

    pub fn current_match_line(&self) -> Option<usize> {
        self.search_index
            .and_then(|index| self.search_matches.get(index).map(|m| m.line))
    }

    fn refresh_search(&mut self, start_line: usize) {
        let Some(query) = self.search_query.as_deref() else {
            self.search_matches.clear();
            self.search_index = None;
            return;
        };
        self.search_matches = collect_matches(&self.cache.lines, query);
        if self.search_matches.is_empty() {
            self.search_index = None;
            return;
        }
        let mut index = None;
        for (idx, entry) in self.search_matches.iter().enumerate() {
            if entry.line >= start_line {
                index = Some(idx);
                break;
            }
        }
        self.search_index = Some(index.unwrap_or(0));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchMatch {
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

fn collect_matches(lines: &[String], query: &str) -> Vec<SearchMatch> {
    if query.is_empty() {
        return Vec::new();
    }
    let mut matches = Vec::new();
    for (line_index, line) in lines.iter().enumerate() {
        let mut offset = 0;
        while let Some(pos) = line[offset..].find(query) {
            let start = offset + pos;
            let end = start + query.len();
            matches.push(SearchMatch {
                line: line_index,
                start,
                end,
            });
            offset = end;
        }
    }
    matches
}
