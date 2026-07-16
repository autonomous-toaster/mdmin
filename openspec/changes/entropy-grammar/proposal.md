## Why

Current grammar stripping uses binary word lists (a word is either "remove" or "keep"). This misses filler words not in the lists and can't distinguish between a common word used as filler vs. the same word used meaningfully. An entropy-based approach scores each word by its information content using frequency, length, and position heuristics, enabling configurable aggressiveness and better accuracy. Council-validated as Level 2 (heuristics) — the best ROI between simple word lists and heavy n-gram models.

## What Changes

- Replace binary word lists with a continuous frequency scoring table
- Add short word heuristic (words ≤3 chars that aren't proper nouns → higher removal score)
- Add sentence position heuristic (first word of sentence → protected from removal)
- Add configurable threshold: `-g light|medium|aggressive` (default: medium)
- Keep existing: negation scope tracking, code block protection, RFC 2119 protection

## Capabilities

### New Capabilities
- *(none — modification to existing `grammar-stripping`)*

### Modified Capabilities
- `minification-core`: Replace binary word lists with entropy-based scoring; add configurable threshold levels

## Impact

- `mdmin/src/grammar.rs`: Replace `HashSet` word lists with `HashMap<word, score>` frequency table; add heuristic scoring functions; add threshold levels
- `mdmin/src/lib.rs`: Update `Config` to support `GrammarLevel` enum (Light/Medium/Aggressive)
- `mdmin/src/main.rs`: Update `-g` flag to accept optional level value
