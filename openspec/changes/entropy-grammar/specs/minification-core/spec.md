## MODIFIED Requirements

### Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Replace binary word lists with frequency scoring table |
| T1.2 | Add short word and sentence position heuristics |
| T1.3 | Add configurable threshold levels (light/medium/aggressive) |
| T1.4 | Add protection list for temporal/spatial words |
| T1.5 | Fix hyphen handling in word boundary detection |

### Requirement: Frequency scoring replaces binary word lists

T1.1 SHALL replace `HashSet` word lists with a `HashMap<&str, f64>` frequency table. T1.1 SHALL complete BEFORE T1.2 SHALL run.

#### Scenario: Common word is scored high
- **WHEN** T1.1 scores "the" against the frequency table
- **THEN** the score SHALL be above 0.8 (very common)

#### Scenario: Rare word is scored low
- **WHEN** T1.1 scores "xylophone" against the frequency table
- **THEN** the score SHALL be 0.0 (not in frequency table)

### Requirement: Frequency table excludes prepositions and temporal words

T1.1 SHALL NOT include prepositions (at, in, from, for, with, by, on, etc.) or temporal words (before, after, during, until, etc.) in the frequency table. T1.1 SHALL complete BEFORE T1.4 SHALL run.

#### Scenario: Preposition is not in frequency table
- **WHEN** T1.1 checks "before" against the frequency table
- **THEN** the score SHALL be 0.0 (not in frequency table)

### Requirement: Short word heuristic adds score bonus

T1.2 SHALL add a +0.3 score bonus to words of ≤3 characters that are not proper nouns. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Short word gets bonus
- **WHEN** T1.2 scores "at" (3 chars, not capitalized)
- **THEN** the score SHALL include a +0.3 bonus

#### Scenario: Capitalized short word is protected
- **WHEN** T1.2 scores "Go" (capitalized, proper noun)
- **THEN** the score SHALL NOT include the short word bonus

### Requirement: Sentence position heuristic protects first words

T1.2 SHALL apply a -0.5 penalty to the first word of each sentence. T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Sentence-initial word is protected
- **WHEN** T1.2 scores "However" at the start of a sentence
- **THEN** the score SHALL include a -0.5 penalty

### Requirement: Configurable threshold levels

T1.3 SHALL support three threshold levels: Light (0.8), Medium (0.6), Aggressive (0.4). T1.3 SHALL complete AFTER T1.2 SHALL complete.

#### Scenario: Light threshold keeps most words
- **WHEN** T1.3 is configured with Light threshold
- **THEN** only words with score ≥ 0.8 SHALL be removed

#### Scenario: Aggressive threshold removes more words
- **WHEN** T1.3 is configured with Aggressive threshold
- **THEN** words with score ≥ 0.4 SHALL be removed

### Requirement: Protection list prevents removal of temporal/spatial words

T1.4 SHALL add a protection list of words that are never removed regardless of frequency score. T1.4 SHALL complete AFTER T1.1 SHALL complete.

#### Scenario: Protected word is never removed
- **WHEN** T1.4 checks "before" against the protection list
- **THEN** the word SHALL NOT be removed even if its frequency score is above the threshold

### Requirement: Hyphens are treated as word characters

T1.5 SHALL treat hyphens as word characters in addition to alphanumeric, apostrophe, and underscore. T1.5 SHALL complete AFTER T1.1 SHALL complete.

#### Scenario: Hyphenated word is preserved
- **WHEN** T1.5 processes "built-in"
- **THEN** "built-in" SHALL be treated as a single word, not split into "built" and "in"
