## Why

mdmin's current compression (L1-L4) targets document structure — headings, tables, lists, code blocks. It ignores sentence-level verbosity: filler words, articles, aux verbs, hedging, and verbose phrasing that carry zero semantic value for LLMs. Research on caveman-compress and similar tools shows 15-30% additional token savings from grammar stripping alone, with no loss of technical accuracy.

## What Changes

- New `--grammar-strip` / `-g` config option, orthogonal to compression levels
- Pure Rust grammar stripping pass using word lists + pattern matching (no Python, no spaCy)
- Language detection via `whatlang` for per-language word lists
- Negation-aware removal: never strip words after "not/never/no/nor" within a sentence
- Code-safe: protected regions (code blocks, inline code, URLs) identified via tree-sitter CST
- New dependency: `whatlang` for language detection, `stop-words` for stop word lists

## Capabilities

### New Capabilities
- `grammar-stripping`: Sentence-level grammar compression — remove filler words, articles, aux verbs, hedging, conjunctions; replace verbose patterns with short synonyms

### Modified Capabilities
- `minification-core`: Add `grammar_strip` field to `Config`; add grammar stripping pass to pipeline
- `cli`: Add `-g`/`--grammar-strip` flag

## Impact

- `mdmin/Cargo.toml`: Add `whatlang`, `stop-words` dependencies
- `mdmin/src/lib.rs`: Add `grammar_strip` to `Config`
- `mdmin/src/grammar.rs`: New module — word lists, replacement patterns, stripping logic
- `mdmin/src/engine.rs`: Add grammar stripping pass after structural transforms
- `mdmin/src/main.rs`: Add `-g`/`--grammar-strip` CLI flag
