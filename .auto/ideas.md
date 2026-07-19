# Deferred Ideas

## Harness improvements (eval.py)
- Handle bare URLs `<url>` in extract_links (currently only handles `[text](url)`)
- Lower min file size from 100 to 50 bytes (or handle empty files gracefully)
- Add abbreviation map auto-generation from grammar.rs patterns

## mdmin engine improvements
- Consider adding a `--check` mode that runs the harness internally
- Profile L2 compression to find bottlenecks
- Investigate if grammar strip can use word-boundary matching instead of substring matching
- Add more technical abbreviations to grammar.rs (e.g., "specification" was missing)
