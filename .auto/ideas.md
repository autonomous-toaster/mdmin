# Deferred Ideas

## Harness improvements (eval.py)
- Handle bare URLs `<url>` in extract_links (currently only handles `[text](url)`)
- Lower min file size from 100 to 50 bytes (or handle empty files gracefully)
- Add abbreviation map auto-generation from grammar.rs patterns

## mdmin engine improvements
- Investigate if `specification` → `specation` is a bug (should be `spec` not `specation`)
- Consider adding a `--check` mode that runs the harness internally
- Profile L2 compression to find bottlenecks
