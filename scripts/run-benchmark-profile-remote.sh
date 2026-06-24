#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_HOST="${KYUUBIKI_LAB_HOST:-kyuubiki-lab}"
REMOTE_DIR="${KYUUBIKI_LAB_BENCH_DIR:-/tmp/kyuubiki-server-test}"
PROFILE="${PROFILE:-200k}"
MATRIX="${MATRIX:-thermal-core}"
REPEAT="${REPEAT:-3}"
RUSTUP_TOOLCHAIN_OVERRIDE="${RUSTUP_TOOLCHAIN_OVERRIDE:-stable}"
OUTPUT_SLUG="${OUTPUT_SLUG:-benchmark-profile-$(date -u +"%Y%m%dT%H%M%SZ")}"
REMOTE_OUTPUT_DIR="${REMOTE_OUTPUT_DIR:-/tmp/kyuubiki-benchmark-profile/$OUTPUT_SLUG}"
LOCAL_OUTPUT_DIR="${LOCAL_OUTPUT_DIR:-$ROOT_DIR/tmp/benchmark-profile/$OUTPUT_SLUG}"
case "$REMOTE_OUTPUT_DIR" in
  /*) REMOTE_JSON_PATH="$REMOTE_OUTPUT_DIR/$MATRIX-$PROFILE.json" ;;
  *) REMOTE_JSON_PATH="$REMOTE_DIR/$REMOTE_OUTPUT_DIR/$MATRIX-$PROFILE.json" ;;
esac
LOCAL_JSON_PATH="$LOCAL_OUTPUT_DIR/$MATRIX-$PROFILE.json"
LOCAL_MD_PATH="$LOCAL_OUTPUT_DIR/README.md"
SYNC_TO_REMOTE="${SYNC_TO_REMOTE:-1}"

usage() {
  cat <<'EOF'
Usage:
  PROFILE=200k MATRIX=thermal-core REPEAT=3 ./scripts/run-benchmark-profile-remote.sh

Runs one Rust benchmark profile/matrix on the shared lab machine without
requiring a checked baseline. Use this for new scale tiers such as 200k before
promoting them into the standard regression gate.

Environment:
  KYUUBIKI_LAB_HOST             SSH host alias. Default: kyuubiki-lab
  KYUUBIKI_LAB_BENCH_DIR        Remote workspace root. Default: /tmp/kyuubiki-server-test
  PROFILE                       Benchmark profile. Default: 200k
  MATRIX                        Benchmark matrix. Default: thermal-core
  REPEAT                        Benchmark repeat count. Default: 3
  RUSTUP_TOOLCHAIN_OVERRIDE     Remote toolchain override. Default: stable
  OUTPUT_SLUG                   Output folder slug under tmp/benchmark-profile/
  LOCAL_OUTPUT_DIR              Local output directory
  REMOTE_OUTPUT_DIR             Remote output directory. Default: /tmp/kyuubiki-benchmark-profile/<slug>
  SYNC_TO_REMOTE                Rsync benchmark source first. Default: 1
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

mkdir -p "$LOCAL_OUTPUT_DIR"

if [ "$SYNC_TO_REMOTE" = "1" ]; then
  rsync -az "$ROOT_DIR/workers/rust/crates/benchmark/src/" \
    "$REMOTE_HOST:$REMOTE_DIR/workers/rust/crates/benchmark/src/"
  rsync -az "$ROOT_DIR/workers/rust/benchmarks/" \
    "$REMOTE_HOST:$REMOTE_DIR/workers/rust/benchmarks/"
fi

ssh "$REMOTE_HOST" "set -euo pipefail; mkdir -p $(printf '%q' "$(dirname "$REMOTE_JSON_PATH")"); cd $(printf '%q' "$REMOTE_DIR")/workers/rust; RUSTUP_TOOLCHAIN=$(printf '%q' "$RUSTUP_TOOLCHAIN_OVERRIDE") cargo run --release -q -p kyuubiki-benchmark -- --profile $(printf '%q' "$PROFILE") --matrix $(printf '%q' "$MATRIX") --repeat $(printf '%q' "$REPEAT") --format json > $(printf '%q' "$REMOTE_JSON_PATH")"

scp "$REMOTE_HOST:$REMOTE_JSON_PATH" "$LOCAL_JSON_PATH"

node - "$LOCAL_JSON_PATH" "$LOCAL_MD_PATH" <<'NODE'
const fs = require("fs");
const [jsonPath, mdPath] = process.argv.slice(2);
const report = JSON.parse(fs.readFileSync(jsonPath, "utf8"));
const rows = report.cases.map((entry) => {
  const peakRssMiB = entry.peak_rss_kib == null ? "--" : (entry.peak_rss_kib / 1024).toFixed(1);
  return `| \`${entry.id}\` | ${entry.node_count} | ${entry.element_count} | ${entry.median_ms.toFixed(3)} | ${peakRssMiB} |`;
});
const markdown = [
  "# Benchmark profile smoke",
  "",
  `- Profile: \`${report.profile}\``,
  `- Matrix: \`${report.matrix}\``,
  `- Repeat: \`${report.repeat}\``,
  `- Case count: \`${report.cases.length}\``,
  "",
  "| Case | Nodes | Elements | Median ms | Peak RSS MiB |",
  "|---|---:|---:|---:|---:|",
  ...rows,
  "",
].join("\n");
fs.writeFileSync(mdPath, markdown);
NODE

echo "remote benchmark profile completed on $REMOTE_HOST"
echo "json: $LOCAL_JSON_PATH"
echo "summary: $LOCAL_MD_PATH"
