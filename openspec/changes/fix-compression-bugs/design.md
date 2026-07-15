## Context

mdmin's compression pipeline has three confirmed bugs that cause output to be larger than input for certain file types. Tested across 200+ real-world markdown files (badrobots, pi-perso, anthropic skills). The bugs are in the table compression (L2), heading normalization (L2), and L3 indentation passes.

## Goals / Non-Goals

**Goals:**
- L2 output MUST never be larger than L1 output for any input
- L3 output MUST never be larger than L2 output for any input
- Table compression MUST use positional format for multi-column tables
- Table compression MUST skip when it would increase byte count
- L3 indentation MUST only apply to nested list items (not flat lists)
- Heading normalization MUST NOT add trailing newlines

**Non-Goals:**
- No changes to the public API
- No changes to L4 output format
- No changes to token estimation (len/4)

## Decisions

### Table compression: positional format with size guard
**Choice**: Emit values space-separated without column names for multi-column tables. Keep `ColumnName:value` only for single-column tables. Always compare compressed vs original size and skip if larger.
**Rationale**: Positional format saves ~25 bytes per row vs column:value for 4-column tables. The size guard prevents any regression. Single-column tables benefit from column:value since there's no ambiguity.
**Council consensus**: Hybrid approach (Option D) — positional for multi-column, column:value for single-column, skip when bloated.

### L3 indentation: hierarchy-aware
**Choice**: Only indent list items that are nested under other list items. Flat lists (all items at same depth) emit without indentation.
**Rationale**: Indentation conveys nesting structure. Flat lists don't need it — the `-` prefix already marks each item. Saves 2 bytes per list item.

### Heading normalization: no trailing newline
**Choice**: Remove the `\n` from heading replacement text. The existing newline after the heading node in the source is preserved by the edit application logic.
**Rationale**: Eliminates double-newline between heading and content. Saves 1 byte per heading.

## Risks / Trade-offs

- **[Positional tables lose column context]** → LLMs must infer meaning from value order. Mitigation: the table header row is still present in the output (as the first positional row), so order is documented.
- **[Size guard adds complexity]** → Each table now requires a trial compression + comparison. Mitigation: the comparison is O(rows) and cheap.
- **[L3 indentation change may break users expecting indented output]** → L3 is documented as "not valid Markdown" — the format is already unstable.
