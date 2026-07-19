//! Core minification engine — parse, transform, emit.

use std::fmt;

use tree_sitter::{Node, Parser, Tree};

use crate::dictionary;
use crate::grammar;
use crate::passes;
use crate::{CodeBlockMode, Config, Level, MinifyResult};

/// Errors that can occur during minification.
#[derive(Debug)]
pub enum Error {
    /// Tree-sitter grammar not available.
    Grammar(String),
    /// Internal error during tree walking.
    Internal(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Grammar(msg) => write!(f, "grammar error: {msg}"),
            Self::Internal(msg) => write!(f, "internal error: {msg}"),
        }
    }
}

impl std::error::Error for Error {}

/// A single edit operation: delete or replace a byte range.
#[derive(Debug, Clone)]
struct Edit {
    start: usize,
    end: usize,
    replacement: String,
}

/// The minifier entry point.
pub struct Minifier {
    config: Config,
    parser: Parser,
}

impl Minifier {
    /// Create a new minifier with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns `Error::Grammar` if the tree-sitter markdown grammar cannot be loaded.
    pub fn new(config: &Config) -> Result<Self, Error> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_markdown_fork::language())
            .map_err(|e| Error::Grammar(e.to_string()))?;
        Ok(Self {
            config: config.clone(),
            parser,
        })
    }

    /// Minify the input markdown string.
    ///
    /// # Errors
    ///
    /// Returns `Error::Internal` if tree walking encounters an unexpected state.
    pub fn minify(&mut self, input: &str) -> Result<MinifyResult, Error> {
        let input_tokens = estimate_tokens(input);

        // Level 0: no-op, skip parse entirely (but still apply grammar strip if enabled)
        if self.config.level == Level::Off {
            let output = if let Some(level) = self.config.grammar_level() {
                grammar::strip_with_level(input, level)
            } else {
                input.to_string()
            };
            let output_tokens = estimate_tokens(&output);
            let savings_pct = if input_tokens > 0 {
                ((input_tokens.saturating_sub(output_tokens)) as f64 / input_tokens as f64) * 100.0
            } else {
                0.0
            };
            return Ok(MinifyResult {
                output,
                input_tokens,
                output_tokens,
                savings_pct,
            });
        }

        // Parse
        let tree = self
            .parser
            .parse(input, None)
            .ok_or_else(|| Error::Internal("parse returned None".to_string()))?;

        // Walk the CST and collect edits
        let edits = collect_edits(&tree, input, &self.config)?;

        // Apply edits to produce output
        let output = apply_edits(input, &edits);

        // Apply L2 text-level passes: nested list flattening, inline tiny sections
        let output = if self.config.level as u8 >= 2 {
            let output = flatten_nested_lists(&output);
            let output = inline_tiny_sections(&output);
            let output = strip_url_protocol(&output);
            compress_list_items(&output)
        } else {
            output
        };

        // Apply grammar stripping if enabled (after structural transforms, before L3/L4)
        let output = if let Some(level) = self.config.grammar_level() {
            grammar::strip_with_level(&output, level)
        } else {
            output
        };

        // Apply local dictionary compression if enabled (after grammar strip, before L3/L4)
        let output = if self.config.dictionary {
            dictionary::compress(&output)
        } else {
            output
        };

        // Apply Level 3 or 4 structural passes on the already-minified text
        // Size guard: if restructuring + legend doesn't reduce size, keep original
        let pre_restructure = output.clone();

        // Always compute L3 as baseline (L4 must beat it)
        let l3_restructured = passes::apply_level_3(&output);
        let has_lists = l3_restructured.lines().any(|l| {
            let t = l.trim();
            t.starts_with("- ") || t.starts_with("+ ") || t.starts_with("! ")
        });
        let l3_with_legend = if self.config.legend && has_lists {
            format!("[mdmin: -=list +=done !=todo]\n{l3_restructured}")
        } else {
            l3_restructured
        };
        let l3_best = if l3_with_legend.len() < pre_restructure.len() {
            l3_with_legend
        } else {
            pre_restructure.clone()
        };

        let output = match self.config.level {
            Level::Structured => l3_best,
            Level::Ultra => {
                let l4_restructured = passes::apply_level_4(&output);
                let l4_with_legend = if self.config.legend && has_lists {
                    format!("[mdmin: -=list +=done !=todo]\n{l4_restructured}")
                } else {
                    l4_restructured
                };
                // L4 must beat or tie L3 (which already beat pre_restructure or fell back)
                if l4_with_legend.len() <= l3_best.len() {
                    l4_with_legend
                } else {
                    l3_best
                }
            }
            _ => output,
        };
        let output_tokens = estimate_tokens(&output);

        let savings_pct = if input_tokens > 0 {
            ((input_tokens.saturating_sub(output_tokens)) as f64 / input_tokens as f64) * 100.0
        } else {
            0.0
        };

        Ok(MinifyResult {
            output,
            input_tokens,
            output_tokens,
            savings_pct,
        })
    }
}

