#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REMOTE_HOST="${KYUUBIKI_LAB_HOST:-kyuubiki-lab}"
REMOTE_DIR="${KYUUBIKI_LAB_BENCH_DIR:-~/kyuubiki-bench-709b8c9}"
REMOTE_ARTIFACT_DIR="${KYUUBIKI_LAB_ARTIFACT_DIR:-$REMOTE_DIR}"
REMOTE_BENCHMARK_WRAPPER="${KYUUBIKI_LAB_BENCHMARK_WRAPPER:-/usr/local/bin/kyuubiki-direct-mesh-benchmark}"
OUTPUT_SLUG="${OUTPUT_SLUG:-nightly-$(date -u +"%Y%m%dT%H%M%SZ")}"
OUTPUT_DIR_REMOTE="${OUTPUT_DIR_REMOTE:-tmp/direct-mesh-benchmark-container/$OUTPUT_SLUG}"
REPEAT="${REPEAT:-3}"
DOCKER_RUN_NETWORK="${DOCKER_RUN_NETWORK:-host}"
HTTP_PROXY_VALUE="${HTTP_PROXY:-${http_proxy:-}}"
HTTPS_PROXY_VALUE="${HTTPS_PROXY:-${https_proxy:-}}"
NO_PROXY_VALUE="${NO_PROXY:-${no_proxy:-}}"
CURRENT_SUMMARY_LOCAL="${CURRENT_SUMMARY_LOCAL:-$ROOT_DIR/tmp/direct-mesh-benchmark-container/$OUTPUT_SLUG/summary.json}"
COMPARE_JSON_LOCAL="${COMPARE_JSON_LOCAL:-$ROOT_DIR/tmp/direct-mesh-benchmark-container/$OUTPUT_SLUG/compare.json}"
COMPARE_MD_LOCAL="${COMPARE_MD_LOCAL:-$ROOT_DIR/tmp/direct-mesh-benchmark-container/$OUTPUT_SLUG/compare.md}"
BASELINE_PATH="${BASELINE_PATH:-$ROOT_DIR/tests/integration/benchmarks/direct-mesh-docker-baseline.json}"
DIRECT_MESH_ELAPSED_THRESHOLD="${DIRECT_MESH_ELAPSED_THRESHOLD:-15}"
DIRECT_MESH_RSS_THRESHOLD="${DIRECT_MESH_RSS_THRESHOLD:-20}"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run-direct-mesh-benchmark-regression.sh

Runs the remote direct-mesh Docker benchmark on the shared lab machine, copies
the resulting summary back to the local workspace, and compares it against the
checked-in baseline.

Environment:
  KYUUBIKI_LAB_HOST                 SSH host alias. Default: kyuubiki-lab
  KYUUBIKI_LAB_BENCH_DIR            Remote benchmark workspace used to launch the wrapper
  KYUUBIKI_LAB_ARTIFACT_DIR         Remote workspace root used to read benchmark artifacts
  KYUUBIKI_LAB_BENCHMARK_WRAPPER    Remote root-owned benchmark wrapper
  OUTPUT_SLUG                       Run folder name under tmp/direct-mesh-benchmark-container/
  OUTPUT_DIR_REMOTE                 Remote benchmark output directory
  REPEAT                            Benchmark repeats. Default: 3
  DOCKER_RUN_NETWORK                Docker run network mode. Default: host
  HTTP_PROXY / HTTPS_PROXY / NO_PROXY
                                    Optional proxy variables forwarded to the remote run
  CURRENT_SUMMARY_LOCAL             Local copy target for remote summary.json
  COMPARE_JSON_LOCAL                Local compare JSON output path
  COMPARE_MD_LOCAL                  Local compare Markdown output path
  BASELINE_PATH                     Checked-in baseline JSON
  DIRECT_MESH_ELAPSED_THRESHOLD     Allowed elapsed regression percentage. Default: 15
  DIRECT_MESH_RSS_THRESHOLD         Allowed RSS regression percentage. Default: 20
EOF
}

if [ "${1:-}" = "--help" ]; then
  usage
  exit 0
fi

mkdir -p "$(dirname "$CURRENT_SUMMARY_LOCAL")"

