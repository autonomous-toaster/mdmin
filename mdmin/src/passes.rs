//! Transform passes for Levels 3 (Structured) and 4 (Ultra).
//!
//! These passes operate on the already-minified output from Levels 1/2,
//! restructuring it into compact formats optimized for LLM token consumption.

/// Apply Level 3 (Structured) transformations to already-minified text.
///
/// Converts flat markdown-like text into TOON-style indented structure:
/// - Section headings become indented keys
/// - Lists are indented under their parent section
/// - Checklists use `+`/`-` notation
#[must_use]
pub fn apply_level_3(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let lines: Vec<&str> = input.lines().collect();

    // Track heading hierarchy for indentation
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];

        if let Some(heading) = parse_heading(line) {
            // Emit heading as indented key
            output.push_str(&format!("{}:\n", &heading));
        } else if let Some(task) = parse_checklist(line) {
            // Convert checklist to +/- notation
            let (checked, text) = task;
            let marker = if checked { "+" } else { "-" };
            output.push_str(&format!("  {marker} {text}\n"));
        } else if line.starts_with("- ") || line.starts_with("* ") {
            // List item — indent under current section
            let text = line.trim_start_matches("- ").trim_start_matches("* ");
            output.push_str(&format!("  - {text}\n"));
        } else if line.is_empty() {
            // Skip blank lines in structured mode
        } else {
            // Regular text — emit as-is
            output.push_str(line);
            output.push('\n');
        }

        i += 1;
    }

    output
}

/// Apply Level 4 (Ultra) transformations to already-minified text.
///
/// Converts to ultra-compact single-line grouped format:
/// - Sections use `key{...}` brace notation
/// - Lists are inline within braces
/// - Minimal whitespace
#[must_use]
pub fn apply_level_4(input: &str) -> String {
    let mut output = String::with_capacity(input.len() / 2);
    let lines: Vec<&str> = input.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        if let Some(heading) = parse_heading(line) {
            // Collect content until next heading or end
            let mut content: Vec<&str> = Vec::new();
            i += 1;
            while i < lines.len() {
                let next = lines[i];
                if parse_heading(next).is_some() {
                    break;
                }
                let trimmed = next.trim();
                if !trimmed.is_empty() {
                    content.push(trimmed);
                }
                i += 1;
            }

            if content.is_empty() {
                output.push_str(&format!("{heading}{{}}\n"));
            } else {
                output.push_str(&format!("{heading}{{{}}}\n", content.join(" ")));
            }
        } else if let Some(task) = parse_checklist(line) {
            let (checked, text) = task;
            let marker = if checked { "+" } else { "-" };
            output.push_str(&format!("{marker}{text}"));
            // Check if next line is also a checklist item
            if i + 1 < lines.len() && parse_checklist(lines[i + 1]).is_some() {
                output.push(' ');
            } else {
                output.push('\n');
            }
            i += 1;
        } else if line.starts_with("- ") || line.starts_with("* ") {
            let text = line.trim_start_matches("- ").trim_start_matches("* ");
            output.push_str(&format!("-{text}"));
            if i + 1 < lines.len()
                && (lines[i + 1].starts_with("- ") || lines[i + 1].starts_with("* "))
            {
                output.push(' ');
            } else {
                output.push('\n');
            }
            i += 1;
        } else if !line.trim().is_empty() {
            output.push_str(line.trim());
            output.push('\n');
            i += 1;
        } else {
            i += 1;
        }
    }

    output
}

/// Parse a heading line like `title:` or `section:`.
fn parse_heading(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if let Some(stripped) = trimmed.strip_suffix(':') {
        if !stripped.is_empty()
            && stripped
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ')
        {
            return Some(stripped.to_string());
        }
    }
    None
}

/// Parse a checklist item like `+ Login` or `- Logout`.
fn parse_checklist(line: &str) -> Option<(bool, String)> {
    let trimmed = line.trim();
    if let Some(text) = trimmed.strip_prefix("+ ") {
        Some((true, text.to_string()))
    } else if let Some(text) = trimmed.strip_prefix("- ") {
        Some((false, text.to_string()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_3_heading_indentation() {
        let input = "why:\nLLM context windows are expensive.\n";
        let result = apply_level_3(input);
        assert!(result.contains("why:\n"), "heading preserved");
    }

    #[test]
    fn test_level_3_checklist() {
        let input = "+ Login\n- Logout\n";
        let result = apply_level_3(input);
        assert!(result.contains("+ Login"), "checked item");
        assert!(result.contains("- Logout"), "unchecked item");
    }

    #[test]
    fn test_level_4_brace_grouping() {
        let input = "auth:\n  jwt\n  oauth\n";
        let result = apply_level_4(input);
        assert!(result.contains("auth{"), "brace group opened");
        assert!(result.contains("}"), "brace group closed");
    }

    #[test]
    fn test_level_4_checklist_compact() {
        let input = "+ Login\n- Logout\n";
        let result = apply_level_4(input);
        assert!(result.contains("+Login"), "compact checked");
        assert!(result.contains("-Logout"), "compact unchecked");
    }

    #[test]
    fn test_parse_heading() {
        assert_eq!(parse_heading("title:"), Some("title".to_string()));
        assert_eq!(parse_heading("what changes:"), Some("what changes".to_string()));
        assert_eq!(parse_heading("not a heading"), None);
        assert_eq!(parse_heading(""), None);
    }

    #[test]
    fn test_parse_checklist() {
        assert_eq!(
            parse_checklist("+ Login"),
            Some((true, "Login".to_string()))
        );
        assert_eq!(
            parse_checklist("- Logout"),
            Some((false, "Logout".to_string()))
        );
        assert_eq!(parse_checklist("plain text"), None);
    }
}