/// Estimate token count (rough: len / 4).
#[must_use]
pub fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Walk the CST and collect all edits for the given level.
fn collect_edits(tree: &Tree, source: &str, config: &Config) -> Result<Vec<Edit>, Error> {
    let mut edits: Vec<Edit> = Vec::new();
    let root = tree.root_node();
    walk_node(&root, source, config, &mut edits)?;
    edits.sort_by_key(|e| e.start);
    Ok(edits)
}

/// Recursively walk a node and collect edits.
fn walk_node(
    node: &Node,
    source: &str,
    config: &Config,
    edits: &mut Vec<Edit>,
) -> Result<(), Error> {
    let kind = node.kind();

    match kind {
        // ── Level 1+ : Strip decorative formatting ──────────────────────
        "emphasis" if config.level as u8 >= 1 => {
            // Strip the single * marker on each side
            let start = node.start_byte();
            let end = node.end_byte();
            if end > start + 2 {
                edits.push(Edit {
                    start,
                    end: start + 1,
                    replacement: String::new(),
                });
                edits.push(Edit {
                    start: end - 1,
                    end,
                    replacement: String::new(),
                });
            }
            return Ok(());
        }

        "strong_emphasis" if config.level as u8 >= 1 => {
            // Strip the double ** marker on each side
            let start = node.start_byte();
            let end = node.end_byte();
            if end > start + 4 {
                edits.push(Edit {
                    start,
                    end: start + 2,
                    replacement: String::new(),
                });
                edits.push(Edit {
                    start: end - 2,
                    end,
                    replacement: String::new(),
                });
            }
            return Ok(());
        }

        "strikethrough" if config.level as u8 >= 1 => {
            // Strip the ~~ marker on each side
            let start = node.start_byte();
            let end = node.end_byte();
            if end > start + 4 {
                edits.push(Edit {
                    start,
                    end: start + 2,
                    replacement: String::new(),
                });
                edits.push(Edit {
                    start: end - 2,
                    end,
                    replacement: String::new(),
                });
            }
            return Ok(());
        }

        "thematic_break" if config.level as u8 >= 1 => {
            // Delete the entire HR line (including trailing newline)
            let end = if source.as_bytes().get(node.end_byte()) == Some(&b'\n') {
                node.end_byte() + 1
            } else {
                node.end_byte()
            };
            edits.push(Edit {
                start: node.start_byte(),
                end,
                replacement: String::new(),
            });
            return Ok(());
        }

        "html_comment" | "html_block" if config.level as u8 >= 1 => {
            // Delete the entire HTML comment/block (includes <!-- and -->)
            edits.push(Edit {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement: String::new(),
            });
            return Ok(());
        }

        // ── Level 2+ : Semantic compression ───────────────────────────
        "atx_heading" if config.level as u8 >= 2 => {
            return handle_atx_heading(node, source, config, edits);
        }

        "table" if config.level as u8 >= 2 => {
            return handle_table(node, source, config, edits);
        }

        // ── Level 3+ : Checklist conversion ───────────────────────────
        "task_list_item" if config.level as u8 >= 3 => {
            return handle_task_list_item(node, source, edits);
        }

        // ── Code blocks ────────────────────────────────────────────────
        "fenced_code_block" => {
            return handle_code_block(node, source, config, edits);
        }

        _ => {}
    }

    // Recurse into children
    let mut cursor = node.walk();
    if cursor.goto_first_child() {
        loop {
            let child = cursor.node();
            walk_node(&child, source, config, edits)?;
            if !cursor.goto_next_sibling() {
                break;
            }
        }
    }

    Ok(())
}

