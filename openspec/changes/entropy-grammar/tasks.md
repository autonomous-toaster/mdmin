## 1. Entropy Grammar

- [x] 1.1 Replace binary word lists with frequency scoring table (`HashMap<&str, f64>`)
- [x] 1.2 Add short word heuristic (≤3 chars, not proper noun → +0.3) and sentence position heuristic (first word → -0.5)
- [x] 1.3 Add configurable threshold levels (Light 0.8, Medium 0.6, Aggressive 0.4) and update CLI
- [x] 1.4 Add protection list for temporal/spatial words (before, after, between, etc.)
- [x] 1.5 Fix hyphen handling — treat hyphens as word characters
