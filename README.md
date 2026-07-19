# mdmin

**Tree-sitter-based Markdown minifier for LLM token optimization.**

Markdown is designed for humans — bold, italic, horizontal rules, blank lines, and verbose section names carry semantic zero for an LLM but consume tokens. mdmin parses Markdown into a tree-sitter CST, applies deterministic transformations, and emits compact output that preserves meaning while reducing token count.

## Usage

```bash
# Pipe from stdin
cat doc.md | mdmin -l 2 > doc.min.md

# Read a file
mdmin -l 3 doc.md

# With env var
MDMIN_LEVEL=3 mdmin doc.md

# Grammar stripping (remove filler words, articles, aux verbs)
mdmin -l 2 -g doc.md

# Local dictionary (compress repeated long strings)
mdmin -l 2 -d doc.md

# All options combined
mdmin -l 2 -g -d doc.md

# Show token savings
mdmin -l 2 -s doc.md
# stderr: tokens: 1200 → 720 (40% savings)

# Write to file
mdmin -l 2 -o out.md doc.md
```

## Compression Levels

| Level | Name | Description | Valid Markdown? |
|-------|------|-------------|----------------|
| 0 | Off | No-op, input == output | ✅ |
| 1 | Light | Strip bold, italic, HR, HTML comments, strikethrough | ✅ |
| 2 | Medium | Normalize headings, compress tables, flatten nested lists, inline tiny sections, strip URL protocols, merge short list items, compress code blocks | ✅ |
| 3 | Structured | TOON-like indented structure, `+`/`!` checklists | ❌ |
| 4 | Ultra | `{}` brace grouping, minimal whitespace | ❌ |

## Grammar Stripping (`-g`)

Orthogonal config option, works at any level. Removes filler words, articles, aux verbs, hedging, and conjunctions from prose text. Pure Rust, no Python dependency.

Uses **entropy-based word scoring** — each word gets a frequency score (0.0–1.0) combined with heuristics for word length and sentence position. Words above a configurable threshold are removed.

**Levels:**
- `-g` or `-g medium` — default, removes moderately common words (score ≥ 0.6)
- `-g light` — conservative, only removes very common words (score ≥ 0.8)
- `-g aggressive` — aggressive, removes less common filler too (score ≥ 0.4)

**Protected words** — temporal/spatial words (before, after, between, within, etc.) are never removed, preserving meaning in spec documents.

**Safeguards:**
- RFC 2119 protected: SHALL, MUST, MAY, SHOULD never removed
- Negation-safe: words after not/never/no preserved
- Code-safe: backtick regions preserved verbatim
- Language-aware: uses `whatlang` for detection, per-language word lists

Adds 5-10pp additional token savings on real-world files.

## Local Dictionary (`-d`)

Orthogonal config option, works at any level. Finds repeated substrings of 12+ chars (min 3 occurrences) using LZ77-style search and replaces them with short `@mN` references. Dictionary header emitted at top of output.

```
@dict:
  @m1: /managed-agents
  @m2: budget_tokens
  @m3: messages.create

Use @m1 API with @m2 and @m3.
```

Adds 1-3pp additional savings on files with repeated paths, URLs, or identifiers.

## Code Block Compression (`-c`)

Orthogonal config option, works at any level. Controls how fenced code blocks are handled:

- `preserve` — leave code blocks entirely unchanged
- `compress-whitespace` — collapse consecutive blank lines within code blocks
- `compress` (default) — strip indentation, trailing whitespace, and collapse blank lines

```bash
mdmin -l 2 -c compress doc.md
```

Code blocks account for ~58% of corpus bytes. Compress mode reduces indentation (7.5% of code chars) and blank lines (20.9% of code lines). Research (arXiv 2508.13666) confirms LLMs maintain performance on unformatted code.

## Benchmark Results

Measured on a corpus of 115 real-world Markdown files (skills, docs, specs) using len/4 token estimation.

| Config | Savings | Config | Savings |
|--------|---------|--------|---------|
| L0 | 0.0% | L1 | 0.8% |
| L2 | 2.8% | L2_d | 11.9% |
| L2_g | 10.5% | **L2_gd** | **19.0%** |
| L3 | 3.5% | L3_d | 12.5% |
| L3_g | 11.1% | L3_gd | 19.5% |
| L4 | 7.7% | L4_d | 17.3% |
| L4_g | 12.0% | L4_gd | 21.0% |

**Monotonicity**: L0 < L1 < L2 < L3 < L4 on all 115 files ✅
**Dictionary reversibility**: 98/98 files fully reversible ✅
**Throughput**: ~50,000 bytes/sec (L2_gd, debug build)

Run the benchmark yourself:
```bash
MDMIN_BENCH_CORPUS="/path/to/skills" cargo run --bin mdmin-bench
```

## CLI

```
mdmin [OPTIONS] [FILE]

Read FILE or stdin, emit minified markdown to stdout.

OPTIONS:
  -l, --level <LEVEL>      Compression level [default: 2] [env: MDMIN_LEVEL]
                            0 | 1 | 2 | 3 | 4
  -c, --code-blocks <MODE> Code block handling [default: compress] [env: MDMIN_CODE_BLOCKS]
                            preserve | compress-whitespace | compress
                            compress strips indentation, trailing whitespace, blank lines
  -g, --grammar-strip [<LEVEL>]  Strip filler words, articles, aux verbs, hedging
                                 [levels: light, medium (default), aggressive]
  -d, --dictionary         Compress repeated long strings with local dictionary
  -o, --output <FILE>      Write to file instead of stdout
  -s, --stats              Show token savings on stderr
  -q, --quiet              Suppress warnings on stderr
      --no-legend          Suppress prefix legend in L3/L4 output
  -h, --help               Print help
  -V, --version            Print version
```

