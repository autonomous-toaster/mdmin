## Context

Three remaining improvements from the original design and recent research. Analysis across 200+ skill files shows the general n-gram dictionary finds 4.6x more candidates than the current targeted approach. Nested list flattening and inline tiny sections were planned in the original design but never implemented.

## Goals / Non-Goals

**Goals:**
- General n-gram dictionary: find ALL repeated substrings ≥15 chars, code-block-safe, same `-d` flag
- Nested list flattening: `- A\n  - B` → `A: B` at L2
- Inline tiny sections: `### Title\n\nBody.` → `title: Body.` at L2

**Non-Goals:**
- No changes to L3/L4 output format
- No changes to grammar stripping or existing dictionary API

## Decisions

### General n-gram over targeted regex
**Choice**: Replace regex-based targeted dictionary with a general substring search using a rolling hash (Rabin-Karp) for O(n) average-time candidate detection.
**Rationale**: Targeted regex misses 4.6x more candidates. A rolling hash is efficient enough for real-time use and catches all repeated substrings regardless of pattern type.
**Validated by**: arxiv 2604.13066 (dictionary-encoding paper) and arxiv 2506.00307 (Meta-Tokens).

### Nested list flattening at L2
**Choice**: Detect nested list items in the tree-sitter walk. When a list item contains another list as its only child, flatten: `- A\n  - B` → `A: B`.
**Rationale**: Nested lists are structural redundancy. The parent item's text becomes a label, children become comma-separated inline items.

### Inline tiny sections at L2
**Choice**: When a heading is immediately followed by a single short paragraph (≤100 chars), merge them: `### Title\n\nBody.` → `title: Body.`
**Rationale**: Tiny sections are common in specs. The heading text becomes a label, the paragraph becomes the value.

## Risks / Trade-offs

- **[Rolling hash collisions]** → Extremely rare with a good hash function. Verify with string comparison on match.
- **[Nested list flattening changes structure]** → Only flattens when the parent has no text of its own (pure container). Preserves parent text when present.
- **[Inline tiny sections may merge unrelated content]** → Only merges when the paragraph is short (≤100 chars) and directly follows the heading. Conservative threshold.
