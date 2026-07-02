#!/usr/bin/env zsh

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
WEB_DIR="$ROOT_DIR/apps/web"
FRONTEND_DIR="$ROOT_DIR/apps/frontend"
HUB_GUI_DIR="$ROOT_DIR/apps/hub-gui"
INSTALLER_GUI_DIR="$ROOT_DIR/apps/installer-gui"
WORKBENCH_GUI_DIR="$ROOT_DIR/apps/workbench-gui"
RUST_DIR="$ROOT_DIR/workers/rust"
RUN_DIR="$ROOT_DIR/tmp/run"
ENV_FILE="$ROOT_DIR/.env.local"
ORCHESTRATOR_LOG="$RUN_DIR/orchestrator.log"
FRONTEND_LOG="$RUN_DIR/frontend.log"
AGENT_LOG="$RUN_DIR/agent.log"
HOT_RUN_DIR="$RUN_DIR/hot"
ORCHESTRATOR_SCREEN="kyuubiki_orchestrator"
FRONTEND_SCREEN="kyuubiki_frontend"
AGENT_SCREEN_PREFIX="kyuubiki_agent"
HOT_STACK_SCREEN="kyuubiki_hot_stack"

load_env_file() {
  if [[ -f "$ENV_FILE" ]]; then
    set -a
    source "$ENV_FILE"
    set +a
  fi
}

load_env_file

agent_endpoints_value() {
  echo "${KYUUBIKI_AGENT_ENDPOINTS:-127.0.0.1:5001,127.0.0.1:5002}"
}


source "$ROOT_DIR/scripts/kyuubiki-legacy-usage.zsh"
source "$ROOT_DIR/scripts/kyuubiki-legacy-runners.zsh"
source "$ROOT_DIR/scripts/kyuubiki-legacy-desktop-gui.zsh"
source "$ROOT_DIR/scripts/kyuubiki-legacy-desktop-artifacts.zsh"
source "$ROOT_DIR/scripts/kyuubiki-legacy-desktop-verify.zsh"
source "$ROOT_DIR/scripts/kyuubiki-legacy-services.zsh"

host_platform() {
  case "$(uname -s)" in
    Darwin) echo "macos" ;;
    Linux) echo "linux" ;;
    MINGW*|MSYS*|CYGWIN*) echo "windows" ;;
    *) echo "linux" ;;
  esac
}

sqlite_database_path_value() {
  echo "${SQLITE_DATABASE_PATH:-$ROOT_DIR/tmp/data/kyuubiki_dev.sqlite3}"
}

ensure_cloud_env() {
  if [[ -z "${DATABASE_URL:-}" ]]; then
    echo "DATABASE_URL is required for cloud/postgres mode" >&2
    exit 1
  fi
}

storage_mode_label() {
  local mode="${1:-default}"

  case "$mode" in
    local) echo "sqlite" ;;
    cloud|distributed) echo "postgres" ;;
    *) echo "${KYUUBIKI_STORAGE_BACKEND:-sqlite}" ;;
  esac
}

deployment_mode_label() {
  local mode="${1:-default}"

  case "$mode" in
    local) echo "local" ;;
    cloud) echo "cloud" ;;
    distributed) echo "distributed" ;;
    *) echo "${KYUUBIKI_DEPLOYMENT_MODE:-local}" ;;
  esac
}

storage_mode_exports() {
  local mode="${1:-default}"

  case "$mode" in
    local)
      printf 'KYUUBIKI_STORAGE_BACKEND="sqlite" SQLITE_DATABASE_PATH="%s"' "$(sqlite_database_path_value)"
      ;;
    cloud)
      ensure_cloud_env
      printf 'KYUUBIKI_STORAGE_BACKEND="postgres" DATABASE_URL="%s"' "$DATABASE_URL"
      ;;
    distributed)
      ensure_cloud_env
      printf 'KYUUBIKI_STORAGE_BACKEND="postgres" DATABASE_URL="%s"' "$DATABASE_URL"
      ;;
    *)
      printf 'KYUUBIKI_STORAGE_BACKEND="%s"' "${KYUUBIKI_STORAGE_BACKEND:-sqlite}"
      ;;
  esac
}

deployment_mode_exports() {
  local mode="${1:-default}"
  printf 'KYUUBIKI_DEPLOYMENT_MODE="%s"' "$(deployment_mode_label "$mode")"
}

