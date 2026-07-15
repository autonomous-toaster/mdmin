## Context

mdmin is a new crate in this repo. It will be consumed by sift (in `../baish/`) as a workspace dependency. The crate must parse Markdown using tree-sitter, apply deterministic transformations per compression level, and emit optimized text. No existing Markdown processing infrastructure exists in this repo.

## Goals / Non-Goals

**Goals:**
- Tree-sitter-based Markdown parsing and AST transformation
- 5 compression levels (0–4) with deterministic, single-pass behavior
- CLI binary with file/stdin I/O, level selection, stats output
- Code block handling: preserve or compress-whitespace
- Level 0 MUST be a no-op (skip parse entirely)
- Token estimation (len/4) for stats reporting

**Non-Goals:**
- No custom dictionary or abbreviation system
- No iteration loop (single pass per level)
- No code block stripping (caller can pipe through grep)
- No Lua API (sift concern, out of scope)
- No streaming or incremental parsing

## Decisions

### Tree-sitter over pure-Rust parsers
**Choice**: `tree-sitter` + `tree-sitter-markdown` (v0.7.1)
**Rationale**: Tree-sitter's CST preserves source byte ranges, enabling precise surgical transformations (delete a node, replace text, keep everything else unchanged). Pure-Rust parsers (pulldown-cmark, markdown crate) produce higher-level ASTs that lose source position information, making reconstruction lossy.
**Trade-off**: C library dependency via cc. Acceptable for a CLI tool.

### Single-pass architecture
**Choice**: Each level is a single pass through the CST. No iteration.
**Rationale**: Predictable, debuggable, O(n). Iteration (minify → count → try harder) adds complexity with unclear benefit — callers know what level they want.

### Level 0 as no-op
**Choice**: Level 0 skips tree-sitter entirely and returns input unchanged.
**Rationale**: Zero overhead when minification is disabled. Useful for benchmarking and passthrough mode.

### TOON-like format for Level 3, ultra-compact for Level 4
**Choice**: Level 3 emits indentation-based structured output (similar to TOON). Level 4 emits single-line grouped output with `{}` and minimal whitespace.
**Rationale**: L3 is readable and debuggable (~40-55% savings). L4 maximizes token density (~55-70% savings) for context-constrained scenarios. TOON is already proven in sift's JSON pipeline.

### Transform passes per level

| Pass | L1 | L2 | L3 | L4 |
|---|---|---|---|---|
| Strip emphasis/strong/strikethrough | ✅ | ✅ | ✅ | ✅ |
| Strip HTML comments | ✅ | ✅ | ✅ | ✅ |
| Strip horizontal rules | ✅ | ✅ | ✅ | ✅ |
| Strip reference-style link defs | ✅ | ✅ | ✅ | ✅ |
| Collapse whitespace | ✅ | ✅ | ✅ | ✅ |
| Normalize headings (`# Title` → `title:`) | ❌ | ✅ | ✅ | ✅ |
| Flatten nested lists | ❌ | ✅ | ✅ | ✅ |
| Compress tables | ❌ | ✅ | ✅ | ✅ |
| Inline tiny sections | ❌ | ✅ | ✅ | ✅ |
| Remove boilerplate phrases | ❌ | ✅ | ✅ | ✅ |
| TOON-like indented structure | ❌ | ❌ | ✅ | ✅ |
| Checklist → `+`/`-` notation | ❌ | ❌ | ✅ | ✅ |
| Single-line grouping `{}` | ❌ | ❌ | ❌ | ✅ |
| Remove optional whitespace | ❌ | ❌ | ❌ | ✅ |

### Code block handling
**Choice**: `Preserve` (default) and `CompressWhitespace` modes. No `Strip`.
**Rationale**: Stripping code blocks is too destructive — code often carries the most semantically dense information. Callers who want to strip can pipe through `grep -v '```'`.

## Risks / Trade-offs

- **[Tree-sitter C dependency]** → Requires cc build tool. Mitigation: tree-sitter is mature, widely used, and the C lib is vendored via the crate.
- **[Level 3/4 output not valid Markdown]** → Callers must know they're getting LLM-optimized output, not renderable Markdown. Mitigation: document clearly, Level 2 is the highest that still produces valid Markdown.
- **[Token estimation is rough (len/4)]** → Not accurate for all tokenizers. Mitigation: good enough for relative savings reporting. sift can replace with actual tokenizer if needed.
