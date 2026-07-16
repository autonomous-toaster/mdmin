## Why

Long file paths, URLs, and identifiers repeat frequently in skill files and documentation. A single path like `/managed-agents` can appear 18+ times in one file, consuming ~270 chars. A local dictionary that defines each long string once and references it with a short code (`@1`, `@2`) can save 1-3% additional tokens on top of existing compression, with minimal complexity.

## What Changes

- New `--dictionary` / `-d` config option, orthogonal to compression levels and grammar stripping
- Targeted pattern detection: long file paths (30+ chars), long URLs, long identifiers
- Dictionary emitted at top of output: `@1 = /path/to/file`, `@2 = https://long.url/...`
- Code-block-safe: patterns inside fenced code blocks and inline code are not compressed
- Minimum savings threshold: only compress if net savings > 0 (dictionary entry + references < original text)

## Capabilities

### New Capabilities
- `targeted-dictionary`: Local dictionary compression for long repeated strings (paths, URLs, identifiers)

### Modified Capabilities
- `minification-core`: Add `dictionary` field to `Config`; add dictionary pass to pipeline
- `cli`: Add `-d`/`--dictionary` flag

## Impact

- `mdmin/src/dictionary.rs`: New module — pattern detection, dictionary building, replacement logic
- `mdmin/src/engine.rs`: Add dictionary pass after grammar stripping, before L3/L4
- `mdmin/src/lib.rs`: Add `dictionary` to `Config`
- `mdmin/src/main.rs`: Add `-d`/`--dictionary` CLI flag
