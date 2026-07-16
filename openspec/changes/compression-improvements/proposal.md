## Why

Three remaining high-ROI improvements from the original design and recent research: (1) general n-gram dictionary compression finds 4.6x more repeated patterns than the current targeted approach, (2) nested list flattening compresses hierarchical bullets into inline format, and (3) inline tiny sections merges short headings with their single-paragraph body. Together they add ~5-8% additional token savings on typical documents.

## What Changes

- **General n-gram dictionary**: Replace targeted regex-based dictionary with a general substring search (suffix array or rolling hash). Finds ALL repeated substrings ≥15 chars, not just paths/URLs. Code-block-safe. Same `-d` flag and `@dict` format.
- **Nested list flattening**: `- Feature\n  - Sub` → `Feature: Sub`. Applied at L2.
- **Inline tiny sections**: `### Goal\n\nBuild API.` → `goal: Build API.` Applied at L2.

## Capabilities

### New Capabilities
- *(none — all modifications to existing `minification-core`)*

### Modified Capabilities
- `minification-core`: Replace targeted dictionary with general n-gram; add nested list flattening; add inline tiny sections

## Impact

- `mdmin/src/dictionary.rs`: Rewrite — replace regex patterns with general substring search, add code block protection
- `mdmin/src/engine.rs`: Add nested list flattening and inline tiny sections to L2 walk
- No dependency changes, no API changes (same `-d` flag)
