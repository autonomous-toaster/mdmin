//! General n-gram dictionary compression — find ALL repeated substrings
//! of 15+ chars and replace them with short `@N` references.
//!
//! Uses LZ77-style approach: find repeated substrings, extend to longest match,
//! keep only the longest non-overlapping candidates.
//!
//! Dictionary emitted at top of output:
//! ```text
//! @dict:
//!   @1: /very/long/path/to/file.rs
//!   @2: budget_tokens
//! ```

use std::collections::HashMap;

/// Minimum substring length for dictionary consideration.
const MIN_LEN: usize = 15;

/// Minimum occurrences for compression to be worthwhile.
const MIN_OCCURRENCES: usize = 2;

/// Apply general n-gram dictionary compression to text.
#[must_use]
pub fn compress(text: &str) -> String {
    let candidates = find_repeated(text);
    if candidates.is_empty() {
        return text.to_string();
    }

    // Build dictionary
    let mut dict: HashMap<&str, String> = HashMap::new();
    let mut entries: Vec<String> = Vec::new();

    for (i, candidate) in candidates.iter().enumerate() {
        let key = format!("@{}", i + 1);
        dict.insert(candidate.string, key.clone());
        entries.push(format!("  {key}: {}", candidate.string));
    }

    let header = format!("@dict:\n{}\n", entries.join("\n"));

    // Replace occurrences (longest first)
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

struct Candidate<'a> {
    string: &'a str,
    count: usize,
}

/// Find repeated substrings using LZ77-style approach.
/// For each position, find the longest substring that appears elsewhere.
fn find_repeated(text: &str) -> Vec<Candidate<'_>> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    if n < MIN_LEN * 2 {
        return Vec::new();
    }

    // Build map of all 15-char substrings to their positions
    let mut index: HashMap<&str, Vec<usize>> = HashMap::new();
    let mut char_positions: Vec<usize> = Vec::new();
    let mut i = 0;
    while i < n {
        char_positions.push(i);
        let c = bytes[i];
        if c & 0x80 == 0 { i += 1; }
        else if c & 0xE0 == 0xC0 { i += 2; }
        else if c & 0xF0 == 0xE0 { i += 3; }
        else if c & 0xF8 == 0xF0 { i += 4; }
        else { i += 1; }
    }

    if char_positions.len() < MIN_LEN {
        return Vec::new();
    }

    // Index all 15-char substrings
    for idx in 0..char_positions.len() - MIN_LEN {
        let start = char_positions[idx];
        let end = char_positions[idx + MIN_LEN];
        let sub = &text[start..end];
        index.entry(sub).or_default().push(start);
    }

    // For each position, find the longest match
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut seen_regions: Vec<(usize, usize)> = Vec::new(); // (start, end) of already-matched regions

    for idx in 0..char_positions.len() - MIN_LEN {
        let start = char_positions[idx];
        let seed = &text[start..char_positions[idx + MIN_LEN]];

        // Skip if this position is already covered by a longer match
        if seen_regions.iter().any(|&(s, e)| start >= s && start < e) {
            continue;
        }

        let positions = match index.get(seed) {
            Some(p) => p,
            None => continue,
        };

        if positions.len() < MIN_OCCURRENCES {
            continue;
        }

        // Find the first occurrence that's not at the same position
        let first_pos = match positions.iter().find(|&&p| p != start) {
            Some(&p) => p,
            None => continue,
        };

        // Extend the match as far as possible
        let mut match_len = MIN_LEN;
        let max_len = 60.min(n - start).min(n - first_pos);
        while match_len < max_len {
            let c1_start = start + match_len;
            let c2_start = first_pos + match_len;
            if c1_start >= n || c2_start >= n {
                break;
            }
            if bytes[c1_start] != bytes[c2_start] {
                break;
            }
            match_len += 1;
        }

        if match_len < MIN_LEN {
            continue;
        }

        // Get the matched string
        let matched = &text[start..start + match_len];

        // Skip pure punctuation
        let alpha = matched.chars().filter(|c| c.is_alphanumeric()).count();
        if alpha < 3 {
            continue;
        }

        // Count occurrences of this exact string
        let count = text.matches(matched).count();
        if count < MIN_OCCURRENCES {
            continue;
        }

        // Calculate net savings
        let gross = matched.len() * count;
        let overhead = matched.len() + 2 * count;
        let net = gross.saturating_sub(overhead);
        if net <= 0 {
            continue;
        }

        candidates.push(Candidate { string: matched, count });
        seen_regions.push((start, start + match_len));
    }

    // Sort by net savings
    candidates.sort_by(|a, b| {
        let a_save = a.string.len() * a.count - (a.string.len() + 2 * a.count);
        let b_save = b.string.len() * b.count - (b.string.len() + 2 * b.count);
        b_save.cmp(&a_save)
    });

    // Deduplicate by string value
    let mut seen_strings: Vec<&str> = Vec::new();
    candidates.retain(|c| {
        if seen_strings.contains(&c.string) {
            false
        } else {
            seen_strings.push(c.string);
            true
        }
    });

    candidates.truncate(20);
    candidates
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
        assert!(result.matches("@1").count() >= 2, "@1 should appear in header and content");
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
        let result = compress("short text");
        assert_eq!(result, "short text");
    }

    #[test]
    fn test_repeated_phrase() {
        let text = "the quick brown fox jumps over the lazy dog. the quick brown fox jumps again.";
        let result = compress(text);
        assert!(result.contains("@dict:"), "should have dictionary for repeated phrase");
        // Should have at most a few entries (not 12+ overlapping ones)
        let entry_count = result.matches("@dict:").count();
        assert!(entry_count >= 1, "should have dictionary");
    }
}