/// Handle an ATX heading (e.g., `## Title`).
fn handle_atx_heading(
    node: &Node,
    source: &str,
    config: &Config,
    edits: &mut Vec<Edit>,
) -> Result<(), Error> {
    let children: Vec<Node> = node.children(&mut node.walk()).collect();

    // Find the heading marker (e.g., `##`) and the content
    let marker = children.iter().find(|c| c.kind().contains("marker"));
    let content = children.iter().find(|c| c.kind() == "heading_content");

    if let (Some(_marker), Some(content)) = (marker, content) {
        let heading_text = source[content.start_byte()..content.end_byte()].trim();

        if !heading_text.is_empty() {
            match config.level {
                Level::Medium | Level::Structured | Level::Ultra => {
                    // `## Title` → `title:` (no trailing newline — the original
                    // newline after the heading node is preserved by the edit logic)
                    let normalized = format!("{}:", heading_text.to_lowercase());
                    edits.push(Edit {
                        start: node.start_byte(),
                        end: node.end_byte(),
                        replacement: normalized,
                    });
                }
                _ => {}
            }
        }
    }

    Ok(())
}

/// Handle a table node.
fn handle_table(
    node: &Node,
    source: &str,
    _config: &Config,
    edits: &mut Vec<Edit>,
) -> Result<(), Error> {
    let children: Vec<Node> = node.children(&mut node.walk()).collect();

    // Find table rows (header row + delimiter row + body rows)
    let rows: Vec<&Node> = children
        .iter()
        .filter(|c| {
            let k = c.kind();
            k == "table_header_row" || k == "table_data_row"
        })
        .collect();

    if rows.is_empty() {
        return Ok(());
    }

    // Parse header cells
    let header_row = rows[0];
    let header_cells: Vec<String> = header_row
        .children(&mut header_row.walk())
        .filter(|c| c.kind() == "table_cell" || c.kind() == "table_header_cell")
        .map(|c| source[c.start_byte()..c.end_byte()].trim().to_string())
        .collect();

    // Parse body rows
    let mut lines: Vec<String> = Vec::new();
    for row in rows.iter().skip(1) {
        let row_kind = row.kind();
        if row_kind == "table_data_row" {
            let cells: Vec<String> = row
                .children(&mut row.walk())
                .filter(|c| c.kind() == "table_cell")
                .map(|c| source[c.start_byte()..c.end_byte()].trim().to_string())
                .collect();

            if cells.len() == 1 && header_cells.len() == 1 {
                // Single column: keep column:value format (adds context, no ambiguity)
                lines.push(format!("{}:{}", header_cells[0], cells[0]));
            } else {
                // Multi-column: positional format (space-separated values)
                lines.push(cells.join(" "));
            }
        }
    }

    if lines.is_empty() {
        return Ok(());
    }

    let replacement = lines.join("\n") + "\n";

    // Size guard: skip compression if it would increase byte count
    let original_size = node.end_byte() - node.start_byte();
    if replacement.len() > original_size {
        return Ok(());
    }

    edits.push(Edit {
        start: node.start_byte(),
        end: node.end_byte(),
        replacement,
    });

    Ok(())
}

/// Handle a fenced code block.
fn handle_code_block(
    node: &Node,
    source: &str,
    config: &Config,
    edits: &mut Vec<Edit>,
) -> Result<(), Error> {
    match config.code_blocks {
        CodeBlockMode::Preserve => {
            // Leave entirely unchanged — don't recurse into children
        }
        CodeBlockMode::CompressWhitespace => {
            // Find the code fence content and collapse blank lines
            let children: Vec<Node> = node.children(&mut node.walk()).collect();
            if let Some(content) = children.iter().find(|c| c.kind() == "code_fence_content") {
                let text = &source[content.start_byte()..content.end_byte()];
                // Strip leading/trailing blank lines, then collapse internal runs
                let stripped = strip_leading_trailing_blank_lines(text);
                let collapsed = collapse_blank_lines(&stripped);
                if collapsed != text {
                    edits.push(Edit {
                        start: content.start_byte(),
                        end: content.end_byte(),
                        replacement: collapsed,
                    });
                }
            }
        }
    }
    Ok(())
}

