## ADDED Requirements

### Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add `whatlang` and `stop-words` dependencies |
| T1.2 | Implement word lists and replacement patterns per language |
| T1.3 | Implement negation-aware grammar stripping logic |
| T1.4 | Integrate grammar stripping into engine pipeline |
| T1.5 | Add `--grammar-strip` CLI flag and Config option |

### Requirement: Grammar stripping removes filler words

T1.3 SHALL remove articles (a/an/the), filler adverbs (just/really/basically), hedging (might/could/perhaps), aux verbs (is/are/was), and conjunctions (and/or/but) from prose text. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Articles are removed
- **WHEN** T1.3 processes "the system shall support the API"
- **THEN** the output SHALL contain "system shall support API"

#### Scenario: Filler adverbs are removed
- **WHEN** T1.3 processes "this is really very important"
- **THEN** the output SHALL contain "this is important"

### Requirement: Grammar stripping replaces verbose patterns

T1.3 SHALL replace verbose phrases with short synonyms. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Verbose patterns are shortened
- **WHEN** T1.3 processes "in order to utilize the API"
- **THEN** the output SHALL contain "to use the API"

### Requirement: Negation scope is preserved

T1.3 SHALL NOT remove any words within negation scope (following "not/never/no/nor" until end of sentence or clause boundary). T1.3 SHALL complete BEFORE T1.4 SHALL run.

#### Scenario: Negation words are preserved
- **WHEN** T1.3 processes "the system shall not allow access"
- **THEN** the output SHALL contain "not allow access"

#### Scenario: Implicit negation is preserved
- **WHEN** T1.3 processes "hardly any requests succeed"
- **THEN** the output SHALL contain "hardly requests succeed"

### Requirement: Code regions are protected

T1.3 SHALL NOT modify content inside code blocks, inline code, URLs, or file paths. T1.4 SHALL complete AFTER T1.3 SHALL complete.

#### Scenario: Code blocks are unchanged
- **WHEN** T1.3 processes text containing a fenced code block
- **THEN** the code block content SHALL be identical to the input

### Requirement: Grammar stripping is a config option

T1.5 SHALL add `grammar_strip` field to `Config` and `-g`/`--grammar-strip` flag to CLI. T1.5 SHALL complete AFTER T1.4 SHALL complete.

#### Scenario: Config option enables grammar stripping
- **WHEN** T1.5 is configured with `grammar_strip: true`
- **THEN** the grammar stripping pass SHALL run after structural transforms
