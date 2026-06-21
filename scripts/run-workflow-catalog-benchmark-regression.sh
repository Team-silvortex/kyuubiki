#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_HOST="${KYUUBIKI_LAB_HOST:-kyuubiki-lab}"
REMOTE_DIR="${KYUUBIKI_LAB_WORKFLOW_BENCH_DIR:-~/kyuubiki}"
OUTPUT_SLUG="${OUTPUT_SLUG:-workflow-catalog-$(date -u +"%Y%m%dT%H%M%SZ")}"
OUTPUT_PATH_REMOTE="${OUTPUT_PATH_REMOTE:-tmp/$OUTPUT_SLUG.json}"
REPEAT="${REPEAT:-3}"
CURRENT_SUMMARY_LOCAL="${CURRENT_SUMMARY_LOCAL:-$ROOT_DIR/tmp/workflow-catalog-benchmark/$OUTPUT_SLUG/summary.json}"
COMPARE_JSON_LOCAL="${COMPARE_JSON_LOCAL:-$ROOT_DIR/tmp/workflow-catalog-benchmark/$OUTPUT_SLUG/compare.json}"
COMPARE_MD_LOCAL="${COMPARE_MD_LOCAL:-$ROOT_DIR/tmp/workflow-catalog-benchmark/$OUTPUT_SLUG/compare.md}"
BASELINE_PATH="${BASELINE_PATH:-$ROOT_DIR/tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json}"
WORKFLOW_MEDIAN_THRESHOLD="${WORKFLOW_MEDIAN_THRESHOLD:-50}"
WORKFLOW_AVG_THRESHOLD="${WORKFLOW_AVG_THRESHOLD:-80}"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run-workflow-catalog-benchmark-regression.sh

Runs the remote workflow catalog benchmark on the shared lab machine, copies
the resulting summary back to the local workspace, and compares it against the
checked-in baseline.

Environment:
  KYUUBIKI_LAB_HOST                 SSH host alias. Default: kyuubiki-lab
  KYUUBIKI_LAB_WORKFLOW_BENCH_DIR   Remote workspace root. Default: ~/kyuubiki
  OUTPUT_SLUG                       Run folder name under tmp/workflow-catalog-benchmark/
  OUTPUT_PATH_REMOTE                Remote benchmark JSON output path under the workspace
  REPEAT                            Benchmark repeats. Default: 3
  CURRENT_SUMMARY_LOCAL             Local copy target for remote summary.json
  COMPARE_JSON_LOCAL                Local compare JSON output path
  COMPARE_MD_LOCAL                  Local compare Markdown output path
  BASELINE_PATH                     Checked-in baseline JSON
  WORKFLOW_MEDIAN_THRESHOLD         Allowed median regression percentage. Default: 50
  WORKFLOW_AVG_THRESHOLD            Allowed average regression percentage. Default: 80
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

mkdir -p "$(dirname "$CURRENT_SUMMARY_LOCAL")"

ssh "$REMOTE_HOST" \
  "export PATH=\$HOME/.local/elixir-1.15.7-otp-25/bin:\$PATH; \
   cd $REMOTE_DIR/apps/web && \
   ERL_LIBS=\"\$PWD/_build/test/lib\" \
   elixir ../../scripts/workflow-catalog-benchmark.exs \
     --repeat $(printf '%q' "$REPEAT") \
     --output ../../$(printf '%q' "$OUTPUT_PATH_REMOTE")"

scp "$REMOTE_HOST:$REMOTE_DIR/$OUTPUT_PATH_REMOTE" "$CURRENT_SUMMARY_LOCAL"

node "$ROOT_DIR/scripts/compare-workflow-catalog-benchmark.mjs" \
  --current "$CURRENT_SUMMARY_LOCAL" \
  --baseline "$BASELINE_PATH" \
  --json-out "$COMPARE_JSON_LOCAL" \
  --report-out "$COMPARE_MD_LOCAL" \
  --fail-on-median-regression-pct "$WORKFLOW_MEDIAN_THRESHOLD" \
  --fail-on-avg-regression-pct "$WORKFLOW_AVG_THRESHOLD"

node "$ROOT_DIR/scripts/build-regression-lane-catalog.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-regression-gate-report.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-nightly-artifact-overview.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

echo "remote summary copied to $CURRENT_SUMMARY_LOCAL"
echo "comparison json: $COMPARE_JSON_LOCAL"
echo "comparison report: $COMPARE_MD_LOCAL"
