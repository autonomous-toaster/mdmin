## 1. Grammar Stripping

- [x] 1.1 Add `whatlang` and `stop-words` dependencies to `Cargo.toml`
- [x] 1.2 Implement word lists (articles, filler, aux verbs, hedging, conjunctions) and replacement patterns per language in `src/grammar.rs`
- [x] 1.3 Implement negation-aware grammar stripping logic with code region protection
- [x] 1.4 Integrate grammar stripping pass into engine pipeline (after structural transforms, before L3/L4)
- [x] 1.5 Add `grammar_strip` to `Config`, `-g`/`--grammar-strip` to CLI
