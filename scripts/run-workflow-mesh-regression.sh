#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
NODE_BIN="${NODE_BIN:-node}"
OUTPUT_SLUG="${OUTPUT_SLUG:-workflow-mesh-$(date -u +"%Y%m%dT%H%M%SZ")}"
OUTPUT_DIR="${OUTPUT_DIR:-$ROOT_DIR/tmp/workflow-mesh-regression/$OUTPUT_SLUG}"
LOG_PATH="${LOG_PATH:-$OUTPUT_DIR/run.log}"

TEST_FILES=(
  "tests/integration/workflow-distributed-smoke.test.mjs"
  "tests/integration/workflow-offline-mesh-smoke.test.mjs"
  "tests/integration/workflow-offline-mesh-branch-diagnostics-smoke.test.mjs"
)

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run-workflow-mesh-regression.sh

Runs the current distributed workflow mesh regression trio in strict sequence so
the shared local orchestrator port does not collide across tests.

Environment:
  NODE_BIN      Node.js executable to use. Default: node
  OUTPUT_SLUG   Output folder slug under tmp/workflow-mesh-regression/
  OUTPUT_DIR    Output directory for run.log plus summary artifacts
  LOG_PATH      Log path for the combined regression run
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

cd "$ROOT_DIR"
mkdir -p "$OUTPUT_DIR"

{
  for test_file in "${TEST_FILES[@]}"; do
    echo "==> running $test_file"
    "$NODE_BIN" --test "$test_file"
  done

  echo "workflow mesh regression completed"
} 2>&1 | tee "$LOG_PATH"

"$NODE_BIN" "$ROOT_DIR/scripts/build-workflow-mesh-regression-summary.mjs" \
  --log "$LOG_PATH" \
  --output-dir "$OUTPUT_DIR"

"$NODE_BIN" "$ROOT_DIR/scripts/build-workflow-mesh-regression-index.mjs" \
  --root "$ROOT_DIR/tmp/workflow-mesh-regression"

echo "workflow mesh regression log: $LOG_PATH"
