//! Grammar stripping — remove filler words, articles, aux verbs, hedging,
//! and verbose patterns from prose text. Pure Rust, no Python dependency.
//!
//! Uses word lists per language (via `whatlang` detection) and negation-aware
//! removal to avoid inverting meaning.

use std::collections::HashSet;

use whatlang::Lang;

/// Word lists and replacement patterns for a language.
struct LangWords {
    /// Articles to remove (a, an, the).
    articles: &'static [&'static str],
    /// Filler adverbs to remove (just, really, basically).
    filler: &'static [&'static str],
    /// Auxiliary verbs to remove (is, are, was, were).
    /// NOTE: "shall", "should", "must", "may", "will" are NOT included —
    /// they are RFC 2119 keywords with semantic meaning in specs.
    aux_verbs: &'static [&'static str],
    /// Hedging words to remove (might, could, perhaps).
    hedging: &'static [&'static str],
    /// Conjunctions to remove (and, or, but).
    conjunctions: &'static [&'static str],
    /// Negation markers — NEVER remove words after these within a sentence.
    negation: &'static [&'static str],
    /// Replacement pairs: (pattern, replacement).
    replacements: &'static [(&'static str, &'static str)],
}
/// English word lists.
const EN: LangWords = LangWords {
    articles: &["a", "an", "the"],
    filler: &[
        "just", "really", "basically", "actually", "simply",
        "essentially", "generally", "very", "quite", "extremely",
        "incredibly", "absolutely", "totally", "completely", "utterly",
        "highly", "particularly", "especially", "truly",
    ],
    aux_verbs: &[
        "is", "are", "was", "were", "been", "being",
        "have", "has", "had", "do", "does", "did",
        "will", "would", "can", "could",
        // NOTE: shall/should/must/may intentionally omitted
        // — they are RFC 2119 keywords with semantic meaning in specs
    ],
    hedging: &[
        "perhaps", "possibly", "maybe",
    ],
    conjunctions: &[
        "and", "or", "but", "however", "furthermore",
        "additionally", "moreover", "nevertheless", "nonetheless",
    ],
    negation: &[
        "not", "n't", "never", "no", "nor", "neither",
        "nobody", "none", "nothing", "nowhere",
        "hardly", "scarcely", "barely", "without",
    ],
    replacements: &[
        ("in order to", "to"),
        ("make sure to", "ensure"),
        ("the reason is because", "because"),
        ("due to the fact that", "because"),
        ("in spite of the fact that", "although"),
        ("utilize", "use"),
        ("utilizes", "uses"),
        ("utilized", "used"),
        ("implement", "build"),
        ("implements", "builds"),
        ("implemented", "built"),
        ("demonstrate", "show"),
        ("demonstrates", "shows"),
        ("demonstrated", "showed"),
        ("sufficient", "enough"),
        ("additional", "more"),
        ("subsequent", "next"),
        ("preceding", "prior"),
        ("facilitate", "help"),
        ("endeavor", "try"),
    ],
};

/// Apply grammar stripping to text.
///
/// Removes filler words, articles, aux verbs, hedging, and conjunctions.
/// Replaces verbose patterns with short synonyms.
/// Negation-aware: never removes words within negation scope.
/// Code-safe: skips content inside backtick-delimited regions.
#[must_use]
pub fn strip(text: &str) -> String {
    // Detect language
    let lang = whatlang::detect(text)
        .map(|info| info.lang())
        .unwrap_or(Lang::Eng);

    let words = match lang {
        Lang::Eng => &EN,
        _ => &EN, // Fallback to English for now
    };

    // Build HashSets for O(1) lookup
    let negation_set: HashSet<&str> = words.negation.iter().copied().collect();
    let articles_set: HashSet<&str> = words.articles.iter().copied().collect();
    let filler_set: HashSet<&str> = words.filler.iter().copied().collect();
    let aux_set: HashSet<&str> = words.aux_verbs.iter().copied().collect();
    let hedging_set: HashSet<&str> = words.hedging.iter().copied().collect();
    let conj_set: HashSet<&str> = words.conjunctions.iter().copied().collect();

    // First pass: apply replacements
    let mut result = text.to_string();
    for (pattern, replacement) in words.replacements {
        result = result.replace(pattern, replacement);
    }

    // Second pass: word-level removal with negation and code safety
    strip_words(&result, &negation_set, &articles_set, &filler_set, &aux_set, &hedging_set, &conj_set)
}

