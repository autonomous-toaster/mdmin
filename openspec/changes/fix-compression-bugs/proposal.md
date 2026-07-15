## Why

mdmin's table compression and L3 indentation passes can make output **larger** than input for real-world files. Tested across 200+ skill files (badrobots, pi-perso, anthropic), 15+ files show L2 > L1 byte counts, with the worst case (mealie/SKILL.md) bloating by +636B (+5%). The root cause: `ColumnName:value` prepending adds more bytes than it saves for multi-column tables, and L3 indentation adds 2 spaces per list item unconditionally.

## What Changes

- **Table compression**: Switch from `ColumnName:value` to positional format (space-separated values) for multi-column tables. Skip compression entirely when it would increase byte count.
- **L3 indentation**: Only indent list items when they are nested under other list items (actual hierarchy). Flat lists emit without indentation.
- **Heading `\n`**: Remove trailing `\n` from heading replacement to avoid double newlines.
- **No breaking changes** to the public API — only internal algorithm changes.

## Capabilities

### New Capabilities
- *(none — all changes are modifications to existing `minification-core`)*

### Modified Capabilities
- `minification-core`: Table compression strategy changed from column:value to positional; L3 indentation changed from unconditional to hierarchy-aware; heading normalization no longer adds trailing newline

## Impact

- `mdmin/src/engine.rs`: Fix `handle_table` to use positional format + size guard; fix `handle_atx_heading` to not add `\n`
- `mdmin/src/passes.rs`: Fix `apply_level_3` to only indent nested lists
- No dependency changes, no API changes
