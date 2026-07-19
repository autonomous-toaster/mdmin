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
const MIN_LEN: usize = 11;

/// Minimum occurrences for compression to be worthwhile.
const MIN_OCCURRENCES: usize = 4;

/// Apply general n-gram dictionary compression to text.
#[must_use]
/// Minimum file size for dictionary compression (small files: overhead > savings).
#[cfg(not(test))]
const MIN_FILE_SIZE: usize = 0;
#[cfg(test)]
const MIN_FILE_SIZE: usize = 0;

pub fn compress(text: &str) -> String {
    // Skip dictionary for small files (overhead > savings)
    if text.len() < MIN_FILE_SIZE {
        return text.to_string();
    }
    let candidates = find_repeated(text);
    if candidates.is_empty() {
        return text.to_string();
    }

    // Build dictionary
    let mut dict: HashMap<&str, String> = HashMap::new();
    let mut entries: Vec<String> = Vec::new();

    for (i, candidate) in candidates.iter().enumerate() {
        let key = format!("@m{}", i + 1);
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
/// Only matches at word boundaries to avoid fragments.
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
    let mut seen_regions: Vec<(usize, usize)> = Vec::new();

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

        // Extend the match to word boundaries
        let mut match_start = start;
        let mut match_end = char_positions[idx + MIN_LEN];
        let _other_start_pos = first_pos + (match_end - start);

        // Extend backward to word boundary (max 10 chars back)
        while match_start > 0 && start - match_start < 10 {
            // Get the character just before match_start using chars().last()
            let prefix = &text[..match_start];
            let prev_char = match prefix.chars().last() {
                Some(c) => c,
                None => break,
            };
            let prev_char_len = prev_char.len_utf8();
            
            if prev_char.is_alphanumeric() || prev_char == '_' || prev_char == '/' || prev_char == '.' || prev_char == '-' || prev_char == '\'' {
                match_start -= prev_char_len;
            } else {
                break;
            }
        }

        // Extend forward to word boundary
        let backward_offset = start - match_start;
        let other_start = if backward_offset <= first_pos {
            first_pos - backward_offset
        } else {
            // Can't extend backward as much on the other occurrence
            first_pos
        };
        let max_len = 60.min(n - match_start).min(n - other_start);
        while match_end < match_start + max_len {
            let c = text[match_end..].chars().next().unwrap_or('\0');
            if c.is_alphanumeric() || c == '_' || c == '/' || c == '.' || c == '-' {
                match_end += c.len_utf8();
            } else {
                break;
            }
        }

        let matched_raw = &text[match_start..match_end];
        let matched = matched_raw.trim();

        // Skip if too short after boundary adjustment or trimming
        if matched.len() < MIN_LEN {
            continue;
        }

        // Skip candidates containing newlines, markdown link syntax, HTML tags, or code boundaries
        if matched.contains('\n') || matched.contains("](") || matched.contains("](https")
            || matched.contains('<') || matched.contains('>') || matched.contains('`')
        {
            continue;
        }

        // Skip candidates that start with non-alphanumeric (except / for paths)
        let first = matched.chars().next().unwrap_or('\0');
        if !first.is_alphanumeric() && first != '/' {
            continue;
        }

        // Skip candidates that are mostly non-alphanumeric
        let alpha_count = matched.chars().filter(|c| c.is_alphanumeric()).count();
        let alpha_ratio = alpha_count as f64 / matched.len() as f64;
        if alpha_ratio < 0.4 || alpha_count < 5 {
            continue;
        }

        // Count occurrences of this exact string
        let count = text.matches(matched).count();
        if count < MIN_OCCURRENCES {
            continue;
        }

        // Calculate net savings (@mN references are 3 chars)
        let gross = matched.len() * count;
        let overhead = matched.len() + 3 * count;
        let net = gross.saturating_sub(overhead);
        if net <= 0 {
            continue;
        }

        candidates.push(Candidate { string: matched, count });
        seen_regions.push((match_start, match_end));
    }

    // Sort by net savings (@mN references are 3 chars)
    candidates.sort_by(|a, b| {
        let a_save = a.string.len() * a.count - (a.string.len() + 3 * a.count);
        let b_save = b.string.len() * b.count - (b.string.len() + 3 * b.count);
        b_save.cmp(&a_save)
    });

    // Deduplicate: remove candidates that are substrings of other candidates
    let mut seen_strings: Vec<&str> = Vec::new();
    candidates.retain(|c| {
        if seen_strings.iter().any(|s| s.contains(c.string) || c.string.contains(*s)) {
            false
        } else {
            seen_strings.push(c.string);
            true
        }
    });

    candidates.truncate(30);
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_compression() {
        let text = "use /very/long/path/to/some/file.rs and also /very/long/path/to/some/file.rs and again /very/long/path/to/some/file.rs and once more /very/long/path/to/some/file.rs";
        let result = compress(text);
        assert!(result.starts_with("@dict:"), "should start with dictionary header");
        assert!(result.contains("@m1"), "should have reference @m1");
        assert!(result.matches("@m1").count() >= 4, "@m1 should appear in header and content");
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
        let text = "the quick brown fox jumps over the lazy dog. the quick brown fox jumps again. the quick brown fox leaps high. the quick brown fox runs fast.";
        let result = compress(text);
        assert!(result.contains("@dict:"), "should have dictionary for repeated phrase");
        // Should have at most a few entries (not 12+ overlapping ones)
        let entry_count = result.matches("@dict:").count();
        assert!(entry_count >= 1, "should have dictionary");
    }
}


