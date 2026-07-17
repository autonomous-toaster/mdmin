## Context

mdmin has 37 unit tests covering individual features, but no end-to-end benchmark that measures token savings, monotonicity, structure preservation, or grammar strip safety. The 222-file corpus (skills from badrobots and anthropic) provides a realistic test bed.

## Goals / Non-Goals

**Goals:**
- Token savings measurement using tiktoken-rs (real tokenizer)
- Monotonicity check: L0 < L1 < L2 < L3 < L4 on all corpus files
- Structure preservation: round-trip parse of headings, lists, tables
- Grammar strip coverage: every removed word is in the approved frequency table
- Dictionary reversibility: @N references expand back to original text
- Per-level and per-feature breakdowns (-g, -d, -g -d, neither)
- Machine-readable output (JSON) for CI integration

**Non-Goals:**
- No LLM-based evaluation (no API calls, no perplexity)
- No changes to mdmin's source code
- No integration with external CI systems (just the benchmark binary)

## Decisions

### tiktoken-rs over len/4
**Choice**: Use `tiktoken-rs` for token counting instead of the current len/4 approximation.
**Rationale**: Council consensus. len/4 is a crude approximation that varies by tokenizer. tiktoken-rs provides accurate counts for specific models (cl100k_base for GPT-4, etc.).

### Round-trip structure extraction over downstream task accuracy
**Choice**: Parse original and compressed Markdown, extract headings/lists/tables, compare.
**Rationale**: Directly measures what mdmin preserves. No LLM API calls needed. Fast, deterministic, automatable. More rigorous than aggregate accuracy metrics that can hide edge cases.

### Grammar strip coverage verification
**Choice**: For every word removed by -g, verify it's in the frequency table. Report unexpected removals.
**Rationale**: Checks every removal individually rather than relying on aggregate metrics. Catches edge cases where a meaningful word slips through the protection lists.

### Corpus access via symlinks
**Choice**: Symlink the 222-file corpus from known paths rather than copying.
**Rationale**: Avoids duplicating files. Corpus is already available at the known paths. Symlinks are git-ignored.

## Benchmark Suites

### Suite 1: Token Savings
For each corpus file, for each level (0-4) and feature combination (none, -g, -d, -g -d):
- Count tokens with tiktoken-rs (cl100k_base)
- Report: tokens, savings %, bytes, compression ratio
- Aggregate: mean, median, min, max across corpus

### Suite 2: Monotonicity
For each corpus file:
- Verify L0 <= L1 <= L2 <= L3 <= L4 (byte count)
- Report: any violations with file name and sizes

### Suite 3: Structure Preservation
For each corpus file, for each level (1-4):
- Parse original: extract headings (##, ###), list items (-, *), table rows (|)
- Parse compressed: extract same structures
- Compare: headings match, list items preserved, table data preserved
- Report: structure preservation % per level

### Suite 4: Grammar Strip Coverage
For each corpus file with -g enabled:
- Collect all words removed by grammar strip
- Check each against frequency table + protection lists
- Report: total removed, in frequency table, protected, unexpected

### Suite 5: Dictionary Reversibility
For each corpus file with -d enabled:
- Extract all @N definitions from the @dict header
- Replace @N references in the output with their definitions
- Verify the result matches the pre-dictionary text
- Report: any irreversibility issues

## Output Format

Machine-readable JSON report:

```json
{
  "corpus": {
    "files": 222,
    "total_bytes": 2015218
  },
  "token_savings": {
    "L0": { "tokens": 500000, "bytes": 2015218 },
    "L2": { "tokens": 460000, "bytes": 1858636, "savings_pct": 8.0 },
    "L2_g": { "tokens": 440000, "bytes": 1780000, "savings_pct": 12.0 },
    "L2_d": { "tokens": 455000, "bytes": 1840000, "savings_pct": 9.0 },
    "L2_gd": { "tokens": 435000, "bytes": 1760000, "savings_pct": 13.0 }
  },
  "monotonicity": { "violations": 0 },
  "structure_preservation": {
    "L1": { "headings": 99.5, "lists": 100.0, "tables": 100.0 },
    "L2": { "headings": 95.0, "lists": 98.0, "tables": 90.0 }
  },
  "grammar_coverage": {
    "total_removed": 12847,
    "in_frequency_table": 12847,
    "protected_removed": 0,
    "unexpected": 0
  },
  "dictionary_reversibility": {
    "files_with_dict": 200,
    "fully_reversible": 200
  }
}
```

## Risks / Trade-offs

- **[Corpus access]** — Symlinks may break if source paths change. Mitigation: document the expected paths, add a fallback download script.
- **[tiktoken-rs compatibility]** — tiktoken-rs may not support all platforms. Mitigation: fall back to len/4 with a warning.
- **[Benchmark runtime]** — 222 files × 5 levels × 4 feature combinations = 4440 runs. Mitigation: single pass per file, cache intermediate results.
