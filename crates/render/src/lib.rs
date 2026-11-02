use std::fmt;
use std::process::{Command, Stdio};

mod args_validation;

pub use args_validation::{ArgsInterpretation, ValidationError, classify_args};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TextStyle {
    pub bold: bool,
    pub underlined: bool,
    pub italic: bool,
    pub reversed: bool,
}

impl TextStyle {
    fn combine(self, other: Self) -> Self {
        Self {
            bold: self.bold || other.bold,
            underlined: self.underlined || other.underlined,
            italic: self.italic || other.italic,
            reversed: self.reversed || other.reversed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StyledSpan {
    pub content: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StyledLine {
    spans: Vec<StyledSpan>,
}

impl StyledLine {
    pub fn plain(text: impl Into<String>) -> Self {
        let content = text.into();
        if content.is_empty() {
            Self::default()
        } else {
            Self {
                spans: vec![StyledSpan {
                    content,
                    style: TextStyle::default(),
                }],
            }
        }
    }

    pub fn spans(&self) -> &[StyledSpan] {
        &self.spans
    }

    pub fn plain_text(&self) -> String {
        self.spans
            .iter()
            .map(|span| span.content.as_str())
            .collect()
    }
}

impl From<String> for StyledLine {
    fn from(value: String) -> Self {
        Self::plain(value)
    }
}

impl From<&str> for StyledLine {
    fn from(value: &str) -> Self {
        Self::plain(value)
    }
}

#[derive(Debug)]
pub enum RenderError {
    Io(std::io::Error),
    Utf8(std::string::FromUtf8Error),
    CommandFailed(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RenderError::Io(err) => write!(f, "io error: {err}"),
            RenderError::Utf8(err) => write!(f, "utf8 error: {err}"),
            RenderError::CommandFailed(msg) => write!(f, "command failed: {msg}"),
        }
    }
}

impl std::error::Error for RenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            RenderError::Io(err) => Some(err),
            RenderError::Utf8(err) => Some(err),
            RenderError::CommandFailed(_) => None,
        }
    }
}

impl From<std::io::Error> for RenderError {
    fn from(value: std::io::Error) -> Self {
        RenderError::Io(value)
    }
}

impl From<std::string::FromUtf8Error> for RenderError {
    fn from(value: std::string::FromUtf8Error) -> Self {
        RenderError::Utf8(value)
    }
}

pub trait ManRenderer {
    fn render(
        &self,
        name: &str,
        section: Option<&str>,
        width: u16,
    ) -> Result<Vec<StyledLine>, RenderError>;
}

#[derive(Debug, Default)]
pub struct SystemManRenderer;

impl SystemManRenderer {
    pub fn new() -> Self {
        Self
    }
}

impl ManRenderer for SystemManRenderer {
    fn render(
        &self,
        name: &str,
        section: Option<&str>,
        width: u16,
    ) -> Result<Vec<StyledLine>, RenderError> {
        let safe_width = width.max(1).to_string();
        let mut man_cmd = Command::new("man");
        man_cmd.env("MANWIDTH", &safe_width).env("MANPAGER", "cat");

        if let Some(section) = section {
            man_cmd.arg(section);
        }
        man_cmd.arg(name);
        man_cmd.stdout(Stdio::piped());
        man_cmd.stderr(Stdio::piped());

        let output = man_cmd.output()?;
        if !output.status.success() {
            let message = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let message = if message.is_empty() {
                format!("man exited with {}", output.status)
            } else {
                message
            };
            return Err(RenderError::CommandFailed(message));
        }

        let text = String::from_utf8(output.stdout)?;
        Ok(decode_terminal_text(&text))
    }
}

#[derive(Clone, Copy)]
struct Cell {
    character: char,
    style: TextStyle,
}

/// Converts terminal-oriented `man` output into visible text and semantic styles.
/// Traditional groff/mandoc output uses overstrikes for emphasis; some systems use
/// ANSI SGR instead. Both forms are accepted here so the system `man` command stays
/// the portability boundary.
pub fn decode_terminal_text(text: &str) -> Vec<StyledLine> {
    text.split_terminator('\n').map(decode_line).collect()
}

fn decode_line(raw: &str) -> StyledLine {
    let chars: Vec<char> = raw.chars().collect();
    let mut cells = Vec::new();
    let mut cursor = 0usize;
    let mut style = TextStyle::default();
    let mut index = 0usize;

    while index < chars.len() {
        match chars[index] {
            '\u{1b}' => index = consume_escape(&chars, index, &mut style),
            '\u{8}' => {
                cursor = cursor.saturating_sub(1);
                index += 1;
            }
            '\r' => {
                cursor = 0;
                index += 1;
            }
            '\t' => {
                let next_stop = ((cursor / 8) + 1) * 8;
                while cursor < next_stop {
                    write_cell(&mut cells, &mut cursor, ' ', style);
                }
                index += 1;
            }
            character if character.is_control() => index += 1,
            character => {
                write_cell(&mut cells, &mut cursor, character, style);
                index += 1;
            }
        }
    }

    cells_to_line(cells)
}

