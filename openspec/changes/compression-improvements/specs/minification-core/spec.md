## MODIFIED Requirements

### Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Replace targeted dictionary with general n-gram dictionary |
| T1.2 | Implement nested list flattening at L2 |
| T1.3 | Implement inline tiny sections at L2 |

### Requirement: General n-gram dictionary finds all repeated substrings

T1.1 SHALL replace the regex-based targeted dictionary with a general substring search. T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Repeated phrase is compressed
- **WHEN** T1.1 processes text with a repeated 15+ char phrase
- **THEN** the phrase SHALL be replaced with an `@N` reference

#### Scenario: Code blocks are protected
- **WHEN** T1.1 processes text with a repeated string inside a fenced code block
- **THEN** the string inside the code block SHALL NOT be replaced

### Requirement: Nested lists are flattened at L2

T1.2 SHALL flatten nested list items where the parent has no text of its own. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Nested list is flattened
- **WHEN** T1.2 processes `- Feature\n  - Sub\n  - Sub2`
- **THEN** the output SHALL contain `Feature: Sub, Sub2`

### Requirement: Tiny sections are inlined at L2

T1.3 SHALL merge headings followed by a single short paragraph (≤100 chars). T1.3 SHALL complete AFTER T1.2 SHALL complete.

#### Scenario: Tiny section is inlined
- **WHEN** T1.3 processes `### Goal\n\nBuild API.\n`
- **THEN** the output SHALL contain `goal: Build API.`
