## ADDED Requirements

### Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Implement targeted pattern detection (paths, URLs, identifiers) |
| T1.2 | Implement dictionary building, replacement, and emission |
| T1.3 | Integrate dictionary pass into engine pipeline |
| T1.4 | Add `-d`/`--dictionary` CLI flag and Config option |

### Requirement: Long paths are compressed

T1.1 SHALL detect file paths of 30+ characters that appear 3+ times. T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Repeated path is compressed
- **WHEN** T1.1 detects `/very/long/path/to/some/file.rs` appearing 3 times
- **THEN** the path SHALL be replaced with a short reference like `@1`

### Requirement: Long URLs are compressed

T1.1 SHALL detect URLs of 30+ characters that appear 2+ times. T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Repeated URL is compressed
- **WHEN** T1.1 detects `https://api.example.com/v2/users/123/profile` appearing twice
- **THEN** the URL SHALL be replaced with a short reference

### Requirement: Dictionary is emitted at top of output

T1.2 SHALL emit a dictionary header before the compressed content. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Dictionary precedes content
- **WHEN** T1.2 compresses content with 2 dictionary entries
- **THEN** the output SHALL start with `@dict:\n  @1: ...\n  @2: ...\n` followed by the content

### Requirement: Code blocks are protected

T1.2 SHALL NOT compress strings inside fenced code blocks or inline code. T1.3 SHALL complete AFTER T1.2 SHALL complete.

#### Scenario: Code block content is unchanged
- **WHEN** T1.2 processes text containing a fenced code block with a long path
- **THEN** the path inside the code block SHALL NOT be replaced

### Requirement: Minimum savings threshold

T1.2 SHALL skip compression when net savings (original - dictionary_entry - references) <= 0. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Single-occurrence string is not compressed
- **WHEN** T1.2 encounters a long string appearing only once
- **THEN** the string SHALL NOT be added to the dictionary
