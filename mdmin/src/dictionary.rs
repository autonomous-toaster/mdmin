//! Targeted dictionary compression — find long repeated strings (paths, URLs,
//! identifiers) and replace them with short `@N` references.
//!
//! Dictionary emitted at top of output:
//! ```text
//! @dict:
//!   @1: /very/long/path/to/file.rs
//!   @2: https://api.example.com/v2/users/123
//! ```

use std::collections::HashMap;

use regex_lite::Regex;

/// Minimum occurrences for compression to be worthwhile.
const MIN_OCCURRENCES: usize = 2;

/// Apply targeted dictionary compression to text.
///
/// Finds long repeated paths, URLs, and identifiers, replaces them with `@N`
/// references, and prepends a dictionary header.
#[must_use]
pub fn compress(text: &str) -> String {
    // Find candidates
    let candidates = find_candidates(text);
    if candidates.is_empty() {
        return text.to_string();
    }

    // Build dictionary: map string -> @N
    let mut dict: HashMap<&str, String> = HashMap::new();
    let mut entries: Vec<String> = Vec::new();

    for (i, candidate) in candidates.iter().enumerate() {
        let key = format!("@{}", i + 1);
        dict.insert(candidate.string, key.clone());
        entries.push(format!("  {key}: {}", candidate.string));
    }

    if entries.is_empty() {
        return text.to_string();
    }

    // Build dictionary header
    let header = format!("@dict:\n{}\n", entries.join("\n"));

    // Replace occurrences (longest first to avoid partial replacements)
    let mut result = text.to_string();
    let mut sorted: Vec<&str> = candidates.iter().map(|c| c.string).collect();
    sorted.sort_by(|a, b| b.len().cmp(&a.len()));

    for s in &sorted {
        if let Some(replacement) = dict.get(*s) {
            result = result.replace(s, replacement);
        }
    }

    format!("{header}{result}")
}

/// A candidate for dictionary compression: (string, occurrence_count).
struct Candidate<'a> {
    string: &'a str,
    count: usize,
}

/// Find long repeated strings using regex patterns.
fn find_candidates(text: &str) -> Vec<Candidate<'_>> {
    let mut candidates: Vec<Candidate> = Vec::new();

    // Pattern 1: Long file paths (/path/to/file.rs)
    if let Ok(re) = Regex::new(r"/[a-zA-Z0-9_/.\-]{30,}") {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for m in re.find_iter(text) {
            *seen.entry(m.as_str()).or_insert(0) += 1;
        }
        for (s, count) in &seen {
            if *count >= MIN_OCCURRENCES {
                candidates.push(Candidate { string: s, count: *count });
            }
        }
    }

    // Pattern 2: Long URLs (https://...)
    if let Ok(re) = Regex::new(r"https?://[^\s)]{30,}") {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for m in re.find_iter(text) {
            *seen.entry(m.as_str()).or_insert(0) += 1;
        }
        for (s, count) in &seen {
            if *count >= MIN_OCCURRENCES {
                candidates.push(Candidate { string: s, count: *count });
            }
        }
    }

    // Pattern 3: Long identifiers (snake_case or CamelCase, 20+ chars)
    if let Ok(re) = Regex::new(r"[a-zA-Z_][a-zA-Z0-9_]{19,}") {
        let mut seen: HashMap<&str, usize> = HashMap::new();
        for m in re.find_iter(text) {
            *seen.entry(m.as_str()).or_insert(0) += 1;
        }
        for (s, count) in &seen {
            if *count >= MIN_OCCURRENCES {
                candidates.push(Candidate { string: s, count: *count });
            }
        }
    }

    // Deduplicate by string
    let mut dedup: HashMap<&str, usize> = HashMap::new();
    for c in &candidates {
        let entry = dedup.entry(c.string).or_insert(0);
        *entry += c.count;
    }

    // Filter by net savings: (len * count) - (len + 2*count) > 0
    let mut result: Vec<Candidate> = dedup
        .into_iter()
        .filter(|(s, count)| {
            let gross = s.len() * count;
            let overhead = s.len() + 2 * count; // dict entry + references
            gross > overhead
        })
        .map(|(s, count)| Candidate { string: s, count })
        .collect();

    // Sort by net savings descending
    result.sort_by(|a, b| {
        let a_save = a.string.len() * a.count - (a.string.len() + 2 * a.count);
        let b_save = b.string.len() * b.count - (b.string.len() + 2 * b.count);
        b_save.cmp(&a_save)
    });

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_compression() {
        let text = "use /very/long/path/to/some/file.rs and also /very/long/path/to/some/file.rs";
        let result = compress(text);
        assert!(result.starts_with("@dict:"), "should start with dictionary header");
        assert!(result.contains("@1"), "should have reference @1");
        // Path should appear in dictionary header but @1 in content
        assert!(result.matches("@1").count() >= 2, "@1 should appear in header and content");
    }

    #[test]
    fn test_url_compression() {
        let text = "see https://api.example.com/v2/users/123/profile and https://api.example.com/v2/users/123/profile";
        let result = compress(text);
        assert!(result.contains("@dict:"), "should have dictionary header");
        assert!(result.contains("@1"), "should have reference");
    }

    #[test]
    fn test_single_occurrence_not_compressed() {
        let text = "just one /very/long/path/to/some/file.rs here";
        let result = compress(text);
        assert!(!result.contains("@dict:"), "no dictionary for single occurrence");
    }

    #[test]
    fn test_empty_text() {
        let result = compress("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_no_long_strings() {
        let result = compress("short text only");
        assert_eq!(result, "short text only");
    }
}

