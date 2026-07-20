//! Language-specific code block compression using tree-sitter ASTs.
//!
//! For supported languages, parses code blocks and removes comments
//! and docstrings using accurate AST knowledge. This is lossless for
//! code semantics — comments and docstrings are not executable code.
//!
//! Falls back to original content for unsupported languages or parse
//! failures.

use tree_sitter::Parser;

/// Configuration for a single language.
struct LangConfig {
    parser: Parser,
    /// Node kinds that are comments (to remove entirely).
    comment_kinds: &'static [&'static str],
    /// Node kinds that are docstrings (to remove entirely).
    /// Docstrings are string literals in expression-statement position.
    docstring_kinds: &'static [&'static str],
}

fn get_config(lang: &str) -> Option<LangConfig> {
    match lang {
        "python" | "py" => {
            let mut p = Parser::new();
            p.set_language(&tree_sitter::Language::from(tree_sitter_python::LANGUAGE)).ok()?;
            Some(LangConfig {
                parser: p,
                comment_kinds: &["comment"],
                docstring_kinds: &["expression_statement"],
            })
        }
        "bash" | "sh" | "shell" | "zsh" => {
            let mut p = Parser::new();
            p.set_language(&tree_sitter::Language::from(tree_sitter_bash::LANGUAGE)).ok()?;
            Some(LangConfig {
                parser: p,
                comment_kinds: &["comment"],
                docstring_kinds: &[],
            })
        }
        "javascript" | "js" | "jsx" => {
            let mut p = Parser::new();
            p.set_language(&tree_sitter::Language::from(tree_sitter_javascript::LANGUAGE)).ok()?;
            Some(LangConfig {
                parser: p,
                comment_kinds: &["comment"],
                docstring_kinds: &[],
            })
        }
        "typescript" | "ts" | "tsx" => {
            let mut p = Parser::new();
            p.set_language(&tree_sitter::Language::from(tree_sitter_typescript::LANGUAGE_TYPESCRIPT)).ok()?;
            Some(LangConfig {
                parser: p,
                comment_kinds: &["comment"],
                docstring_kinds: &[],
            })
        }
        "go" | "golang" => {
            let mut p = Parser::new();
            p.set_language(&tree_sitter::Language::from(tree_sitter_go::LANGUAGE)).ok()?;
            Some(LangConfig {
                parser: p,
                comment_kinds: &["comment"],
                docstring_kinds: &[],
            })
        }
        "java" => {
            let mut p = Parser::new();
            p.set_language(&tree_sitter::Language::from(tree_sitter_java::LANGUAGE)).ok()?;
            Some(LangConfig {
                parser: p,
                comment_kinds: &["comment"],
                docstring_kinds: &[],
            })
        }
        _ => None,
    }
}

/// Compress a code block by removing comments and docstrings using AST.
///
/// Returns the original content for unsupported languages or parse failures.
pub fn compress(code: &str, lang: &str) -> String {
    let Some(mut cfg) = get_config(lang) else {
        return code.to_string();
    };

    let tree = match cfg.parser.parse(code, None) {
        Some(t) => t,
        None => return code.to_string(),
    };

    let root = tree.root_node();
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    collect_removable_ranges(root, &cfg, &mut ranges, false);

    if ranges.is_empty() {
        return code.to_string();
    }

    // Sort in reverse order so removals don't shift positions
    ranges.sort_by(|a, b| b.0.cmp(&a.0));

    let mut result = code.to_string();
    for (start, end) in &ranges {
        // Preserve shebang lines (#!/...) even though bash parser treats them as comments
        let before = &result[..*start];
        let after = &result[*end..];
        if before.ends_with("#!") || before.ends_with("#! ") {
            continue;
        }
        result.replace_range(*start..*end, "");
    }

    // Clean up: collapse runs of blank lines
    let mut cleaned = String::with_capacity(result.len());
    let mut prev_blank = false;
    for line in result.lines() {
        if line.trim().is_empty() {
            if !prev_blank {
                cleaned.push('\n');
                prev_blank = true;
            }
        } else {
            cleaned.push_str(line);
            cleaned.push('\n');
            prev_blank = false;
        }
    }

    cleaned
}