if ! ssh "$REMOTE_HOST" "sudo -n $REMOTE_BENCHMARK_WRAPPER --help" >/dev/null 2>&1; then
  echo "passwordless sudo is not configured for $REMOTE_BENCHMARK_WRAPPER on $REMOTE_HOST" >&2
  echo "configure a narrow NOPASSWD sudoers rule for the benchmark wrapper before using this regression wrapper" >&2
  exit 1
fi

remote_env=(
  "DOCKER_RUN_NETWORK=$DOCKER_RUN_NETWORK"
)

if [ -n "$HTTP_PROXY_VALUE" ]; then
  remote_env+=("HTTP_PROXY=$HTTP_PROXY_VALUE" "http_proxy=$HTTP_PROXY_VALUE")
fi

if [ -n "$HTTPS_PROXY_VALUE" ]; then
  remote_env+=("HTTPS_PROXY=$HTTPS_PROXY_VALUE" "https_proxy=$HTTPS_PROXY_VALUE")
fi

if [ -n "$NO_PROXY_VALUE" ]; then
  remote_env+=("NO_PROXY=$NO_PROXY_VALUE" "no_proxy=$NO_PROXY_VALUE")
fi

preserve_env_names=()
for env_pair in "${remote_env[@]}"; do
  preserve_env_names+=("${env_pair%%=*}")
done

preserve_env_arg=""
if [ "${#preserve_env_names[@]}" -gt 0 ]; then
  preserve_env_arg="--preserve-env=$(IFS=,; printf '%s' "${preserve_env_names[*]}")"
fi

remote_exports=""
for env_pair in "${remote_env[@]}"; do
  remote_exports="$remote_exports export $(printf '%q' "$env_pair");"
done

ssh "$REMOTE_HOST" "cd $REMOTE_DIR &&$remote_exports sudo -n $preserve_env_arg $REMOTE_BENCHMARK_WRAPPER --skip-build --repeat $(printf '%q' "$REPEAT") --output-dir $(printf '%q' "$OUTPUT_DIR_REMOTE")"

REMOTE_SUMMARY_PATH="$(
  ssh "$REMOTE_HOST" "
    set -e
    for candidate in $(printf '%q ' \
      "$REMOTE_ARTIFACT_DIR/$OUTPUT_DIR_REMOTE/summary.json" \
      "$REMOTE_DIR/$OUTPUT_DIR_REMOTE/summary.json" \
      '$HOME/kyuubiki-bench-709b8c9/'"$OUTPUT_DIR_REMOTE"'/summary.json' \
      '$HOME/kyuubiki/'"$OUTPUT_DIR_REMOTE"'/summary.json'); do
      resolved_candidate=\$(eval printf '%s' \"\$candidate\")
      if [ -f \"\$resolved_candidate\" ]; then
        printf '%s\n' \"\$resolved_candidate\"
        exit 0
      fi
    done
    exit 1
  "
)" || {
  echo "failed to locate remote summary.json under $OUTPUT_DIR_REMOTE on $REMOTE_HOST" >&2
  exit 1
}

scp "$REMOTE_HOST:$REMOTE_SUMMARY_PATH" "$CURRENT_SUMMARY_LOCAL"

node "$ROOT_DIR/scripts/compare-direct-mesh-benchmark.mjs" \
  --current "$CURRENT_SUMMARY_LOCAL" \
  --baseline "$BASELINE_PATH" \
  --json-out "$COMPARE_JSON_LOCAL" \
  --report-out "$COMPARE_MD_LOCAL" \
  --fail-on-elapsed-regression-pct "$DIRECT_MESH_ELAPSED_THRESHOLD" \
  --fail-on-rss-regression-pct "$DIRECT_MESH_RSS_THRESHOLD"

node "$ROOT_DIR/scripts/build-regression-lane-catalog.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-regression-gate-report.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-nightly-artifact-overview.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

echo "remote summary copied to $CURRENT_SUMMARY_LOCAL"
echo "remote summary source: $REMOTE_SUMMARY_PATH"
echo "comparison json: $COMPARE_JSON_LOCAL"
echo "comparison report: $COMPARE_MD_LOCAL"
