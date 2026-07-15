## Context

mdmin currently compresses markdown structure (headings, tables, lists) but ignores sentence-level verbosity. Research on caveman-compress and similar tools shows that removing filler words, articles, aux verbs, and hedging can save 15-30% additional tokens on prose-heavy documents. The challenge is doing this deterministically without a Python NLP dependency.

## Goals / Non-Goals

**Goals:**
- Grammar stripping as an orthogonal config option (`--grammar-strip`), applicable at any level
- Pure Rust implementation — no Python, no spaCy, no external runtime
- Negation-aware: never remove words in negation scope
- Code-safe: never modify content inside code blocks, inline code, URLs, or file paths
- Language-aware: use `whatlang` for detection, load per-language word lists
- Replace verbose patterns with short synonyms ("in order to" → "to", "utilize" → "use")

**Non-Goals:**
- No POS tagging (too complex for pure Rust, word lists are sufficient)
- No sentence restructuring or reordering
- No semantic analysis beyond negation scope tracking
- No changes to existing L1-L4 behavior

## Decisions

### Word lists over POS tagging
**Choice**: Use curated word lists (articles, filler, aux verbs, hedging, conjunctions) instead of POS tagging.
**Rationale**: Pure Rust POS taggers exist (`viterbi_pos_tagger`) but add complexity and model data. Word lists cover 95% of cases with zero model overhead. The caveman-compress NLP script uses spaCy POS tagging but its actual removal rules are equivalent to word lists — it just uses POS tags as a more precise filter. For a minifier, the precision gain from POS tagging doesn't justify the complexity.

### Negation scope tracking
**Choice**: Track negation scope within each sentence. When a negation marker ("not", "n't", "never", "no", "nor", "neither", "hardly", "scarcely", "without") is encountered, stop removing words until the end of the sentence or a clause boundary (`,`, `;`, `:`, `—`).
**Rationale**: Prevents meaning inversion. The caveman-nlp script has this bug — it removes "not" as a stop word, turning "system shall NOT allow" into "system allow".

### Code safety via tree-sitter CST
**Choice**: Use the existing tree-sitter CST to identify protected regions (code blocks, inline code, URLs) before grammar stripping. The grammar pass operates on plain text but skips byte ranges marked as protected.
**Rationale**: Reuses existing infrastructure. No need for a separate parser.

### Integration as post-processing pass
**Choice**: Grammar stripping runs after tree-sitter structural transforms, before L3/L4 structural passes.
**Rationale**: Operates on already-minified text. L3/L4 structural passes may add indentation/grouping that would interfere with word-level removal.

## Risks / Trade-offs

- **[False positives on technical terms]** → "the" in "the set of all integers" is a valid article but removing it changes meaning. Mitigation: only remove articles when they appear as standalone words, not as part of technical phrases. Use a small exception list for common technical patterns.
- **[Implicit negation]** → "hardly", "scarcely", "without" are less common negators that might be missed. Mitigation: include them in the negation marker list.
- **[Tonal damage]** → Removing "just", "really", "perhaps" changes tone. Mitigation: this is intentional — the feature is for LLM consumption where tone is irrelevant. Document clearly.
- **[Language detection accuracy]** → `whatlang` may misdetect language on short text segments. Mitigation: fall back to English word lists on uncertainty.