/// Strip filler words from text, respecting negation scope and code regions.
fn strip_words(
    text: &str,
    negation: &HashSet<&str>,
    articles: &HashSet<&str>,
    filler: &HashSet<&str>,
    aux_verbs: &HashSet<&str>,
    hedging: &HashSet<&str>,
    conjunctions: &HashSet<&str>,
) -> String {
    let mut output = String::with_capacity(text.len());
    let mut in_negation = false;
    let mut in_code = false;
    let mut in_backtick = false;
    let mut word_buf = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        // Track code blocks (```...```)
        if ch == '`' {
            let next_is_tick = chars.peek() == Some(&'`');
            if next_is_tick {
                let after = chars.nth(0); // skip second `
                if after == Some('`') {
                    // Toggle code block
                    in_code = !in_code;
                    output.push_str("```");
                    continue;
                } else {
                    // Not a code fence, treat as single backtick
                    in_backtick = !in_backtick;
                    output.push('`');
                    if let Some(c) = after {
                        output.push(c);
                    }
                    continue;
                }
            } else {
                // Toggle inline code
                in_backtick = !in_backtick;
                output.push('`');
                continue;
            }
        }

        // If inside code or backtick, emit verbatim
        if in_code || in_backtick {
            output.push(ch);
            continue;
        }

        // Build words
        if ch.is_alphanumeric() || ch == '\'' || ch == '_' {
            word_buf.push(ch);
            continue;
        }

        // End of word — process it
        if !word_buf.is_empty() {
            let word = word_buf.as_str();
            let lower = word.to_lowercase();

            // Check negation scope
            if negation.contains(lower.as_str()) {
                in_negation = true;
                output.push_str(word);
                output.push(ch);
                word_buf.clear();
                continue;
            }

            // Check clause boundaries — reset negation
            if ch == ',' || ch == ';' || ch == ':' || ch == '—' || ch == '(' || ch == ')' {
                in_negation = false;
            }

            // Check end of sentence — reset negation
            if ch == '.' || ch == '!' || ch == '?' {
                in_negation = false;
            }

            // Remove word if it's filler and not in negation scope
            let should_remove = !in_negation && (
                articles.contains(lower.as_str())
                || filler.contains(lower.as_str())
                || aux_verbs.contains(lower.as_str())
                || hedging.contains(lower.as_str())
                || conjunctions.contains(lower.as_str())
            );

            if !should_remove {
                output.push_str(word);
            } else {
                // Word removed — skip the following space too if it's a space
                if ch == ' ' {
                    word_buf.clear();
                    continue;
                }
            }

            word_buf.clear();
        }

        output.push(ch);
    }

    // Flush remaining word buffer
    if !word_buf.is_empty() {
        let word = word_buf.as_str();
        let lower = word.to_lowercase();
        let should_remove = !in_negation && (
            articles.contains(lower.as_str())
            || filler.contains(lower.as_str())
            || aux_verbs.contains(lower.as_str())
            || hedging.contains(lower.as_str())
            || conjunctions.contains(lower.as_str())
        );
        if !should_remove {
            output.push_str(word);
        }
    }

    // Clean up: collapse multiple spaces
    let mut cleaned = String::with_capacity(output.len());
    let mut prev_space = false;
    for ch in output.chars() {
        if ch == ' ' {
            if prev_space {
                continue;
            }
            prev_space = true;
        } else {
            prev_space = false;
        }
        cleaned.push(ch);
    }

    cleaned
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_articles() {
        let result = strip("the system shall support the API");
        assert!(!result.contains("the "), "articles should be removed");
        assert!(result.contains("system"), "content preserved");
        assert!(result.contains("API"), "nouns preserved");
    }

    #[test]
    fn test_rfc2119_keywords_preserved() {
        let result = strip("the system SHALL support the API");
        assert!(result.contains("SHALL"), "RFC 2119 SHALL preserved");
        let result = strip("the system MUST validate input");
        assert!(result.contains("MUST"), "RFC 2119 MUST preserved");
        let result = strip("the system MAY cache results");
        assert!(result.contains("MAY"), "RFC 2119 MAY preserved");
        let result = strip("the system SHOULD log errors");
        assert!(result.contains("SHOULD"), "RFC 2119 SHOULD preserved");
    }

    #[test]
    fn test_remove_filler() {
        let result = strip("this is really very important");
        assert!(!result.contains("really"), "filler removed");
        assert!(result.contains("important"), "content preserved");
    }

    #[test]
    fn test_negation_preserved() {
        let result = strip("the system shall not allow access");
        assert!(result.contains("not"), "negation preserved");
        assert!(result.contains("allow"), "verb preserved");
    }

    #[test]
    fn test_negation_with_contraction() {
        let result = strip("the system doesn't support that");
        assert!(result.contains("n't"), "negation contraction preserved");
    }

    #[test]
    fn test_verbose_replacement() {
        let result = strip("in order to utilize the API");
        assert!(result.contains("to use"), "verbose pattern replaced");
    }

    #[test]
    fn test_code_block_protected() {
        let result = strip("text ```fn is_valid() {}``` text");
        assert!(result.contains("is_valid"), "code content preserved");
    }

    #[test]
    fn test_inline_code_protected() {
        let result = strip("use the `is_valid` function");
        assert!(result.contains("is_valid"), "inline code preserved");
    }

    #[test]
    fn test_negation_reset_at_sentence() {
        let result = strip("not this. the next sentence");
        assert!(result.contains("not this"), "negation scope respected");
        assert!(!result.contains("the next"), "article after reset removed");
    }

    #[test]
    fn test_hardly_as_negation() {
        let result = strip("hardly any requests succeed");
        assert!(result.contains("hardly"), "implicit negation preserved");
    }
}
