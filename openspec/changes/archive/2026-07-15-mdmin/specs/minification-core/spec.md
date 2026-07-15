## ADDED Requirements

### Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Create crate structure with Cargo.toml and source files |
| T1.2 | Add tree-sitter and tree-sitter-markdown dependencies |
| T1.3 | Define public API types |
| T2.1 | Implement tree-sitter Markdown parse/serialize cycle |
| T2.2 | Implement Level 0 as no-op |
| T2.3 | Implement Level 1 transform passes |
| T2.4 | Implement Level 2 transform passes |
| T2.5 | Implement Level 3 transform passes |
| T2.6 | Implement Level 4 transform passes |
| T2.7 | Implement code block handling |
| T2.8 | Implement token estimation and stats computation |

### Requirement: Parse/serialize cycle is implemented before transform passes

T2.1 SHALL complete BEFORE T2.3 SHALL run.

#### Scenario: Parse and serialize round-trips
- **WHEN** T2.1 parses and immediately serializes without transformations
- **THEN** the output SHALL match the input exactly

#### Scenario: Malformed input produces error nodes
- **WHEN** T2.1 processes malformed Markdown
- **THEN** the CST SHALL contain error nodes rather than failing

### Requirement: Level 0 no-op is implemented before parse/serialize

T2.2 SHALL complete BEFORE T2.1 SHALL run.

#### Scenario: Level 0 returns input unchanged
- **WHEN** T2.2 is configured with Level 0
- **THEN** the output SHALL be byte-identical to the input

### Requirement: Code block handling is implemented before transform passes

T2.7 SHALL complete BEFORE T2.3 SHALL run.

#### Scenario: Preserve mode leaves code blocks unchanged
- **WHEN** T2.7 is configured with `Preserve`
- **THEN** fenced code blocks SHALL appear verbatim in the output

#### Scenario: CompressWhitespace collapses blank lines
- **WHEN** T2.7 is configured with `CompressWhitespace`
- **THEN** runs of blank lines within code blocks SHALL be collapsed to one

### Requirement: Token estimation is implemented before transform passes

T2.8 SHALL complete BEFORE T2.3 SHALL run.

#### Scenario: Stats are computed
- **WHEN** T2.8 minifies input
- **THEN** the result SHALL include `input_tokens`, `output_tokens`, and `savings_pct`

### Requirement: Level 1 strips decorative formatting

T2.3 SHALL complete BEFORE T2.4 SHALL run.

#### Scenario: Bold text is stripped
- **WHEN** T2.3 processes `**important**`
- **THEN** the output SHALL contain `important` without `**`

#### Scenario: Horizontal rules are removed
- **WHEN** T2.3 processes a `---` horizontal rule
- **THEN** the output SHALL NOT contain the `---` line

#### Scenario: HTML comments are removed
- **WHEN** T2.3 processes `<!-- comment -->`
- **THEN** the output SHALL NOT contain the comment

### Requirement: Level 2 applies semantic compression

T2.4 SHALL complete BEFORE T2.5 SHALL run.

#### Scenario: Headings are normalized
- **WHEN** T2.4 processes `# Installation`
- **THEN** the output SHALL contain `installation:` (lowercase, colon suffix)

#### Scenario: Nested lists are flattened
- **WHEN** T2.4 processes `- Feature\n  - Sub`
- **THEN** the output SHALL contain `Feature: Sub`

#### Scenario: Tables are compressed
- **WHEN** T2.4 processes a Markdown table
- **THEN** the output SHALL use `key:value` format per row

### Requirement: Level 3 emits TOON-like structured output

T2.5 SHALL complete BEFORE T2.6 SHALL run.

#### Scenario: Sections become indented blocks
- **WHEN** T2.5 processes `## Authentication\n- JWT\n- OAuth`
- **THEN** the output SHALL use indentation to show nesting

#### Scenario: Checklists use +/- notation
- **WHEN** T2.5 processes `- [x] Login\n- [ ] Logout`
- **THEN** the output SHALL use `+ Login` and `- Logout`

### Requirement: Level 4 emits ultra-compact output

T2.6 SHALL complete AFTER T2.5 SHALL complete.

#### Scenario: Groups use brace notation
- **WHEN** T2.6 processes `## Authentication\n- JWT\n- OAuth`
- **THEN** the output SHALL use `auth{jwt oauth}` format

#### Scenario: Optional whitespace is removed
- **WHEN** T2.6 processes any input
- **THEN** the output SHALL minimize whitespace between structural elements
