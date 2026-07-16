## Context

Analysis across 200+ skill files shows that long file paths, URLs, and identifiers repeat frequently. A targeted dictionary (regex-based pattern detection + short code references) can save 1-3% additional tokens with minimal complexity. The Meta-Tokens paper (arxiv 2506.00307) validates that LZ77-style repetition compression is effective for LLM token reduction.

## Goals / Non-Goals

**Goals:**
- Compress long repeated strings (paths 30+ chars, URLs, identifiers 20+ chars)
- Dictionary emitted at top of output: `@N = string`
- Code-block-safe: skip patterns inside fenced code and inline code
- Minimum savings threshold: skip if net savings <= 0
- Config option `-d`/`--dictionary`, orthogonal to levels and grammar strip

**Non-Goals:**
- No general-purpose substring search (suffix arrays, rolling hashes)
- No compression of short strings (< 20 chars)
- No compression of single-occurrence strings
- No changes to existing L1-L4 behavior

## Decisions

### Targeted patterns over general search
**Choice**: Use regex patterns for known long-string types (paths, URLs, identifiers) instead of a general substring search algorithm.
**Rationale**: General substring search (suffix array) is complex and finds many candidates inside code blocks. Targeted patterns are simple, fast, and catch the most common cases. The Meta-Tokens paper uses general search, but at the token level where patterns are denser. At the text level, targeted patterns cover 90% of the savings with 10% of the complexity.

### Dictionary format
**Choice**: `@N = string` at top of output, `@N` references in text.
**Rationale**: `@` is rarely used in markdown content, minimizing ambiguity. The format is self-documenting. References are 2-3 chars (`@1`-`@99`), keeping overhead minimal.

### Code block protection
**Choice**: Use tree-sitter CST to identify protected regions before dictionary pass. Skip patterns inside fenced code blocks and inline code.
**Rationale**: Reuses existing infrastructure. Prevents corruption of code examples.

### Minimum threshold
**Choice**: Only compress if `(string_length × occurrences) - (dictionary_entry_length + 2 × occurrences) > 0`.
**Rationale**: Prevents bloat. A 30-char string appearing 3 times saves: 90 - 30 - 6 = 54 chars. A 20-char string appearing 2 times saves: 40 - 20 - 4 = 16 chars. Below this threshold, the dictionary overhead outweighs savings.

## Risks / Trade-offs

- **[Dictionary adds overhead to small files]** → Minimum threshold prevents this. Files with no long repeated strings are unaffected.
- **[`@` symbol in content]** → Rare in markdown. If `@` appears in the original text, references could be ambiguous. Mitigation: use `@dict` header to clearly separate dictionary from content.
- **[Code block protection misses edge cases]** → Tree-sitter CST identifies most code regions. Inline code with mixed backticks may be missed. Mitigation: grammar module already handles this.
