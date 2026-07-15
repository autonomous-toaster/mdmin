## MODIFIED Requirements

### Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Fix table compression to use positional format with size guard |
| T1.2 | Fix heading normalization to not add trailing newline |
| T1.3 | Fix L3 indentation to only indent nested lists |

### Requirement: Table compression uses positional format

T1.1 SHALL change table compression from `ColumnName:value` to space-separated values for multi-column tables. T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Multi-column table uses positional format
- **WHEN** T1.1 compresses a table with 3+ columns
- **THEN** the output SHALL use space-separated values without column names

#### Scenario: Single-column table uses column:value
- **WHEN** T1.1 compresses a table with 1 column
- **THEN** the output SHALL use `ColumnName:value` format

### Requirement: Table compression skips when it would bloat

T1.1 SHALL compare compressed size to original size and skip compression when it would increase byte count. T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Compression skipped on bloat
- **WHEN** T1.1 compresses a table where positional format is larger than original
- **THEN** the original table SHALL be emitted unchanged

### Requirement: Heading normalization does not add trailing newline

T1.2 SHALL remove the trailing `\n` from heading replacement text. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: No double newline after heading
- **WHEN** T1.2 normalizes a heading
- **THEN** the output SHALL NOT contain a double newline between heading and content

### Requirement: L3 indentation is hierarchy-aware

T1.3 SHALL only indent list items that are nested under other list items. T1.3 SHALL complete AFTER T1.2 SHALL complete.

#### Scenario: Flat list has no indentation
- **WHEN** T1.3 processes a flat list (all items at same depth)
- **THEN** the output SHALL NOT add indentation to list items

#### Scenario: Nested list has indentation
- **WHEN** T1.3 processes a nested list (items at different depths)
- **THEN** the output SHALL indent child items under their parent
