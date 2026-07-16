//! mdmin — Tree-sitter-based Markdown minifier for LLM token optimization.
//!
//! Reduces token consumption by stripping decorative formatting, compressing
//! structure, and emitting compact representations. 5 levels from no-op to
//! ultra-compact DSL.

#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod dictionary;
mod engine;
mod grammar;
mod passes;

pub use engine::Minifier;
pub use grammar::Level as GrammarLevel;

/// Compression level.
///
/// Each level is a superset of the previous. Level 0 is a no-op.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// No-op — input returned unchanged, no parse.
    Off = 0,
    /// Strip decorative formatting (bold, italic, HR, comments, reference defs).
    Light = 1,
    /// Semantic compression (normalize headings, flatten lists, compress tables).
    Medium = 2,
    /// TOON-like indented structure with `+`/`-` checklist notation.
    Structured = 3,
    /// Ultra-compact single-line grouped output with `{}`.
    Ultra = 4,
}

impl Level {
    /// Parse a level from a string (CLI or env var input).
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "0" | "off" => Some(Self::Off),
            "1" | "light" => Some(Self::Light),
            "2" | "medium" => Some(Self::Medium),
            "3" | "structured" => Some(Self::Structured),
            "4" | "ultra" | "extreme" => Some(Self::Ultra),
            _ => None,
        }
    }
}

/// How to handle fenced code blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeBlockMode {
    /// Leave code blocks entirely unchanged (default).
    Preserve,
    /// Collapse runs of blank lines within code blocks to a single newline.
    CompressWhitespace,
}

impl CodeBlockMode {
    /// Parse from a string (CLI or env var input).
    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "preserve" => Some(Self::Preserve),
            "compress" => Some(Self::CompressWhitespace),
            _ => None,
        }
    }
}

/// Configuration for a minification pass.
#[derive(Debug, Clone)]
pub struct Config {
    /// Compression level.
    pub level: Level,
    /// Code block handling mode.
    pub code_blocks: CodeBlockMode,
    /// Prepend a legend line explaining prefix conventions (L3/L4 only).
    pub legend: bool,
    /// Grammar stripping level (None = disabled).
    pub grammar_strip: Option<GrammarLevel>,
    /// Compress long repeated strings (paths, URLs, identifiers) with a local dictionary.
    pub dictionary: bool,
}

impl Config {
    /// Create a new config with the given level and default settings.
    #[must_use]
    pub fn new(level: Level) -> Self {
        Self {
            level,
            code_blocks: CodeBlockMode::Preserve,
            legend: true,
            grammar_strip: None,
            dictionary: false,
        }
    }

    /// Set code block handling mode.
    #[must_use]
    pub fn with_code_blocks(mut self, mode: CodeBlockMode) -> Self {
        self.code_blocks = mode;
        self
    }

    /// Enable or disable the prefix legend in L3/L4 output.
    #[must_use]
    pub fn with_legend(mut self, enabled: bool) -> Self {
        self.legend = enabled;
        self
    }

    /// Enable grammar stripping with a specific level.
    #[must_use]
    pub fn with_grammar_strip(mut self, level: GrammarLevel) -> Self {
        self.grammar_strip = Some(level);
        self
    }

    /// Disable grammar stripping.
    #[must_use]
    pub fn without_grammar_strip(mut self) -> Self {
        self.grammar_strip = None;
        self
    }

    /// Enable or disable local dictionary compression (long repeated strings).
    #[must_use]
    pub fn with_dictionary(mut self, enabled: bool) -> Self {
        self.dictionary = enabled;
        self
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(Level::Medium)
    }
}

impl Config {
    /// Check if grammar stripping is enabled.
    #[must_use]
    pub const fn is_grammar_strip_enabled(&self) -> bool {
        self.grammar_strip.is_some()
    }

    /// Get the grammar stripping level, if enabled.
    #[must_use]
    pub const fn grammar_level(&self) -> Option<GrammarLevel> {
        self.grammar_strip
    }
}

/// Result of a minification pass.
#[derive(Debug, Clone)]
pub struct MinifyResult {
    /// The minified output text.
    pub output: String,
    /// Estimated input token count (len / 4).
    pub input_tokens: usize,
    /// Estimated output token count (len / 4).
    pub output_tokens: usize,
    /// Percentage of tokens saved (0.0 – 100.0).
    pub savings_pct: f64,
}

// ─── Re-exports for convenience ──────────────────────────────────────────────

pub use engine::Error as MinifyError;
