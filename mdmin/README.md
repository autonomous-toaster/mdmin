# mdmin

**Tree-sitter-based Markdown minifier for LLM token optimization.**

Markdown is designed for humans — bold, italic, horizontal rules, blank lines, and verbose section names carry semantic zero for an LLM but consume tokens. mdmin parses Markdown into a tree-sitter CST, applies deterministic transformations, and emits compact output that preserves meaning while reducing token count by up to 70%.

## Usage

```bash
# Pipe from stdin
cat doc.md | mdmin -l 2 > doc.min.md

# Read a file
mdmin -l 3 doc.md

# With env var
MDMIN_LEVEL=3 mdmin doc.md

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
| 2 | Medium | Normalize headings, compress tables | ✅ |
| 3 | Structured | TOON-like indented structure, `+`/`-` checklists | ❌ |
| 4 | Ultra | `{}` brace grouping, minimal whitespace | ❌ |

### Examples

**Input:**
```markdown
## Authentication

**Important**: The server SHALL support JWT.

- [x] Login succeeds
- [ ] Invalid token rejected

| Name | Port |
|------|------|
| API | 8080 |
```

**L1** (strip decoration):
```markdown
## Authentication
Important: The server SHALL support JWT.
- [x] Login succeeds
- [ ] Invalid token rejected
| Name | Port |
| API | 8080 |
```

**L2** (semantic compression):
```markdown
authentication:
Important: The server SHALL support JWT.
- [x] Login succeeds
- [ ] Invalid token rejected
Name:API Port:8080
```

**L3** (structured):
```markdown
authentication:
  Important: The server SHALL support JWT.
  + Login succeeds
  - Invalid token rejected
  Name:API Port:8080
```

**L4** (ultra-compact):
```markdown
authentication{Important: The server SHALL support JWT. + Login succeeds - Invalid token rejected Name:API Port:8080}
```

## CLI

```
mdmin [OPTIONS] [FILE]

Read FILE or stdin, emit minified markdown to stdout.

OPTIONS:
  -l, --level <LEVEL>      Compression level [default: 2] [env: MDMIN_LEVEL]
                            0 | 1 | 2 | 3 | 4
  -c, --code-blocks <MODE> Code block handling [default: preserve] [env: MDMIN_CODE_BLOCKS]
                            preserve | compress
  -o, --output <FILE>      Write to file instead of stdout
  -s, --stats              Show token savings on stderr
  -q, --quiet              Suppress warnings on stderr
  -h, --help               Print help
  -V, --version            Print version
```

## Rust API

```rust
use mdmin::{Config, Level, CodeBlockMode, Minifier};

let config = Config::new(Level::Medium)
    .with_code_blocks(CodeBlockMode::Preserve);

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
4. **Structure** — for L3/L4, an additional pass restructures the output into compact formats

No dictionary, no iteration loop — single-pass, deterministic.

## License

WTFPL — Do What the Fuck You Want to Public License.
