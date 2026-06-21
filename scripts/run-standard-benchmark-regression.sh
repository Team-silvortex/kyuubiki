#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_HOST="${KYUUBIKI_LAB_HOST:-kyuubiki-lab}"
REMOTE_DIR="${KYUUBIKI_LAB_BENCH_DIR:-~/kyuubiki}"
PROFILE="${PROFILE:-10k}"
REPEAT="${REPEAT:-1}"
OUTPUT_SLUG="${OUTPUT_SLUG:-standard-benchmark-$(date -u +"%Y%m%dT%H%M%SZ")}"
REMOTE_REPORT_DIR="${REMOTE_REPORT_DIR:-tmp/standard-benchmark/$OUTPUT_SLUG}"
LOCAL_OUTPUT_DIR="${LOCAL_OUTPUT_DIR:-$ROOT_DIR/tmp/standard-benchmark/$OUTPUT_SLUG}"
MERGED_REPORT_LOCAL="${MERGED_REPORT_LOCAL:-$LOCAL_OUTPUT_DIR/standard-$PROFILE-compare.md}"
BENCHMARK_MEDIAN_THRESHOLD="${BENCHMARK_MEDIAN_THRESHOLD:-25}"
BENCHMARK_RSS_THRESHOLD="${BENCHMARK_RSS_THRESHOLD:-20}"
BENCHMARK_MIN_BASELINE_MS="${BENCHMARK_MIN_BASELINE_MS:-5.0}"
SYNC_TO_REMOTE="${SYNC_TO_REMOTE:-1}"
RETAIN_RUNS="${RETAIN_RUNS:-12}"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run-standard-benchmark-regression.sh

Runs the standard Rust benchmark regression trio on the shared `kyuubiki-lab`
reference machine, writes per-matrix and merged comparison reports remotely,
and copies those reports back into the local workspace.

Environment:
  KYUUBIKI_LAB_HOST              SSH host alias. Default: kyuubiki-lab
  KYUUBIKI_LAB_BENCH_DIR         Remote workspace root. Default: ~/kyuubiki
  PROFILE                        Benchmark profile. Default: 10k
  REPEAT                         Benchmark repeats. Default: 1
  OUTPUT_SLUG                    Output folder slug under tmp/standard-benchmark/
  REMOTE_REPORT_DIR              Remote output directory under the workspace
  LOCAL_OUTPUT_DIR               Local output directory
  MERGED_REPORT_LOCAL            Local path for merged Markdown report
  BENCHMARK_MEDIAN_THRESHOLD     Allowed median regression percentage. Default: 25
  BENCHMARK_RSS_THRESHOLD        Allowed peak RSS regression percentage. Default: 20
  BENCHMARK_MIN_BASELINE_MS      Ignore hard failures below this baseline median. Default: 5.0
  SYNC_TO_REMOTE                 Rsync benchmark-only source before running. Default: 1
  RETAIN_RUNS                    Local retained run directories. Default: 12
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

mkdir -p "$LOCAL_OUTPUT_DIR"

if [ "$SYNC_TO_REMOTE" = "1" ]; then
  rsync -az "$ROOT_DIR/Makefile" "$REMOTE_HOST:$REMOTE_DIR/Makefile"
  rsync -az \
    "$ROOT_DIR/scripts/build-nightly-artifact-overview.mjs" \
    "$ROOT_DIR/scripts/build-standard-benchmark-index.mjs" \
    "$ROOT_DIR/scripts/build-standard-benchmark-report.mjs" \
    "$ROOT_DIR/scripts/run-standard-benchmark-regression.sh" \
    "$REMOTE_HOST:$REMOTE_DIR/scripts/"
  rsync -az \
    "$ROOT_DIR/workers/rust/crates/benchmark/src/" \
    "$REMOTE_HOST:$REMOTE_DIR/workers/rust/crates/benchmark/src/"
  rsync -az \
    "$ROOT_DIR/workers/rust/benchmarks/" \
    "$REMOTE_HOST:$REMOTE_DIR/workers/rust/benchmarks/"
fi

ssh "$REMOTE_HOST" "cd $REMOTE_DIR && mkdir -p $(printf '%q' "$REMOTE_REPORT_DIR") && \
  make benchmark-standard-compare \
    PROFILE=$(printf '%q' "$PROFILE") \
    REPEAT=$(printf '%q' "$REPEAT") \
    BENCHMARK_MEDIAN_THRESHOLD=$(printf '%q' "$BENCHMARK_MEDIAN_THRESHOLD") \
    BENCHMARK_RSS_THRESHOLD=$(printf '%q' "$BENCHMARK_RSS_THRESHOLD") \
    BENCHMARK_MIN_BASELINE_MS=$(printf '%q' "$BENCHMARK_MIN_BASELINE_MS") && \
  make benchmark-standard-report \
    PROFILE=$(printf '%q' "$PROFILE") \
    REPEAT=$(printf '%q' "$REPEAT") \
    OUTPUT=$(printf '%q' "$REMOTE_REPORT_DIR/standard-$PROFILE-compare.md")"

scp "$REMOTE_HOST:$REMOTE_DIR/$REMOTE_REPORT_DIR/standard-$PROFILE-compare.md" "$MERGED_REPORT_LOCAL"
scp "$REMOTE_HOST:$REMOTE_DIR/workers/rust/benchmarks/reports/mechanical-core-$PROFILE-compare.md" "$LOCAL_OUTPUT_DIR/mechanical-core-$PROFILE-compare.md"
scp "$REMOTE_HOST:$REMOTE_DIR/workers/rust/benchmarks/reports/thermal-core-$PROFILE-compare.md" "$LOCAL_OUTPUT_DIR/thermal-core-$PROFILE-compare.md"
scp "$REMOTE_HOST:$REMOTE_DIR/workers/rust/benchmarks/reports/compound-core-$PROFILE-compare.md" "$LOCAL_OUTPUT_DIR/compound-core-$PROFILE-compare.md"

node "$ROOT_DIR/scripts/build-standard-benchmark-index.mjs" \
  --root "$ROOT_DIR/tmp/standard-benchmark" \
  --retain "$RETAIN_RUNS"
node "$ROOT_DIR/scripts/build-regression-lane-catalog.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-regression-gate-report.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-nightly-artifact-overview.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

echo "remote standard benchmark regression completed on $REMOTE_HOST"
echo "local output dir: $LOCAL_OUTPUT_DIR"
echo "merged report: $MERGED_REPORT_LOCAL"