fn write_cell(cells: &mut Vec<Cell>, cursor: &mut usize, character: char, style: TextStyle) {
    if *cursor >= cells.len() {
        cells.resize(
            *cursor + 1,
            Cell {
                character: ' ',
                style: TextStyle::default(),
            },
        );
        cells[*cursor] = Cell { character, style };
    } else {
        let existing = &mut cells[*cursor];
        if existing.character == '_' && character != '_' {
            existing.character = character;
            existing.style = existing.style.combine(style);
            existing.style.underlined = true;
        } else if character == '_' && existing.character != '_' {
            existing.style = existing.style.combine(style);
            existing.style.underlined = true;
        } else if existing.character == character {
            existing.style = existing.style.combine(style);
            existing.style.bold = true;
        } else {
            existing.character = character;
            existing.style = existing.style.combine(style);
        }
    }
    *cursor += 1;
}

fn cells_to_line(cells: Vec<Cell>) -> StyledLine {
    let mut spans: Vec<StyledSpan> = Vec::new();
    for cell in cells {
        match spans.last_mut() {
            Some(span) if span.style == cell.style => span.content.push(cell.character),
            _ => spans.push(StyledSpan {
                content: cell.character.to_string(),
                style: cell.style,
            }),
        }
    }
    StyledLine { spans }
}

fn consume_escape(chars: &[char], start: usize, style: &mut TextStyle) -> usize {
    let Some(kind) = chars.get(start + 1).copied() else {
        return start + 1;
    };
    if kind != '[' {
        return start + 2;
    }

    let mut end = start + 2;
    while end < chars.len() && !(('\u{40}'..='\u{7e}').contains(&chars[end])) {
        end += 1;
    }
    if end == chars.len() {
        return end;
    }
    if chars[end] == 'm' {
        apply_sgr(&chars[start + 2..end], style);
    }
    end + 1
}

fn apply_sgr(params: &[char], style: &mut TextStyle) {
    let params: String = params.iter().collect();
    for value in params.split(';') {
        match value.parse::<u16>().unwrap_or(0) {
            0 => *style = TextStyle::default(),
            1 => style.bold = true,
            3 => style.italic = true,
            4 => style.underlined = true,
            7 => style.reversed = true,
            22 => style.bold = false,
            23 => style.italic = false,
            24 => style.underlined = false,
            27 => style.reversed = false,
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn span(content: &str, style: TextStyle) -> StyledSpan {
        StyledSpan {
            content: content.to_string(),
            style,
        }
    }

    #[test]
    fn decodes_plain_text_and_tabs() {
        assert_eq!(
            decode_terminal_text("one\ttwo\n"),
            vec![StyledLine {
                spans: vec![span("one     two", TextStyle::default())],
            }]
        );
    }

    #[test]
    fn decodes_traditional_overstrike_emphasis() {
        let line = decode_terminal_text("N\u{8}N _\u{8}u b\u{8}b\u{8}_\n");
        assert_eq!(line[0].plain_text(), "N u b");
        assert_eq!(
            line[0].spans()[0],
            span(
                "N",
                TextStyle {
                    bold: true,
                    ..TextStyle::default()
                }
            )
        );
        assert_eq!(
            line[0].spans()[2],
            span(
                "u",
                TextStyle {
                    underlined: true,
                    ..TextStyle::default()
                }
            )
        );
        assert_eq!(
            line[0].spans()[4],
            span(
                "b",
                TextStyle {
                    bold: true,
                    underlined: true,
                    ..TextStyle::default()
                }
            )
        );
    }

    #[test]
    fn decodes_ansi_sgr_and_resets() {
        let line = decode_terminal_text("a\u{1b}[1;4mb\u{1b}[0mc\n");
        assert_eq!(line[0].plain_text(), "abc");
        assert_eq!(
            line[0].spans(),
            &[
                span("a", TextStyle::default()),
                span(
                    "b",
                    TextStyle {
                        bold: true,
                        underlined: true,
                        ..TextStyle::default()
                    }
                ),
                span("c", TextStyle::default())
            ]
        );
    }

    #[test]
    fn carriage_return_overwrites_existing_cells() {
        let line = decode_terminal_text("abc\rZ\n");
        assert_eq!(line[0].plain_text(), "Zbc");
    }

    #[test]
    fn system_renderer_preserves_emphasis_when_man_is_available() {
        let available = Command::new("man")
            .arg("-w")
            .arg("man")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if !available {
            return;
        }

        let lines = SystemManRenderer::new()
            .render("man", None, 80)
            .expect("render system man page");
        assert!(lines.iter().flat_map(StyledLine::spans).any(|span| {
            span.style.bold || span.style.underlined || span.style.italic || span.style.reversed
        }));
    }
}
