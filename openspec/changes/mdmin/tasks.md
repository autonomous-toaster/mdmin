## 1. Project Setup

- [ ] 1.1 Create `mdmin/` crate with `Cargo.toml`, `src/lib.rs`, `src/main.rs`
- [ ] 1.2 Add `tree-sitter` and `tree-sitter-markdown` dependencies
- [ ] 1.3 Define public API types: `Level`, `CodeBlockMode`, `Config`, `MinifyResult`, `Minifier`

## 2. Core Engine

- [ ] 2.1 Implement tree-sitter Markdown parse/serialize cycle (parse CST, walk, emit text from byte ranges)
- [ ] 2.2 Implement Level 0 as no-op (skip parse, return input unchanged)
- [ ] 2.3 Implement Level 1 transform passes (strip emphasis, strong, strikethrough, HR, HTML comments, reference defs, collapse whitespace)
- [ ] 2.4 Implement Level 2 transform passes (normalize headings, flatten nested lists, compress tables, inline tiny sections, remove boilerplate phrases)
- [ ] 2.5 Implement Level 3 transform passes (TOON-like indented structure, `+`/`-` checklist notation)
- [ ] 2.6 Implement Level 4 transform passes (single-line grouped output with `{}`, minimal whitespace)
- [ ] 2.7 Implement code block handling (`Preserve` and `CompressWhitespace` modes)
- [ ] 2.8 Implement token estimation (len/4) and `MinifyResult` stats computation

## 3. CLI

- [ ] 3.1 Implement CLI argument parsing with `clap` (`-l`/`--level`, `-c`/`--code-blocks`, `-o`/`--output`, `-s`/`--stats`)
- [ ] 3.2 Implement file/stdin I/O (read from path or stdin, write to stdout or `-o` file)
- [ ] 3.3 Implement env var support (`MDMIN_LEVEL`, `MDMIN_CODE_BLOCKS`) with flag override
- [ ] 3.4 Implement stats output on stderr when `-s` is passed