/// Recursively find byte ranges of comments and docstrings to remove.
///
/// `in_docstring_context` tracks whether we're inside a function/class body
/// where a lone string expression is a docstring.
fn collect_removable_ranges(
    node: tree_sitter::Node,
    cfg: &LangConfig,
    ranges: &mut Vec<(usize, usize)>,
    in_docstring_context: bool,
) {
    // Check if this is a comment
    if cfg.comment_kinds.contains(&node.kind()) {
        ranges.push((node.start_byte(), node.end_byte()));
        return;
    }

    // Check if this is a docstring (Python: expression_statement containing a string)
    if in_docstring_context && cfg.docstring_kinds.contains(&node.kind()) {
        // Check if the expression_statement contains only a string
        let mut cursor = node.walk();
        let children: Vec<_> = node.children(&mut cursor).collect();
        if children.len() == 1 {
            let child = children[0];
            let kind = child.kind();
            if kind == "string" || kind == "string_content" {
                ranges.push((node.start_byte(), node.end_byte()));
                return;
            }
        }
    }

    // Determine if children are in a docstring context
    let is_docstring_context = in_docstring_context
        || cfg.structural_parent_kinds()
            .map_or(false, |kinds| kinds.contains(&node.kind()));

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_removable_ranges(child, cfg, ranges, is_docstring_context);
    }
}

impl LangConfig {
    /// Node kinds that create a docstring context (function/class bodies).
    fn structural_parent_kinds(&self) -> Option<&[&'static str]> {
        match self.docstring_kinds {
            _ if self.docstring_kinds.is_empty() => None,
            _ => Some(&["function_definition", "class_definition", "module"]),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_python_comments() {
        let code = "def hello():\n    # this is a comment\n    print(\"hello\")\n    # another comment\n";
        let result = compress(code, "python");
        assert!(!result.contains("this is a comment"));
        assert!(!result.contains("another comment"));
        assert!(result.contains("def hello():"));
        assert!(result.contains("print(\"hello\")"));
    }

    #[test]
    fn test_remove_python_docstring() {
        let code = "def hello(name: str) -> str:\n    \"\"\"Say hello to someone.\"\"\"\n    return f\"Hello, {name}!\"\n";
        let result = compress(code, "python");
        assert!(!result.contains("Say hello to someone"));
        assert!(result.contains("def hello(name: str) -> str:"));
        assert!(result.contains("return f\"Hello, {name}!\""));
    }

    #[test]
    fn test_remove_bash_comments() {
        let code = "# this is a comment\necho \"hello\"\n# another comment\n";
        let result = compress(code, "bash");
        assert!(!result.contains("this is a comment"));
        assert!(!result.contains("another comment"));
        assert!(result.contains("echo \"hello\""));
    }

    #[test]
    fn test_unsupported_language() {
        let code = "some code";
        let result = compress(code, "mermaid");
        assert_eq!(result, code);
    }

    #[test]
    fn test_remove_js_comments() {
        let code = "function hello() {\n    // this is a comment\n    console.log(\"hello\");\n    /* block comment */\n}\n";
        let result = compress(code, "javascript");
        assert!(!result.contains("this is a comment"));
        assert!(!result.contains("block comment"));
        assert!(result.contains("function hello()"));
        assert!(result.contains("console.log(\"hello\")"));
    }

    #[test]
    fn test_preserve_string_with_hash() {
        // Python string containing # should not be affected
        let code = "x = \"# not a comment\"\n# actual comment\n";
        let result = compress(code, "python");
        assert!(result.contains("# not a comment"));
        assert!(!result.contains("actual comment"));
    }
}
