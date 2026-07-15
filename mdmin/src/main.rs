//! mdmin CLI — Markdown minifier for LLM token optimization.
//!
//! Reads from a file or stdin, writes minified output to stdout.
//! Compression level via `-l` flag or `MDMIN_LEVEL` env var.

#![deny(clippy::all, clippy::pedantic, clippy::nursery)]
#![forbid(unsafe_code)]

use std::io::{self, Read, Write};
use std::path::PathBuf;

use clap::Parser;

use mdmin::{CodeBlockMode, Config, Level, Minifier};

/// Tree-sitter-based Markdown minifier for LLM token optimization.
#[derive(Parser)]
#[command(name = "mdmin", version, about)]
struct Args {
    /// Input file (reads from stdin if omitted).
    file: Option<PathBuf>,

    /// Compression level [default: 2] [env: MDMIN_LEVEL]
    #[arg(short = 'l', long = "level", env = "MDMIN_LEVEL")]
    level: Option<String>,

    /// Code block handling mode [default: preserve] [env: MDMIN_CODE_BLOCKS]
    #[arg(short = 'c', long = "code-blocks", env = "MDMIN_CODE_BLOCKS")]
    code_blocks: Option<String>,

    /// Write output to file instead of stdout.
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Show token savings statistics on stderr.
    #[arg(short = 's', long = "stats")]
    stats: bool,

    /// Suppress warnings on stderr.
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// Suppress the prefix legend in L3/L4 output.
    #[arg(long = "no-legend")]
    no_legend: bool,
}

fn main() {
    if let Err(e) = run() {
        eprintln!("mdmin: error: {e}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Resolve level
    let level = match args.level.as_deref() {
        Some(s) => Level::from_str(s).ok_or_else(|| {
            format!(
                "invalid level '{}': use 0-4 or off/light/medium/structured/ultra",
                s
            )
        })?,
        None => Level::Medium,
    };

    // Resolve code block mode
    let code_blocks = match args.code_blocks.as_deref() {
        Some(s) => CodeBlockMode::from_str(s).ok_or_else(|| {
            format!("invalid code block mode '{}': use preserve or compress", s)
        })?,
        None => CodeBlockMode::Preserve,
    };

    // Read input
    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path)
            .map_err(|e| format!("reading '{}': {e}", path.display()))?,
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("reading stdin: {e}"))?;
            buf
        }
    };

    // Minify
    let config = Config::new(level)
        .with_code_blocks(code_blocks)
        .with_legend(!args.no_legend);
    let mut minifier = Minifier::new(&config)
        .map_err(|e| format!("initializing minifier: {e}"))?;
    let result = minifier
        .minify(&input)
        .map_err(|e| format!("minification failed: {e}"))?;

    // Write output
    match &args.output {
        Some(path) => {
            std::fs::write(path, &result.output)
                .map_err(|e| format!("writing '{}': {e}", path.display()))?;
        }
        None => {
            io::stdout()
                .write_all(result.output.as_bytes())
                .map_err(|e| format!("writing stdout: {e}"))?;
        }
    }

    // Show stats on stderr
    if args.stats {
        if !args.quiet {
            eprintln!(
                "tokens: {} → {} ({:.0}% savings)",
                result.input_tokens, result.output_tokens, result.savings_pct
            );
        }
    }

    Ok(())
}
