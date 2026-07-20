# mdmin

**Tree-sitter-based Markdown minifier for LLM token optimization.**

Markdown is designed for humans — bold, italic, horizontal rules, blank lines, and verbose section names carry semantic zero for an LLM but consume tokens. mdmin parses Markdown into a tree-sitter CST, applies deterministic transformations, and emits compact output that preserves meaning while reducing token count.

## Usage

```bash
# Basic compression
cat doc.md | mdmin -l 2 > doc.min.md

# With grammar stripping (remove filler words)
mdmin -l 2 -g doc.md

# All options
mdmin -l 4 -g -d doc.md

# Show token savings
mdmin -l 2 -s doc.md
```

## Compression Levels

| Level | Name | Description |
|-------|------|-------------|
| 0 | Off | No-op |
| 1 | Light | Strip bold, italic, HR, HTML comments, strikethrough |
| 2 | Medium | Normalize headings, compress tables, flatten nested lists, inline tiny sections, strip URL protocols, compress code blocks |
| 3 | Structured | TOON-like indented structure, `+`/`!` checklists |
| 4 | Ultra | `{}` brace grouping, minimal whitespace |

## Grammar Stripping (`-g`)

Removes filler words, articles, aux verbs, hedging from prose. Uses entropy-based word scoring with configurable threshold.

**Levels:** `light` (0.8), `medium`/default (0.6), `aggressive` (0.4)

**Safeguards:** RFC 2119 keywords preserved, negation-aware, code blocks skipped, language-aware.

## Code Block Compression (`-c`)

Controls fenced code block handling: `preserve`, `compress-whitespace`, or `compress` (default).

Strips indentation, trailing whitespace, blank lines, and short comment-only lines. For Python blocks, also removes docstrings via tree-sitter AST.

## Dictionary (`-d`)

Finds repeated substrings (12+ chars, 4+ occurrences) and replaces with `@mN` references. Most effective on files with repeated paths, URLs, or identifiers.

## Benchmark Results

Measured on two corpora (226 files total) using cl100k_base tokenizer.

| Config | Badrobots | Anthropic |
|--------|-----------|-----------|
| L4 + grammar + dict | **16.1%** | **17.6%** |
| Pass rate | **100%** | **100%** |

## CLI

```
mdmin [OPTIONS] [FILE]

Options:
  -l, --level <LEVEL>        Compression level [0-4, default: 2]
  -c, --code-blocks <MODE>   Code block handling [preserve|compress-whitespace|compress]
  -g, --grammar-strip [LVL]  Strip filler words [light|medium|aggressive]
  -d, --dictionary           Compress repeated strings
  -o, --output <FILE>        Write to file
  -s, --stats                Show token savings
  -q, --quiet                Suppress warnings
      --no-legend            Suppress L3/L4 legend
```

## Rust API

```rust
use mdmin::{Config, Level, Minifier};

let config = Config::new(Level::Medium)
    .with_grammar_strip(mdmin::GrammarLevel::Medium)
    .with_dictionary(true);

let mut minifier = Minifier::new(&config)?;
let result = minifier.minify(input)?;
```

## How it works

1. **Parse** — tree-sitter-markdown produces a CST
2. **Walk** — recursive walk matches node kinds, collects edits
3. **Apply** — edits sorted by position, applied to source
4. **L2 passes** — list flattening, section inlining, URL stripping, code compression
5. **Grammar strip** — optional filler word removal (negation-aware, code-safe)
6. **Dictionary** — optional LZ77-style repeated substring compression
7. **L3/L4** — structural restructuring into compact formats

Single-pass, deterministic, no ML dependency.

## License

WTFPL