## Rust API

```rust
use mdmin::{Config, Level, CodeBlockMode, GrammarLevel, Minifier};

let config = Config::new(Level::Medium)
    .with_code_blocks(CodeBlockMode::Compress)
    .with_grammar_strip(GrammarLevel::Medium)
    .with_dictionary(true);

let mut minifier = Minifier::new(&config)?;
let result = minifier.minify(input)?;

println!("{} tokens ({}% savings)",
    result.output_tokens,
    result.savings_pct as usize);
```

## How it works

1. **Parse** — tree-sitter-markdown produces a CST with byte-accurate node positions
2. **Walk** — recursive tree walk matches node kinds and collects edits (deletions/replacements)
3. **Apply** — edits are sorted by position and applied to the source text
4. **L2 text passes** — nested list flattening, inline tiny sections, URL protocol stripping, list item compression, code block compression
5. **Grammar strip** — optional pass removes filler words, articles, aux verbs (negation-aware, RFC 2119-safe)
6. **Dictionary** — optional LZ77-style pass finds repeated substrings (min 12 chars, min 3 occurrences), emits `@dict` header
7. **Structure** — for L3/L4, an additional pass restructures the output into compact formats

No iteration loop — single-pass, deterministic.

## Inspirations & Sources

mdmin builds on ideas from several projects and papers in the LLM token optimization space:

### Projects

- **[rtk (Rust Token Killer)](https://github.com/rtk-ai/rtk)** — CLI proxy that reduces LLM token consumption by 60-90% on common dev commands. Proved that deterministic output compression is viable and effective. mdmin applies similar principles to Markdown content rather than CLI output.
- **[Headroom](https://github.com/headroomlabs-ai/headroom)** — Per-content-type compression pipeline (JSON, code, text) for AI agent tool outputs. Demonstrated that different content types need different compression strategies.
- **[Caveman](https://github.com/JuliusBrussee/caveman)** — Claude Code skill that cuts 65% of output tokens by speaking in compressed prose. Its grammar stripping rules (drop articles, filler, hedging) directly inspired mdmin's `-g` flag.
- **[Caveman Compression](https://github.com/wilpel/caveman-compression)** — NLP-based text compression using spaCy for POS-aware stop word removal. mdmin's grammar stripping is a pure Rust reimplementation of this approach, with added negation safety and RFC 2119 keyword protection.

### Papers

- **[ONTO: A Token-Efficient Columnar Notation for LLM Input Optimization](https://arxiv.org/abs/2604.17512)** (arXiv 2604.17512) — Columnar notation that declares field names once then lists values positionally, achieving 46-51% token reduction vs JSON. Validates mdmin's positional table compression approach.
- **[Lossless Prompt Compression via Dictionary-Encoding and In-Context Learning](https://arxiv.org/abs/2604.13066)** (arXiv 2604.13066) — Demonstrates that LLMs can understand compressed meta-tokens when given a dictionary. Validates mdmin's `-d` flag approach and shows that aggressive dictionary compression is safe for LLM consumption.
- **[Lossless Token Sequence Compression via Meta-Tokens](https://arxiv.org/abs/2506.00307)** (arXiv 2506.00307) — LZ77-style compression at the token level, achieving 27% average reduction. mdmin's n-gram dictionary applies the same principle at the text level.
- **[CompactPrompt: A Unified Pipeline for Prompt and Data Compression in LLM Workflows](https://arxiv.org/abs/2510.18043)** (arXiv 2510.18043) — Uses n-gram abbreviation, self-information scoring, and numerical quantization for prompt compression. Inspired mdmin's multi-pass approach.
- **[Large Language Model as Token Compressor and Decompressor](https://arxiv.org/abs/2603.25340)** (arXiv 2603.25340) — Explores using LLMs themselves for lossy compression. Complementary approach to mdmin's deterministic method.
- **[Entropy Gate: Entropy Quenching for Near-Lossless Token Compression in LLM Pipelines](https://arxiv.org/abs/2606.03739)** (arXiv 2606.03739) — Scores each token by multi-factor information energy combining statistical, structural, and positional components. Inspired mdmin's entropy-based word scoring for grammar stripping.
- **[The Hidden Cost of Readability: How Code Formatting Silently Consumes Your LLM Budget](https://arxiv.org/abs/2508.13666)** (arXiv 2508.13666) — Shows LLMs maintain performance on unformatted code while reducing token consumption. Validates mdmin's code block compression approach.

### Principles

- **Deterministic over learned** — mdmin uses rule-based transformations, not ML models. Predictable, debuggable, zero runtime cost.
- **Structure-aware** — tree-sitter CST enables precise surgical edits without regex fragility.
- **Single-pass** — each level is one walk through the CST. No iteration, no convergence loops.
- **Orthogonal options** — grammar stripping and dictionary are config flags, not levels. Composability over monolithic modes.

## License

WTFPL — Do What the Fuck You Want to Public License.
