use std::fmt;
use std::io::Read;
use std::process::{Command, Stdio};

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
    ) -> Result<Vec<String>, RenderError>;
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
    ) -> Result<Vec<String>, RenderError> {
        let safe_width = width.max(1).to_string();
        let mut man_cmd = Command::new("man");
        man_cmd.env("MANWIDTH", &safe_width).env("MANPAGER", "cat");

        if let Some(section) = section {
            man_cmd.arg(section);
        }
        man_cmd.arg(name);
        man_cmd.stdout(Stdio::piped());
        man_cmd.stderr(Stdio::piped());

        let mut man_child = man_cmd.spawn()?;
        let man_stdout = man_child
            .stdout
            .take()
            .ok_or_else(|| RenderError::CommandFailed("man stdout unavailable".to_string()))?;
        let mut man_stderr = man_child
            .stderr
            .take()
            .ok_or_else(|| RenderError::CommandFailed("man stderr unavailable".to_string()))?;

        let mut col_child = Command::new("col")
            .arg("-bx")
            .stdin(Stdio::from(man_stdout))
            .stdout(Stdio::piped())
            .spawn()?;

        let mut output = Vec::new();
        if let Some(mut stdout) = col_child.stdout.take() {
            stdout.read_to_end(&mut output)?;
        }

        let man_status = man_child.wait()?;
        if !man_status.success() {
            let mut error_output = Vec::new();
            man_stderr.read_to_end(&mut error_output)?;
            let message = String::from_utf8_lossy(&error_output).trim().to_string();
            return Err(RenderError::CommandFailed(format!(
                "{}",
                if message.is_empty() {
                    format!("man exited with {man_status}")
                } else {
                    message
                }
            )));
        }

        let col_status = col_child.wait()?;
        if !col_status.success() {
            return Err(RenderError::CommandFailed(format!(
                "col exited with {col_status}"
            )));
        }

        let text = String::from_utf8(output)?;
        Ok(text.lines().map(|line| line.to_string()).collect())
    }
}
