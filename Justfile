set quiet

# Run all checks (mirrors CI)
[parallel]
ci: veriplan check lint check-file-sizes machete crap test 
build: cargo-build

# Fast compile check — all targets, all workspace crates
check:
    #!/usr/bin/env bash
    if output=$(cargo check --workspace --all-targets 2>&1); then
        echo "✓ check passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# Build (dev profile)
cargo-build:
    #!/usr/bin/env bash
    if output=$(cargo build --workspace 2>&1); then
        echo "✓ build passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# Run tests — show summary on success, full output on failure
test:
    #!/usr/bin/env bash
    output=$(cargo test --workspace 2>&1)
    code=$?
    if [ $code -eq 0 ]; then
        printf '%s\n' "$output" | grep -E "^cargo test:" || echo "✓ tests passed"
    else
        printf '%s\n' "$output"
        exit $code
    fi

# Clippy — deny all,pedantic,nursery (matches workspace config)
lint:
    #!/usr/bin/env bash
    if output=$(cargo clippy --workspace --all-targets -- -Dwarnings 2>&1); then
        echo "✓ lint passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

[group('optional')]
veriplan:
    #!/usr/bin/env bash
    if command -v veriplan >/dev/null 2>&1; then
        if output=$(veriplan check 2>&1); then
            echo "✓ veriplan passed"
        else
            printf '%s\n' "$output"
            exit 1
        fi
    else
        echo "⚠ veriplan skipped (veriplan not installed)"
        exit 0
    fi


# Check format without modifying files
fmt:
    #!/usr/bin/env bash
    if output=$(cargo fmt --check 2>&1); then
        echo "✓ fmt passed"
    else
        printf '%s\n' "$output"
        echo "→ fix with: cargo fmt"
        exit 1
    fi

# Unused dependency check
machete:
    #!/usr/bin/env bash
    if output=$(cargo machete 2>&1); then
        echo "✓ machete passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# CRAP complexity — generates coverage then scores; fails if any function exceeds threshold 30.
# Outputs JSON, filters with jq to show only functions above threshold.
crap:
    #!/usr/bin/env bash
    set -o pipefail
    LCOV=/tmp/lcov-crap.info
    if ! cargo llvm-cov --workspace \
        --lcov --output-path "$LCOV" \
        --ignore-filename-regex 'main\.rs' \
        --bins --tests --quiet 2>/dev/null; then
        exit 1
    fi

    json=$(cargo crap --workspace --lcov "$LCOV" \
        --threshold 30 \
        --exclude 'tests/**' --exclude 'src/**/main.rs' \
        --missing skip --format json 2>/dev/null)
    code=$?

    # Filter functions with CRAP > 30 using jq
    crappy=$(echo "$json" | jq '[.entries[] | select(.crap > 30)]' 2>/dev/null)
    count=$(echo "$crappy" | jq 'length' 2>/dev/null)
    count=${count:-0}

    if [ "$count" -gt 0 ] 2>/dev/null; then
        echo "✗ $count function(s) exceed CRAP threshold 30:"
        echo "$crappy" | jq -r '.[] | "  CRAP=\(.crap | floor)  cyclomatic=\(.cyclomatic | floor)  coverage=\(.coverage | floor)%  \(.function)  \(.file):\(.line)"'
        exit 1
    else
        echo "✓ crap passed"
    fi


# Check that no production source file exceeds the target line limit.
# `max` is the soft target (default 500); `tolerance` adds a small grace margin (default 10%).
# Files under tests/ directories are excluded.
check-file-sizes max="500" tolerance="10":
    #!/usr/bin/env bash
    TARGET={{max}}
    TOL={{tolerance}}
    MAX=$(( TARGET + TARGET * TOL / 100 ))
    fail=0
    while IFS= read -r f; do
        lines=$(wc -l < "$f")
        if [ "$lines" -gt "$MAX" ]; then
            echo "FAIL: $f has $lines lines (target $TARGET, hard limit $MAX)"
            fail=1
        fi
    done < <(find src -name '*.rs' | grep -v '/tests/')
    [ $fail -eq 0 ] && echo "✓ all source files within $MAX lines (target $TARGET + ${TOL}% tolerance)"