require_dir() {
  local dir="$1"
  if [[ ! -d "$dir" ]]; then
    echo "missing directory: $dir" >&2
    exit 1
  fi
}

ensure_run_dir() {
  mkdir -p "$RUN_DIR"
}

ensure_hot_run_dir() {
  mkdir -p "$HOT_RUN_DIR"
}

require_screen() {
  if ! command -v screen >/dev/null 2>&1; then
    echo "screen is required for background start/restart" >&2
    exit 1
  fi
}

port_in_use() {
  local port="$1"
  lsof -nP -iTCP:"$port" -sTCP:LISTEN >/dev/null 2>&1
}

pid_on_port() {
  local port="$1"
  lsof -tiTCP:"$port" -sTCP:LISTEN 2>/dev/null | head -n 1 || true
}

wait_for_port_state() {
  local port="$1"
  local expected="${2:-free}"
  local timeout_seconds="${3:-10}"
  local started_at
  started_at="$(date +%s)"

  while true; do
    if [[ "$expected" == "free" ]]; then
      if ! port_in_use "$port"; then
        return 0
      fi
    else
      if port_in_use "$port"; then
        return 0
      fi
    fi

    if (( $(date +%s) - started_at >= timeout_seconds )); then
      echo "timed out waiting for port $port to become $expected" >&2
      return 1
    fi

    sleep 0.2
  done
}

stop_port() {
  local port="$1"
  local pid
  pid="$(pid_on_port "$port")"

  if [[ -n "${pid:-}" ]]; then
    kill "$pid"
    wait_for_port_state "$port" free 10 || kill -9 "$pid" >/dev/null 2>&1 || true
    wait_for_port_state "$port" free 5 || true
    echo "stopped process $pid on port $port"
  else
    echo "no listening process on port $port"
  fi
}

show_status() {
  local orchestrator_pid frontend_pid
  orchestrator_pid="$(pid_on_port 4000)"
  frontend_pid="$(pid_on_port 3000)"
  local agent_ports agent_port agent_pid agent_port_spec
  agent_ports=("${(@s:,:)$(agent_endpoints_value)}")

  if [[ -n "${orchestrator_pid:-}" ]]; then
    echo "orchestrator: running on http://127.0.0.1:4000 (pid $orchestrator_pid)"
  else
    echo "orchestrator: stopped"
  fi

  if [[ -n "${frontend_pid:-}" ]]; then
    echo "frontend: running on http://127.0.0.1:3000 (pid $frontend_pid)"
  else
    echo "frontend: stopped"
  fi

  for agent_port_spec in "${agent_ports[@]}"; do
    agent_port="${agent_port_spec##*:}"
    agent_pid="$(pid_on_port "$agent_port")"
    if [[ -n "${agent_pid:-}" ]]; then
      echo "agent[$agent_port]: running on tcp://127.0.0.1:${agent_port} (pid $agent_pid)"
    else
      echo "agent[$agent_port]: stopped"
    fi
  done
}





