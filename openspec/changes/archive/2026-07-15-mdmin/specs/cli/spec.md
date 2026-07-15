## ADDED Requirements

### Task Reference

| Task ID | Description |
|---------|-------------|
| T3.1 | Implement CLI argument parsing with clap |
| T3.2 | Implement file/stdin I/O |
| T3.3 | Implement env var support |
| T3.4 | Implement stats output on stderr |

### Requirement: CLI reads from file or stdin

T3.2 SHALL complete BEFORE T3.4 SHALL run.

#### Scenario: File argument is read
- **WHEN** T3.2 is invoked with `mdmin doc.md`
- **THEN** the content of `doc.md` SHALL be read and processed

#### Scenario: Stdin is read when no file given
- **WHEN** T3.2 is invoked with `cat doc.md | mdmin -l 2`
- **THEN** the content SHALL be read from stdin

### Requirement: CLI accepts level via flag and env var

T3.1 SHALL complete BEFORE T3.3 SHALL run. T3.3 SHALL complete BEFORE T3.2 SHALL run.

#### Scenario: Level flag is used
- **WHEN** T3.1 is invoked with `mdmin -l 3 doc.md`
- **THEN** the minifier SHALL use Level 3

#### Scenario: Env var is used without flag
- **WHEN** T3.3 is invoked with `MDMIN_LEVEL=3 mdmin doc.md`
- **THEN** the minifier SHALL use Level 3

#### Scenario: Flag overrides env var
- **WHEN** T3.3 is invoked with `MDMIN_LEVEL=1 mdmin -l 3 doc.md`
- **THEN** the minifier SHALL use Level 3

### Requirement: CLI accepts code block mode via flag and env var

T3.1 SHALL complete BEFORE T3.3 SHALL run.

#### Scenario: Code block flag is used
- **WHEN** T3.1 is invoked with `mdmin -c compress doc.md`
- **THEN** code blocks SHALL be whitespace-compressed

### Requirement: CLI writes to stdout by default

T3.2 SHALL complete BEFORE T3.4 SHALL run.

#### Scenario: Output goes to stdout
- **WHEN** T3.2 minifies input
- **THEN** the output SHALL be written to stdout

### Requirement: CLI shows stats on stderr with -s flag

T3.4 SHALL complete AFTER T3.2 SHALL complete.

#### Scenario: Stats are printed
- **WHEN** T3.4 is invoked with `mdmin -s doc.md`
- **THEN** stderr SHALL contain `tokens: N → M (X% savings)`

### Requirement: CLI supports -o flag for file output

T3.2 SHALL complete BEFORE T3.4 SHALL run.

#### Scenario: Output file is written
- **WHEN** T3.2 is invoked with `mdmin -o out.md doc.md`
- **THEN** the output SHALL be written to `out.md`
