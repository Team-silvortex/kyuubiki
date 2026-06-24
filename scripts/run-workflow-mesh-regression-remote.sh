#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TOOLCHAIN_ENV_JSON="${TOOLCHAIN_ENV_JSON:-$(node "$ROOT_DIR/scripts/toolchain-env.mjs" --json)}"

toolchain_value() {
  local key="$1"
  node -e 'const key = process.argv[1]; let data = ""; process.stdin.on("data", (chunk) => data += chunk); process.stdin.on("end", () => console.log(JSON.parse(data)[key] ?? ""));' "$key" <<<"$TOOLCHAIN_ENV_JSON"
}

REMOTE_HOST="${KYUUBIKI_LAB_HOST:-kyuubiki-lab}"
REMOTE_DIR="${KYUUBIKI_LAB_WORKFLOW_MESH_DIR:-~/kyuubiki}"
OUTPUT_SLUG="${OUTPUT_SLUG:-workflow-mesh-$(date -u +"%Y%m%dT%H%M%SZ")}"
LOCAL_OUTPUT_DIR="${LOCAL_OUTPUT_DIR:-$ROOT_DIR/tmp/workflow-mesh-regression/$OUTPUT_SLUG}"
REMOTE_OUTPUT_DIR="${REMOTE_OUTPUT_DIR:-tmp/workflow-mesh-regression/$OUTPUT_SLUG}"
LOCAL_LOG_PATH="${LOCAL_LOG_PATH:-$LOCAL_OUTPUT_DIR/run.log}"
REMOTE_LOG_PATH="${REMOTE_LOG_PATH:-$REMOTE_DIR/$REMOTE_OUTPUT_DIR/run.log}"
REMOTE_OTP_VERSION="${REMOTE_OTP_VERSION:-$(toolchain_value KYUUBIKI_REMOTE_OTP_VERSION)}"
REMOTE_ELIXIR_VERSION="${REMOTE_ELIXIR_VERSION:-$(toolchain_value KYUUBIKI_REMOTE_ELIXIR_VERSION)}"
REMOTE_PG_BIN_DIR="${REMOTE_PG_BIN_DIR:-/usr/lib/postgresql/16/bin}"
REMOTE_PG_PORT="${REMOTE_PG_PORT:-55432}"
REMOTE_PG_USER="${REMOTE_PG_USER:-kyuubiki}"
REMOTE_PG_DB="${REMOTE_PG_DB:-kyuubiki_mesh_test}"
SYNC_TO_REMOTE="${SYNC_TO_REMOTE:-1}"