/// Handle a task list item (checklist).
/// Converts `- [x] text` to `+ text` and `- [ ] text` to `! text`.
fn handle_task_list_item(
    node: &Node,
    source: &str,
    edits: &mut Vec<Edit>,
) -> Result<(), Error> {
    let children: Vec<Node> = node.children(&mut node.walk()).collect();

    // Find the paragraph child which contains "[x] text" or "[ ] text"
    if let Some(para) = children.iter().find(|c| c.kind() == "paragraph") {
        let text = &source[para.start_byte()..para.end_byte()];

        // Check for checked or unchecked
        if let Some(rest) = text.strip_prefix("[x] ") {
            let replacement = format!("+ {rest}");
            edits.push(Edit {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement,
            });
        } else if let Some(rest) = text.strip_prefix("[ ] ") {
            let replacement = format!("! {rest}");
            edits.push(Edit {
                start: node.start_byte(),
                end: node.end_byte(),
                replacement,
            });
        }
    }

    Ok(())
}

/// Strip leading and trailing blank lines from text.
fn strip_leading_trailing_blank_lines(s: &str) -> String {
    let lines: Vec<&str> = s.lines().collect();
    if lines.is_empty() {
        return s.to_string();
    }

    let mut start = 0;
    while start < lines.len() && lines[start].trim().is_empty() {
        start += 1;
    }

    let mut end = lines.len();
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    if start >= end {
        return String::new();
    }

    lines[start..end].join("\n")
}

/// Collapse runs of 2+ consecutive newlines to a single newline.
fn collapse_blank_lines(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_was_newline = false;

    for ch in s.chars() {
        if ch == '\n' {
            if prev_was_newline {
                continue;
            }
            prev_was_newline = true;
        } else {
            prev_was_newline = false;
        }
        result.push(ch);
    }

    result
}

/// Flatten nested lists: `- Parent\n  - Child1\n  - Child2` → `Parent: Child1, Child2`.
fn flatten_nested_lists(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if this is a parent list item with children on next lines
        if (line.starts_with("- ") || line.starts_with("* ")) && !line.starts_with("  ") {
            // Look ahead for indented children
            let parent_text = line.trim_start_matches("- ").trim_start_matches("* ");
            let mut children: Vec<&str> = Vec::new();
            let mut j = i + 1;

            while j < lines.len() {
                let next = lines[j];
                if next.starts_with("  - ") || next.starts_with("  * ") {
                    let child_text = next.trim().trim_start_matches("- ").trim_start_matches("* ");
                    children.push(child_text);
                    j += 1;
                } else if next.trim().is_empty() {
                    j += 1;
                } else {
                    break;
                }
            }

            if !children.is_empty() {
                // Flatten: Parent: Child1, Child2
                result.push_str(&format!("{}: {}\n", parent_text, children.join(", ")));
                i = j;
                continue;
            }
        }

        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    result
}

/// Inline tiny sections: `title:\nshort text` → `title: short text`.
fn inline_tiny_sections(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Check if this is a heading line (ends with :)
        if line.ends_with(':') && !line.starts_with(' ') && !line.starts_with('@') {
            // Only inline if the NEXT line (no blank lines between) is a short paragraph
            if i + 1 < lines.len() {
                let next = lines[i + 1].trim();
                // Only inline if paragraph is short (≤50 chars), not a list, not a heading
                if !next.is_empty()
                    && next.len() <= 50
                    && !next.starts_with("- ")
                    && !next.starts_with("* ")
                    && !next.ends_with(':')
                {
                    // Merge: "title" + ": short text"
                    let heading = line.trim_end_matches(':');
                    result.push_str(&format!("{}: {}\n", heading, next));
                    i += 2;
                    continue;
                }
            }
        }

        result.push_str(line);
        result.push('\n');
        i += 1;
    }

    result
}

/// Strip protocol from URLs: https://example.com/path → example.com/path
/// Safe because LLMs can infer the protocol from context.
fn strip_url_protocol(text: &str) -> String {
    text.replace("https://", "")
        .replace("http://", "")
}

