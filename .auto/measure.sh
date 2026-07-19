#!/bin/bash
set -euo pipefail

# mdmin benchmark — outputs METRIC lines for autoresearch
cd "$(dirname "$0")/.."

# Quick pre-check: build must succeed
cd mdmin && cargo build --bin mdmin-bench 2>&1 > /dev/null || {
    echo "BUILD FAILED"
    exit 1
}
cd "$OLDPWD"

# Set corpus path
export MDMIN_BENCH_CORPUS="/Users/jean-christophe.saad-dupuy2/src/github.com/jcsaaddupuy/badrobots/skills"

# Run the benchmark
HUMAN_OUTPUT=$(cd mdmin && cargo run --bin mdmin-bench 2>/dev/null)

# Parse with python3 (macOS compatible)
python3 -c "
import re, sys

output = '''$HUMAN_OUTPUT'''

# Duration
m = re.search(r'Duration: (\d+)', output)
duration = int(m.group(1)) if m else 60000

# Monotonicity
mono = 1 if 'No violations' in output else 0

# Dictionary reversibility
m = re.search(r'Files with dict:\s+(\d+)', output)
dict_files = int(m.group(1)) if m else 0
m = re.search(r'Fully reversible:\s+(\d+)', output)
dict_rev = int(m.group(1)) if m else 0

# Grammar unexpected
m = re.search(r'Unexpected:\s+(\d+)', output)
grammar_unexpected = int(m.group(1)) if m else 0

# L2_gd savings
m = re.search(r'L2_gd\s+\d+\s+tokens\s+\d+\s+bytes\s+\(\s*([\d.]+)%\)', output)
l2gd = float(m.group(1)) if m else 0.0

# Corpus bytes
m = re.search(r'Corpus: \d+ files \((\d+)\)', output)
corpus_bytes = int(m.group(1)) if m else 1000000

# Throughput
throughput = int(corpus_bytes * 1000 / duration) if duration > 0 else 0

# Composite metric
tokens_per_sec = round(l2gd * throughput / 1000, 2)

print(f'METRIC token_savings_pct={l2gd}')
print(f'METRIC throughput_bytes_per_sec={throughput}')
print(f'METRIC tokens_per_sec={tokens_per_sec}')
print(f'METRIC monotonicity_violations={mono}')
print(f'METRIC dictionary_reversibility={dict_rev}')
print(f'METRIC grammar_unexpected={grammar_unexpected}')
"
