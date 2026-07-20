# Deferred Ideas

## Harness improvements (eval.py)
- Handle bare URLs `<url>` in extract_links (currently only handles `[text](url)`)
- Add abbreviation map auto-generation from grammar.rs patterns (DONE)

## mdmin engine improvements
- Consider adding a `--check` mode that runs the harness internally
- Profile L2 compression to find bottlenecks
- **Word-boundary matching in grammar** (DONE)
- **Semantic deduplication** — detect repeated phrases across a document and replace 2nd+ occurrences with `[see §X]` references. Distinct from dictionary (which needs 15+ char, 4+ occurrences).
- **Token-aware optimization** — optimize for specific tokenizers (cl100k_base). Some byte-level optimizations don't translate to token savings.
- **Multi-file dictionary** — share dictionary across files in a corpus for better compression of repeated patterns.