usage() {
  cat <<'EOF'
Usage:
  ./scripts/run-workflow-mesh-regression-remote.sh

Sync the workflow mesh regression wrappers and integration tests to the shared
lab machine, run the distributed workflow mesh regression trio there in
sequence, and copy the resulting log back into the local workspace.

Environment:
  KYUUBIKI_LAB_HOST                SSH host alias. Default: kyuubiki-lab
  KYUUBIKI_LAB_WORKFLOW_MESH_DIR   Remote workspace root. Default: ~/kyuubiki
  OUTPUT_SLUG                      Output folder slug under tmp/workflow-mesh-regression/
  LOCAL_OUTPUT_DIR                 Local output directory for pulled artifacts
  REMOTE_OUTPUT_DIR                Remote output directory under the workspace
  LOCAL_LOG_PATH                   Local copied run log path
  REMOTE_LOG_PATH                  Remote run log path
  REMOTE_OTP_VERSION               Remote OTP version directory. Default: 28.4
  REMOTE_ELIXIR_VERSION            Remote Elixir version directory. Default: 1.20.1-otp-28
  REMOTE_PG_BIN_DIR                Remote PostgreSQL bin directory. Default: /usr/lib/postgresql/16/bin
  REMOTE_PG_PORT                   Remote temporary PostgreSQL port. Default: 55432
  REMOTE_PG_USER                   Remote temporary PostgreSQL superuser. Default: kyuubiki
  REMOTE_PG_DB                     Remote temporary PostgreSQL database. Default: kyuubiki_mesh_test
  SYNC_TO_REMOTE                   Rsync runtime/web/rust/test inputs first. Default: 1
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
    "$ROOT_DIR/scripts/build-workflow-mesh-regression-index.mjs" \
    "$ROOT_DIR/scripts/build-workflow-mesh-regression-summary.mjs" \
    "$ROOT_DIR/scripts/build-nightly-artifact-overview.mjs" \
    "$ROOT_DIR/scripts/kyuubiki-runtime.mjs" \
    "$ROOT_DIR/scripts/run-workflow-mesh-regression.sh" \
    "$ROOT_DIR/scripts/run-workflow-mesh-regression-remote.sh" \
    "$REMOTE_HOST:$REMOTE_DIR/scripts/"
  rsync -az \
    "$ROOT_DIR/apps/frontend/public/models/" \
    "$REMOTE_HOST:$REMOTE_DIR/apps/frontend/public/models/"
  rsync -az \
    --exclude '_build/' \
    --exclude 'deps/' \
    "$ROOT_DIR/apps/web/" \
    "$REMOTE_HOST:$REMOTE_DIR/apps/web/"
  rsync -az \
    --exclude 'target/' \
    "$ROOT_DIR/workers/rust/" \
    "$REMOTE_HOST:$REMOTE_DIR/workers/rust/"
fi

rsync -az \
  "$ROOT_DIR/tests/integration/workflow-distributed-smoke.test.mjs" \
  "$ROOT_DIR/tests/integration/workflow-offline-mesh-smoke.test.mjs" \
  "$ROOT_DIR/tests/integration/workflow-offline-mesh-branch-diagnostics-smoke.test.mjs" \
  "$REMOTE_HOST:$REMOTE_DIR/tests/integration/"

ssh "$REMOTE_HOST" "set -euo pipefail; \
  remote_elixir_installs_dir=\${REMOTE_ELIXIR_INSTALLS_DIR:-\$HOME/.elixir-install/installs}; \
  remote_workspace_root=$(printf '%q' "$REMOTE_DIR"); \
  case \"\$remote_workspace_root\" in \
    ~/*) remote_workspace_root=\"\$HOME/\${remote_workspace_root#~/}\" ;; \
  esac; \
  export PATH=\"\$remote_elixir_installs_dir/otp/$(printf '%q' "$REMOTE_OTP_VERSION")/bin:\$PATH\"; \
  export PATH=\"\$remote_elixir_installs_dir/elixir/$(printf '%q' "$REMOTE_ELIXIR_VERSION")/bin:\$PATH\"; \
  remote_pg_root=\"\$remote_workspace_root/$(printf '%q' "$REMOTE_OUTPUT_DIR")/postgres\"; \
  remote_pg_data=\"\$remote_pg_root/data\"; \
  remote_pg_socket=\"/tmp\"; \
  mkdir -p \"\$remote_workspace_root/$(printf '%q' "$REMOTE_OUTPUT_DIR")\" \"\$remote_pg_root\"; \
  if [ ! -f \"\$remote_pg_data/PG_VERSION\" ]; then \
    \"$(printf '%q' "$REMOTE_PG_BIN_DIR")/initdb\" -D \"\$remote_pg_data\" -U $(printf '%q' "$REMOTE_PG_USER") --auth-local=trust --auth-host=trust >/dev/null; \
  fi; \
  \"$(printf '%q' "$REMOTE_PG_BIN_DIR")/pg_ctl\" -D \"\$remote_pg_data\" -o \"-F -p $(printf '%q' "$REMOTE_PG_PORT") -k \$remote_pg_socket -h 127.0.0.1\" -l \"\$remote_pg_root/postgres.log\" start >/dev/null; \
  trap '\"$(printf '%q' "$REMOTE_PG_BIN_DIR")/pg_ctl\" -D \"\$remote_pg_data\" stop -m fast >/dev/null 2>&1 || true' EXIT; \
  \"$(printf '%q' "$REMOTE_PG_BIN_DIR")/createdb\" -h 127.0.0.1 -p $(printf '%q' "$REMOTE_PG_PORT") -U $(printf '%q' "$REMOTE_PG_USER") $(printf '%q' "$REMOTE_PG_DB") >/dev/null 2>&1 || true; \
  export DATABASE_URL=\"ecto://$(printf '%q' "$REMOTE_PG_USER")@127.0.0.1:$(printf '%q' "$REMOTE_PG_PORT")/$(printf '%q' "$REMOTE_PG_DB")\"; \
  export OUTPUT_SLUG=$(printf '%q' "$OUTPUT_SLUG"); \
  export OUTPUT_DIR=\"\$remote_workspace_root/$(printf '%q' "$REMOTE_OUTPUT_DIR")\"; \
  export LOG_PATH=$(printf '%q' "$REMOTE_LOG_PATH"); \
  cd \"\$remote_workspace_root\" && make test-integration-workflow-mesh 2>&1 | tee $(printf '%q' "$REMOTE_LOG_PATH")"

scp "$REMOTE_HOST:$REMOTE_LOG_PATH" "$LOCAL_LOG_PATH"
scp "$REMOTE_HOST:$REMOTE_DIR/$REMOTE_OUTPUT_DIR/summary.json" "$LOCAL_OUTPUT_DIR/summary.json"
scp "$REMOTE_HOST:$REMOTE_DIR/$REMOTE_OUTPUT_DIR/README.md" "$LOCAL_OUTPUT_DIR/README.md"

node "$ROOT_DIR/scripts/build-workflow-mesh-regression-index.mjs" \
  --root "$ROOT_DIR/tmp/workflow-mesh-regression"

node "$ROOT_DIR/scripts/build-regression-lane-catalog.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-regression-gate-report.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

node "$ROOT_DIR/scripts/build-nightly-artifact-overview.mjs" \
  --tmp-root "$ROOT_DIR/tmp"

echo "remote workflow mesh regression completed on $REMOTE_HOST"
echo "local output dir: $LOCAL_OUTPUT_DIR"
echo "local log: $LOCAL_LOG_PATH"
