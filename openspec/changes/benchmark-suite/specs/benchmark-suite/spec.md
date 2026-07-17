## NEW Capability: benchmark-suite

### Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Add tiktoken-rs dev-dependency and create benchmark skeleton |
| T1.2 | Implement token savings and monotonicity suites |
| T1.3 | Implement structure preservation suite |
| T1.4 | Implement grammar strip coverage suite |
| T1.5 | Implement dictionary reversibility suite |
| T1.6 | Add corpus access and JSON report output |

### Requirement: Token savings uses real tokenizer

T1.2 SHALL use tiktoken-rs (cl100k_base encoding) for token counting. T1.2 SHALL complete BEFORE T1.6 SHALL run.

#### Scenario: Token count matches expected range
- **WHEN** T1.2 counts tokens in a known text
- **THEN** the count SHALL match tiktoken's cl100k_base encoding

### Requirement: Monotonicity is verified

T1.2 SHALL verify L0 <= L1 <= L2 <= L3 <= L4 for every corpus file. T1.2 SHALL complete BEFORE T1.6 SHALL run.

#### Scenario: Monotonicity violation is reported
- **WHEN** T1.2 finds L3 > L2 for a file
- **THEN** the violation SHALL be reported with file name and sizes

### Requirement: Structure preservation is measured

T1.3 SHALL parse original and compressed Markdown, extract headings (##, ###), list items (-, *), and table rows (|), and compare. T1.3 SHALL complete BEFORE T1.6 SHALL run.

#### Scenario: Headings are preserved
- **WHEN** T1.3 compresses a file with headings
- **THEN** the heading text SHALL match between original and compressed output

### Requirement: Grammar strip coverage is verified

T1.4 SHALL collect every word removed by -g and verify it's in the frequency table or protection lists. T1.4 SHALL complete BEFORE T1.6 SHALL run.

#### Scenario: Unexpected removal is flagged
- **WHEN** T1.4 finds a removed word not in the frequency table
- **THEN** it SHALL be reported as an unexpected removal

### Requirement: Dictionary reversibility is verified

T1.5 SHALL expand @N references and verify they match the pre-dictionary text. T1.5 SHALL complete BEFORE T1.6 SHALL run.

#### Scenario: Dictionary is fully reversible
- **WHEN** T1.5 expands all @N references in compressed output
- **THEN** the result SHALL match the text before dictionary compression
