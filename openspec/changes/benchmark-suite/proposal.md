## Why

mdmin needs a rigorous benchmark suite to validate correctness and measure performance. Current testing (37 unit tests, manual validation on 222 files) is insufficient for a tool that makes deterministic transformations to structured documents. A benchmark suite provides:

- **Token savings** — real tokenizer (tiktoken) instead of len/4 approximation
- **Monotonicity** — L0 < L1 < L2 < L3 < L4 guaranteed on all inputs
- **Structure preservation** — headings, lists, tables survive compression
- **Grammar strip safety** — every removed word is in the approved frequency table
- **Dictionary reversibility** — @N references expand back to original text

## What Changes

- Add `tiktoken-rs` as a dev dependency
- Create `benches/benchmark.rs` — benchmark binary with 5 test suites
- Create `benches/corpus/` — symlinks or scripts to fetch the 222-file corpus
- Add `cargo bench` target that runs all suites and produces a report

## Capabilities

### New Capabilities
- `benchmark-suite`: Benchmark binary with 5 test suites

### Modified Capabilities
- *(none — benchmark is additive, no changes to existing code)*

## Impact

- `mdmin/Cargo.toml` — add `tiktoken-rs` dev-dependency
- `mdmin/benches/benchmark.rs` — new benchmark binary
- `mdmin/benches/corpus/` — corpus access (symlinks or download script)
- No changes to `src/` — benchmark is purely additive
