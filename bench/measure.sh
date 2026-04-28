#!/usr/bin/env bash
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$REPO_ROOT"

ITERS=5
MODES=("debug" "release")
CRATES=("serde_bench" "typeinfo_bench")

# Warmup: full build so all deps are compiled
cargo build -p serde_bench >/dev/null 2>&1
cargo build -p typeinfo_bench >/dev/null 2>&1
cargo build --release -p serde_bench >/dev/null 2>&1
cargo build --release -p typeinfo_bench >/dev/null 2>&1

for mode in "${MODES[@]}"; do
    FLAGS=""
    if [[ "$mode" == "release" ]]; then
        FLAGS="--release"
    fi
    echo "=== mode: $mode ==="
    for i in $(seq 1 "$ITERS"); do
        for crate in "${CRATES[@]}"; do
            # Force rebuild of just this crate by touching its main.rs
            touch "bench/${crate}/src/main.rs"
            START=$(date +%s.%N)
            cargo build $FLAGS -p "$crate" >/dev/null 2>&1
            END=$(date +%s.%N)
            ELAPSED=$(echo "$END - $START" | bc -l)
            printf "%s\t%s\t%d\t%.3f\n" "$mode" "$crate" "$i" "$ELAPSED"
        done
    done
done