/// Merge consecutive short list items into comma-separated lines.
/// E.g.: "- foo\n- bar\n- baz" → "- foo, bar, baz"
/// Saves newline + indent per merged item.
fn compress_list_items(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut lines = text.lines().peekable();
    
    while let Some(line) = lines.next() {
        // Check if this is a short list item
        let trimmed = line.trim();
        if let Some(content) = trimmed.strip_prefix("- ").or_else(|| trimmed.strip_prefix("* ")) {
            if content.len() <= 50 && !content.contains(',') {
                // Look ahead for consecutive short items with same indent
                let indent = &line[..line.len() - trimmed.len()];
                let mut merged = vec![content.to_string()];
                
                while let Some(next) = lines.peek() {
                    let next_trimmed = next.trim();
                    if let Some(next_content) = next_trimmed.strip_prefix("- ").or_else(|| next_trimmed.strip_prefix("* ")) {
                        let next_indent = &next[..next.len() - next_trimmed.len()];
                        if next_indent == indent && next_content.len() <= 50 && !next_content.contains(',') {
                            merged.push(next_content.to_string());
                            lines.next();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                
                if merged.len() >= 2 {
                    result.push_str(&format!("{}- {}\n", indent, merged.join(", ")));
                } else {
                    result.push_str(line);
                    result.push('\n');
                }
            } else {
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }
    
    result
}

/// Apply a sorted list of edits to the source text.
fn apply_edits(source: &str, edits: &[Edit]) -> String {
    if edits.is_empty() {
        return source.to_string();
    }

    let mut result = String::with_capacity(source.len());
    let mut pos = 0;

    for edit in edits {
        // Copy unchanged text before this edit
        if edit.start > pos {
            result.push_str(&source[pos..edit.start]);
        }
        // Insert replacement (empty for deletions)
        result.push_str(&edit.replacement);
        pos = edit.end;
    }

    // Copy remaining text after last edit
    if pos < source.len() {
        result.push_str(&source[pos..]);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Level;

    #[test]
    fn test_level_0_noop() {
        let input = "# Hello\n\nSome **bold** text.\n";
        let config = Config::new(Level::Off);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert_eq!(result.output, input);
        assert_eq!(result.savings_pct, 0.0);
    }

    #[test]
    fn test_level_1_strips_bold() {
        let input = "**bold**";
        let config = Config::new(Level::Light);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert_eq!(result.output, "bold");
    }

    #[test]
    fn test_level_1_strips_italic() {
        let input = "*italic*";
        let config = Config::new(Level::Light);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert_eq!(result.output, "italic");
    }

    #[test]
    fn test_level_1_strips_hr() {
        let input = "before\n\n---\n\nafter";
        let config = Config::new(Level::Light);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert!(!result.output.contains("---"), "HR should be removed, got: {:?}", result.output);
    }

    #[test]
    fn test_level_1_strips_html_comment() {
        let input = "before<!-- comment -->after";
        let config = Config::new(Level::Light);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert_eq!(result.output, "beforeafter");
    }

    #[test]
    fn test_level_2_normalizes_heading() {
        let input = "## Installation\n\nSome text.\n";
        let config = Config::new(Level::Medium);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert!(
            result.output.contains("installation:"),
            "heading should be normalized, got: {:?}",
            result.output
        );
    }

    #[test]
    fn test_code_block_preserve() {
        let input = "```rust\nfn main() {}\n```\n";
        let config = Config::new(Level::Light).with_code_blocks(CodeBlockMode::Preserve);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert!(result.output.contains("```rust"), "code fence preserved");
    }

    #[test]
    fn test_estimate_tokens() {
        assert_eq!(estimate_tokens("hello"), 1);
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens(&"a".repeat(100)), 25);
    }

    #[test]
    fn test_roundtrip_unchanged() {
        let input = "# Title\n\nA paragraph.\n\n- item 1\n- item 2\n";
        let config = Config::new(Level::Light);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        // L1 should preserve structure, just strip decoration
        assert!(result.output.contains("Title"), "title preserved");
        assert!(result.output.contains("item 1"), "list items preserved");
    }

    #[test]
    fn test_level_3_checklist_conversion() {
        let input = "- [x] Login\n- [ ] Logout\n";
        let config = Config::new(Level::Structured);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert!(result.output.contains("+ Login"), "checked item should use +");
        assert!(result.output.contains("! Logout"), "unchecked item should use !");
        assert!(!result.output.contains("[x]"), "no markdown checkbox syntax");
    }

    #[test]
    fn test_level_4_brace_format() {
        let input = "# Title\n\nContent.\n";
        let config = Config::new(Level::Ultra);
        let mut minifier = Minifier::new(&config).unwrap();
        let result = minifier.minify(input).unwrap();
        assert!(result.output.contains("title{"), "should use brace notation");
        assert!(result.output.contains("}"), "should close brace");
    }
}
















