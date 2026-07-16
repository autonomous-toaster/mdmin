## Context

Current grammar stripping uses binary word lists (HashSet of articles, filler, aux verbs, etc.). This approach has two limitations: (1) words not in the lists are never removed even if they're obvious filler, and (2) words in the lists are always removed even if used meaningfully. An entropy-based approach with continuous scoring and heuristics addresses both issues.

## Validation

Tested against 5 OpenSpec spec documents from the baish project. The initial frequency table was too broad — it included prepositions and temporal words that carry semantic meaning in specs. Key findings:

| Word | Score | Old behavior | New behavior (Medium) | Impact |
|------|-------|-------------|----------------------|--------|
| `before` | 0.65 | Kept | Removed | Temporal constraint lost in `T1.1 SHALL complete BEFORE T1.2` |
| `after` | 0.70 | Kept | Removed | Same issue |
| `at` | 0.85 | Kept | Removed | Structural, safe to remove |
| `in` | 0.94 | Kept | Removed | Structural, safe to remove |
| `from` | 0.83 | Kept | Removed | Structural, safe to remove |
| `for` | 0.90 | Kept | Removed | Structural, safe to remove |
| `with` | 0.88 | Kept | Removed | Structural, safe to remove |

**Conclusion**: The frequency table must only include words that are safe to remove in any context. Prepositions and temporal words carry semantic meaning and should never be removed by grammar stripping.

## Goals / Non-Goals

**Goals:**
- Replace binary word lists with continuous frequency scoring
- Add short word heuristic (≤3 chars → higher removal score)
- Add sentence position heuristic (first word → protected)
- Add configurable threshold: `-g light|medium|aggressive`
- Keep existing: negation scope, code block protection, RFC 2119 protection
- **Protect temporal/spatial words** (before, after, between, etc.) from removal
- **Fix hyphen handling** — treat hyphens as word characters to prevent word splitting

**Non-Goals:**
- No n-gram context model (Level 3) — too heavy for CLI tool
- No ML-based scoring
- No changes to L1-L4 structural compression
- No auto-detection of spec documents — protection list is simpler and more robust

## Decisions

### Frequency scoring over binary lists
**Choice**: Replace `HashSet<&str>` with `HashMap<&str, f64>` where values are log-frequency scores. Words with score above threshold are removed.
**Rationale**: Continuous scoring catches filler words not in the current lists and allows configurable aggressiveness. The frequency table is based on standard English word frequency lists (e.g., Google Books Ngrams, SUBTLEX).

### Frequency table scope
**Choice**: The frequency table SHALL only include words from the original categories: articles, filler adverbs, auxiliary verbs, hedging, conjunctions, pronouns, and determiners. Prepositions and temporal/spatial words SHALL NOT be included.
**Rationale**: Validated against real OpenSpec documents. Prepositions (at, in, from, for, with, by, on) and temporal words (before, after, during, until) carry semantic meaning in spec documents. Removing them changes the meaning of temporal constraints like `T1.1 SHALL complete BEFORE T1.2`.

### Protection list for temporal/spatial words
**Choice**: Add a `PROTECTED` list of words that are never removed regardless of frequency score.
**Rationale**: Some words (before, after, between, within, without, above, below) are common enough to appear in a frequency table but carry too much semantic meaning to remove. A protection list is simpler and more robust than auto-detection of spec documents.

### Hyphen handling
**Choice**: Treat hyphens as word characters in addition to alphanumeric, apostrophe, and underscore.
**Rationale**: Words like "built-in" are split by the hyphen into "built" and "in". The "in" is then removed as a common word, producing garbled output like "built-plugins". Treating hyphens as word characters prevents this.

### Short word heuristic
**Choice**: Words of ≤3 characters that aren't proper nouns (not capitalized in context) get a +0.3 score bonus.
**Rationale**: Short words in English are overwhelmingly function words (articles, prepositions, pronouns). The heuristic catches words like "at", "by", "for", "in", "of", "on", "to", "up" that may not be in the current word lists. Note: this heuristic is applied AFTER the protection list check, so protected words are never removed.

### Sentence position heuristic
**Choice**: The first word of each sentence gets a -0.5 score penalty (protected from removal).
**Rationale**: Sentence-initial words often carry discourse-structuring information ("However", "Therefore", "First"). Removing them can change meaning.

### Configurable threshold
**Choice**: Three levels: Light (threshold 0.8, only very common words), Medium (threshold 0.6, default), Aggressive (threshold 0.4, removes moderately common words).
**Rationale**: Users can choose their risk tolerance. Light is safe for any document. Aggressive may remove borderline words.

## Risks / Trade-offs

- **[Frequency table is English-only]** → Same limitation as current word lists. `whatlang` detection can disable entropy scoring for non-English text and fall back to current behavior.
- **[Short word heuristic may remove meaningful short words]** → "go", "do", "be" are short but meaningful. Mitigation: the heuristic is a bonus, not a guarantee. The frequency score still applies.
- **[Sentence boundary detection is imperfect]** → Simple heuristic: split on `. ! ?` followed by space. May miss edge cases like "Dr." or "U.S." Mitigation: acceptable for a minifier.
- **[Protection list is static]** → New temporal/spatial words may be added to the language. Mitigation: the list is curated and can be extended as needed.
