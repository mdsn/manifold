use std::process::{Command, Stdio};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgsInterpretation {
    SectionAndPages { section: String, pages: Vec<String> },
    Pages(Vec<String>),
}

#[derive(Debug)]
pub enum ValidationError {
    Io(std::io::Error),
}

impl From<std::io::Error> for ValidationError {
    fn from(value: std::io::Error) -> Self {
        ValidationError::Io(value)
    }
}

pub fn classify_args<S: AsRef<str>>(args: &[S]) -> Result<ArgsInterpretation, ValidationError> {
    let Some((first, rest)) = args.split_first() else {
        return Ok(ArgsInterpretation::Pages(Vec::new()));
    };

    if rest.is_empty() {
        return Ok(ArgsInterpretation::Pages(vec![first.as_ref().to_string()]));
    }

    let section_candidate = first.as_ref();
    let pages: Vec<String> = rest.iter().map(|value| value.as_ref().to_string()).collect();

    let mut all_in_section = true;
    for page in &pages {
        if !man_page_exists_in_section(section_candidate, page)? {
            all_in_section = false;
            break;
        }
    }

    if all_in_section {
        Ok(ArgsInterpretation::SectionAndPages {
            section: section_candidate.to_string(),
            pages,
        })
    } else {
        let mut all_pages = Vec::with_capacity(args.len());
        all_pages.push(section_candidate.to_string());
        all_pages.extend(pages);
        Ok(ArgsInterpretation::Pages(all_pages))
    }
}

fn man_page_exists_in_section(section: &str, page: &str) -> Result<bool, ValidationError> {
    let status = Command::new("man")
        .arg("-w")
        .arg("-S")
        .arg(section)
        .arg(page)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    Ok(status.success())
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn classifies_section_and_pages_when_all_pages_exist() {
        if !man_available() {
            return;
        }

        let args = ["1", "man"];
        let result = classify_args(&args).expect("classification failed");

        assert_eq!(
            result,
            ArgsInterpretation::SectionAndPages {
                section: "1".to_string(),
                pages: vec!["man".to_string()],
            }
        );
    }

    #[test]
    fn falls_back_to_pages_when_section_is_invalid() {
        if !man_available() {
            return;
        }

        let args = ["notasection", "man"];
        let result = classify_args(&args).expect("classification failed");

        assert_eq!(
            result,
            ArgsInterpretation::Pages(vec!["notasection".to_string(), "man".to_string()])
        );
    }

    #[test]
    fn falls_back_to_pages_when_any_page_missing_in_section() {
        if !man_available() {
            return;
        }

        let args = ["1", "man", "definitelynotapage"];
        let result = classify_args(&args).expect("classification failed");

        assert_eq!(
            result,
            ArgsInterpretation::Pages(vec![
                "1".to_string(),
                "man".to_string(),
                "definitelynotapage".to_string()
            ])
        );
    }

    #[test]
    fn classifies_single_argument_as_pages() {
        let args = ["man"];
        let result = classify_args(&args).expect("classification failed");

        assert_eq!(
            result,
            ArgsInterpretation::Pages(vec!["man".to_string()])
        );
    }
}