command="${1:-help}"
if [[ $# -gt 0 ]]; then
  shift
fi

case "$command" in
  help)
    usage
    ;;
  project)
    run_frontend_cli project "$@"
    ;;
  macro)
    run_frontend_cli macro "$@"
    ;;
  build-frontend)
    run_build_frontend
    ;;
  build-orchestrator)
    run_build_orchestrator
    ;;
  build-agent)
    run_build_agent
    ;;
  build-hub-gui)
    run_hub_gui_build "${1:-$(host_platform)}"
    ;;
  build-installer-gui)
    run_installer_gui_build "${1:-$(host_platform)}"
    ;;
  build-workbench-gui)
    run_workbench_gui_build "${1:-$(host_platform)}"
    ;;
  start)
    start_services
    ;;
  start-local)
    start_services local
    ;;
  start-cloud)
    start_services cloud
    ;;
  start-distributed)
    start_services distributed
    ;;
  status)
    show_status
    ;;
  stop)
    stop_services
    ;;
  restart)
    restart_services
    ;;
  restart-local)
    restart_services local
    ;;
  restart-cloud)
    restart_services cloud
    ;;
  restart-distributed)
    restart_services distributed
    ;;
  export-db)
    run_export_db "$@"
    ;;
  install)
    run_installer "$@"
    ;;
  doctor)
    run_installer doctor "$@"
    ;;
  validate-env)
    run_installer validate-env "$@"
    ;;
  package)
    run_package_runtime "$@"
    ;;
  package-runtime)
    run_package_runtime "$@"
    ;;
  release-snapshot)
    run_release_snapshot "$@"
    ;;
  package-desktop)
    run_package_desktop "${1:-$(host_platform)}"
    ;;
  desktop-status)
    run_desktop_status "${1:-$(host_platform)}"
    ;;
  desktop-stage)
    run_desktop_stage "${1:-$(host_platform)}"
    ;;
  desktop-build-host)
    run_desktop_build_host
    ;;
  desktop-release)
    run_desktop_release "${1:-$(host_platform)}"
    ;;
  desktop-verify)
    run_desktop_verify "${1:-$(host_platform)}"
    ;;
  hub-gui-dev)
    run_hub_gui_dev
    ;;
  hub-gui-build)
    run_hub_gui_build "${1:-$(host_platform)}"
    ;;
  hot-local)
    run_hot_stack local
    ;;
  hot-cloud)
    run_hot_stack cloud
    ;;
  hot-distributed)
    run_hot_stack distributed
    ;;
  hot-web)
    run_hot_web "${1:-local}"
    ;;
  hot-agent)
    run_hot_agent "${1:-5001}"
    ;;
  hot-hub-gui)
    run_hot_hub_gui
    ;;
  hot-installer-gui)
    run_hot_installer_gui
    ;;
  hot-workbench-gui)
    run_hot_workbench_gui
    ;;
  hot-start-local)
    start_hot_stack_background local
    ;;
  hot-start-cloud)
    start_hot_stack_background cloud
    ;;
  hot-start-distributed)
    start_hot_stack_background distributed
    ;;
  hot-stop)
    stop_hot_stack_background
    ;;
  hot-status)
    show_hot_status
    ;;
  installer-gui-dev)
    run_installer_gui_dev
    ;;
  installer-gui-build)
    run_installer_gui_build "${1:-$(host_platform)}"
    ;;
  workbench-gui-dev)
    run_workbench_gui_dev
    ;;
  workbench-gui-build)
    run_workbench_gui_build "${1:-$(host_platform)}"
    ;;
  test)
    make -C "$ROOT_DIR" test
    ;;
  verify)
    make -C "$ROOT_DIR" verify
    ;;
  web-test)
    (
      cd "$WEB_DIR"
      mix test "$@"
    )
    ;;
  rust-test)
    (
      cd "$RUST_DIR"
      cargo test "$@"
    )
    node "$ROOT_DIR/scripts/audit-rust-line-counts.mjs" --max "${MAX_LINES:-600}"
    ;;
  rust-line-audit)
    node "$ROOT_DIR/scripts/audit-rust-line-counts.mjs" --max "${MAX_LINES:-600}"
    ;;
  frontend-test)
    (
      cd "$FRONTEND_DIR"
      npm run typecheck
      npm run build
    )
    ;;
  integration-ui-mechanical)
    node --test "$ROOT_DIR/tests/integration/workbench-ui-mechanical-smoke.test.mjs"
    ;;
  integration-ui-thermal)
    node --test "$ROOT_DIR/tests/integration/workbench-ui-thermal-smoke.test.mjs"
    ;;
  sdk-smoke)
    run_sdk_smoke
    ;;
  playground-test)
    run_playground_tests
    ;;
  orchestrator)
    run_orchestrator "${1:-4000}"
    ;;
  orchestrator-local)
    run_orchestrator "${1:-4000}" local
    ;;
  orchestrator-cloud)
    run_orchestrator "${1:-4000}" cloud
    ;;
  orchestrator-distributed)
    run_orchestrator "${1:-4000}" distributed
    ;;
  agent)
    run_agent "${1:-5001}"
    ;;
  format)
    make -C "$ROOT_DIR" format
    ;;
  worker)
    run_worker "$@"
    ;;
  smoke)
    run_smoke
    ;;
  playground)
    run_orchestrator "${1:-4000}"
    ;;
  frontend)
    run_frontend "${1:-3000}"
    ;;
  benchmark)
    run_benchmark "$@"
    ;;
  *)
    echo "unknown command: $command" >&2
    echo >&2
    usage >&2
    exit 1
    ;;
esac
