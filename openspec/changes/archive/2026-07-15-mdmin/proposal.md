## Why

LLM context windows are expensive. Markdown is designed for humans — bold, italic, horizontal rules, blank lines, and verbose section names carry semantic zero for an LLM but consume tokens. sift already optimizes JSON output (TOON format) but has no equivalent for Markdown. A dedicated Markdown minifier crate can reduce token consumption by 40–70% on structured documents (specs, docs, READMEs), directly improving sift's value proposition.

## What Changes

- New crate `mdmin` in this repo — a tree-sitter-based Markdown minifier
- 5 compression levels: Off (no-op), Light, Medium, Structured, Ultra
- CLI binary: reads from file or stdin, writes to stdout, level via `-l` flag or `MDMIN_LEVEL` env var
- Code block handling: preserve or compress-whitespace (no strip)
- Token savings stats on stderr via `-s` flag
- No dictionary, no iteration loop — single-pass, deterministic

## Capabilities

### New Capabilities
- `minification-core`: Core minification engine — tree-sitter parse, transform passes per level, serialize back to text
- `cli`: CLI binary — file/stdin I/O, level selection, stats output, env var support

### Modified Capabilities

- *(none — new crate, no existing specs to modify)*

## Impact

- New crate `mdmin/` at repo root with `Cargo.toml`, `src/lib.rs`, `src/bin/mdmin.rs`
- Dependency on `tree-sitter` + `tree-sitter-markdown` (C lib via cc)
- sift will add `mdmin` as a workspace dependency when ready (out of scope for this change)
- No breaking changes to existing code
