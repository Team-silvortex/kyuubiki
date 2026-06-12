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

usage() {
  cat <<'EOF'
Unified entrypoint for local kyuubiki development.

Usage:
  ./scripts/kyuubiki <command> [args...]

Commands:
  help              Show this help
  project           Run the integrated frontend project-bundle CLI
  macro             Run the integrated frontend macro CLI
  build-frontend    Build the browser workbench production bundle
  build-orchestrator Compile the Elixir control plane in production mode
  build-agent       Build the Rust solver agent binary in release mode
  build-hub-gui [platform] Build the Tauri Hub GUI bundles
  build-installer-gui [platform] Build the Tauri installer GUI bundles
  build-workbench-gui [platform] Build the Tauri desktop workbench shell bundles
  start             Start the orchestrator API, frontend, and solver agent in the background
  start-local       Start services with SQLite forced for local development
  start-cloud       Start services with PostgreSQL forced for cloud/distributed use
  start-distributed Start the control plane only for distributed deployments
  status            Show whether local services are running
  stop              Stop the orchestrator API, frontend, and solver agent services
  restart           Restart the orchestrator API, frontend, and solver agent services
  restart-local     Restart services with SQLite forced for local development
  restart-cloud     Restart services with PostgreSQL forced for cloud/distributed use
  restart-distributed Restart the control plane only for distributed deployments
  export-db         Export the orchestrator database snapshot as JSON
  install           Run the cross-platform installer/bootstrap utility
  doctor            Run the cross-platform environment doctor
  validate-env      Validate required environment variables from .env.local
  package           Stage a portable release directory under dist/
  package-runtime   Stage the portable runtime release scaffold under dist/
  package-desktop [platform|all] Build/stage all Tauri desktop shells
  release-snapshot <version> [args...] Scaffold or update a lightweight release snapshot manifest
  desktop-status [platform|all] Show host/platform desktop packaging readiness and next steps
  desktop-stage [platform|all] Stage the desktop/runtime release scaffold under dist/
  desktop-build-host Build the Hub, Installer, and Workbench desktop bundles for this host
  desktop-release [platform|all] Stage desktop release output and build host-native desktop bundles
  desktop-verify [platform|all] Verify staged manifests and platform icon inputs for desktop release
  hub-gui-dev       Run the Tauri Hub GUI in development mode
  hub-gui-build     Build the Tauri Hub GUI bundles
  installer-gui-dev Run the Tauri installer GUI in development mode
  installer-gui-build Build the Tauri installer GUI bundles
  workbench-gui-dev Run the Tauri desktop workbench shell in development mode
  workbench-gui-build Build the Tauri desktop workbench shell bundles
  hot-local         Run the local full-stack dev loop with hot reload/watch
  hot-cloud         Run the cloud/postgres full-stack dev loop with hot reload/watch
  hot-distributed   Run the distributed control-plane dev loop with hot reload/watch
  hot-web           Run the Elixir control plane with restart-on-change
  hot-agent         Run the Rust solver agent with restart-on-change
  hot-hub-gui       Run the Hub Tauri shell in dev/HMR mode
  hot-installer-gui Run the installer Tauri shell in dev/HMR mode
  hot-workbench-gui Run the workbench Tauri shell in dev/HMR mode
  hot-start-local   Start the local full-stack hot-reload loop in the background
  hot-start-cloud   Start the cloud/postgres hot-reload loop in the background
  hot-start-distributed Start the distributed control-plane hot-reload loop in the background
  hot-stop          Stop the managed hot-reload background loop
  hot-status        Show the managed hot-reload background status
  test              Run all tests
  verify            Run formatting checks and tests
  web-test          Run Elixir tests
  rust-test         Run Rust tests
  frontend-test     Run frontend typecheck and production build
  integration-ui-mechanical Run the Playwright Workbench UI smoke for representative mechanical samples
  integration-ui-thermal Run the Playwright Workbench UI smoke for representative thermal and thermo-mechanical samples
  sdk-smoke         Run the Python / Elixir / Rust headless SDK smoke suite
  playground-test   Run browser FEM tests
  agent             Run the Rust FEM TCP agent
  frontend          Run the Next.js workbench UI
  benchmark         Run the Rust solver benchmark suite
  format            Format Elixir and Rust code
  worker [args...]  Run the Rust mock worker CLI
  smoke             Run the Elixir -> Rust integration smoke test
  sdk-smoke         Run the Python / Elixir / Rust SDK smoke suite
  playground        Run the API-only orchestrator locally (legacy alias)
  orchestrator      Run the API-only orchestrator locally

Examples:
  ./scripts/kyuubiki smoke
  ./scripts/kyuubiki project inspect demo.kyuubiki
  ./scripts/kyuubiki project validate demo.kyuubiki --json
  ./scripts/kyuubiki project normalize demo.kyuubiki --out demo.normalized.kyuubiki
  ./scripts/kyuubiki project unpack demo.kyuubiki --out ./tmp/demo-project
  ./scripts/kyuubiki project pack ./tmp/demo-project --out demo.repacked.kyuubiki
  ./scripts/kyuubiki project diff before.kyuubiki after.kyuubiki
  ./scripts/kyuubiki macro inspect review-result.json
  ./scripts/kyuubiki macro validate review-result.json --json
  ./scripts/kyuubiki agent -- --port 5001
  ./scripts/kyuubiki worker -- --job-id demo --project-id p1 --case-id c1 --steps 3
  ./scripts/kyuubiki start
  ./scripts/kyuubiki build-frontend
  ./scripts/kyuubiki build-orchestrator
  ./scripts/kyuubiki build-agent
  ./scripts/kyuubiki build-hub-gui
  ./scripts/kyuubiki build-hub-gui linux
  ./scripts/kyuubiki package-runtime
  ./scripts/kyuubiki package-desktop
  ./scripts/kyuubiki package-desktop linux
  ./scripts/kyuubiki package-desktop all
  ./scripts/kyuubiki release-snapshot 1.6.1 --status staged --dry-run
  ./scripts/kyuubiki desktop-status
  ./scripts/kyuubiki desktop-status all
  ./scripts/kyuubiki desktop-stage all
  ./scripts/kyuubiki desktop-build-host
  ./scripts/kyuubiki desktop-release
  ./scripts/kyuubiki desktop-verify all
  ./scripts/kyuubiki hot-local
  ./scripts/kyuubiki hot-cloud
  ./scripts/kyuubiki hot-distributed
  ./scripts/kyuubiki hot-web
  ./scripts/kyuubiki hot-agent 5001
  ./scripts/kyuubiki hot-hub-gui
  ./scripts/kyuubiki hot-start-local
  ./scripts/kyuubiki hot-status
  ./scripts/kyuubiki start-local
  ./scripts/kyuubiki start-cloud
  ./scripts/kyuubiki start-distributed
  KYUUBIKI_STORAGE_BACKEND=postgres DATABASE_URL=ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev ./scripts/kyuubiki start
  cp .env.example .env.local && ./scripts/kyuubiki restart
  ./scripts/kyuubiki orchestrator
  ./scripts/kyuubiki frontend
  ./scripts/kyuubiki benchmark -- --repeat 20 --format json
  ./scripts/kyuubiki restart
  ./scripts/kyuubiki export-db > kyuubiki-database.json
  ./scripts/kyuubiki doctor
  ./scripts/kyuubiki validate-env
  ./scripts/kyuubiki install bootstrap
  ./scripts/kyuubiki package
  ./scripts/kyuubiki hub-gui-dev
  ./scripts/kyuubiki hub-gui-build
  ./scripts/kyuubiki installer-gui-dev
  ./scripts/kyuubiki installer-gui-build
  ./scripts/kyuubiki workbench-gui-dev
  ./scripts/kyuubiki workbench-gui-build
EOF
}

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

run_worker() {
  require_dir "$RUST_DIR"
  (
    cd "$RUST_DIR"
    cargo run -p kyuubiki-cli -- "$@"
  )
}

run_smoke() {
  require_dir "$WEB_DIR"
  (
    cd "$WEB_DIR"
    mix run -e "
      alias KyuubikiWeb.Jobs.Store
      alias KyuubikiWeb.Workers.MockWorkerAdapter

      {:ok, job} =
        Store.create(%{
          job_id: \"job-smoke\",
          project_id: \"project-smoke\",
          simulation_case_id: \"case-smoke\"
        })

      IO.puts(\"running smoke flow for #{job.job_id}\")
      IO.inspect(MockWorkerAdapter.run_job(job), label: \"worker_result\")
      IO.inspect(Store.get(job.job_id), label: \"final_job\")
    "
  )
}

run_playground_tests() {
  (
    cd "$ROOT_DIR"
    node --test apps/web/playground/test/fem.test.mjs
  )
}

run_sdk_smoke() {
  require_dir "$ROOT_DIR/sdks/python"
  require_dir "$ROOT_DIR/sdks/elixir"
  require_dir "$ROOT_DIR/sdks/rust"

  (
    cd "$ROOT_DIR"
    PYTHONPATH=sdks/python python3 -m unittest discover -s sdks/python/tests
  )

  (
    cd "$ROOT_DIR/sdks/elixir"
    mix test
  )

  (
    cd "$ROOT_DIR"
    cargo test --manifest-path sdks/rust/Cargo.toml --test smoke
  )
}

run_release_snapshot() {
  (
    cd "$ROOT_DIR"
    node ./scripts/create-release-snapshot.mjs "$@"
  )
}

run_orchestrator() {
  local port="${1:-4000}"
  local mode="${2:-default}"

  if port_in_use "$port"; then
    echo "orchestrator already running at http://127.0.0.1:${port}"
    return 0
  fi

  (
    cd "$WEB_DIR"
    echo "serving orchestrator API at http://127.0.0.1:${port} ($(storage_mode_label "$mode"), $(deployment_mode_label "$mode"))"
    eval "$(storage_mode_exports "$mode") $(deployment_mode_exports "$mode")" PORT="$port" mix run --no-halt
  )
}

run_agent() {
  local port="${1:-5001}"

  if port_in_use "$port"; then
    echo "Rust FEM agent already running at tcp://127.0.0.1:${port}"
    return 0
  fi

  (
    cd "$RUST_DIR"
    echo "serving Rust FEM agent at tcp://127.0.0.1:${port}"
    cargo run -p kyuubiki-cli -- agent --port "$port"
  )
}

run_frontend() {
  local port="${1:-3000}"

  if port_in_use "$port"; then
    echo "Next.js workbench already running at http://127.0.0.1:${port}"
    return 0
  fi

  (
    cd "$FRONTEND_DIR"
    echo "serving Next.js workbench at http://127.0.0.1:${port}"
    npm run dev
  )
}

run_hot_stack() {
  local mode="${1:-local}"
  require_dir "$ROOT_DIR"
  (
    cd "$ROOT_DIR"
    eval "$(storage_mode_exports "$mode") $(deployment_mode_exports "$mode")" \
      KYUUBIKI_AGENT_ENDPOINTS="$(agent_endpoints_value)" \
      node ./scripts/hot-dev.mjs stack --mode "$mode" --orchestrator-port 4000 --frontend-port 3000 --agent-endpoints "$(agent_endpoints_value)"
  )
}

run_hot_web() {
  local mode="${1:-local}"
  require_dir "$ROOT_DIR"
  (
    cd "$ROOT_DIR"
    eval "$(storage_mode_exports "$mode") $(deployment_mode_exports "$mode")" \
      KYUUBIKI_AGENT_ENDPOINTS="$(agent_endpoints_value)" \
      node ./scripts/hot-dev.mjs web --mode "$mode" --port 4000
  )
}

run_hot_agent() {
  local port="${1:-5001}"
  require_dir "$ROOT_DIR"
  (
    cd "$ROOT_DIR"
    node ./scripts/hot-dev.mjs agent --port "$port"
  )
}

run_hot_hub_gui() {
  require_dir "$ROOT_DIR"
  (
    cd "$ROOT_DIR"
    node ./scripts/hot-dev.mjs hub-gui
  )
}

run_hot_installer_gui() {
  require_dir "$ROOT_DIR"
  (
    cd "$ROOT_DIR"
    node ./scripts/hot-dev.mjs installer-gui
  )
}

run_hot_workbench_gui() {
  require_dir "$ROOT_DIR"
  (
    cd "$ROOT_DIR"
    node ./scripts/hot-dev.mjs workbench-gui
  )
}

run_benchmark() {
  require_dir "$RUST_DIR"
  (
    cd "$RUST_DIR"
    cargo run -p kyuubiki-benchmark -- "$@"
  )
}

run_frontend_cli() {
  require_dir "$FRONTEND_DIR"
  (
    cd "$FRONTEND_DIR"
    node ./scripts/kyuubiki-cli.mjs "$@"
  )
}

run_export_db() {
  local url="${1:-http://127.0.0.1:4000/api/v1/export/database}"

  if ! port_in_use 4000; then
    echo "orchestrator is not running at http://127.0.0.1:4000" >&2
    exit 1
  fi

  curl -fsS "$url"
}

run_installer() {
  require_dir "$RUST_DIR"
  (
    cd "$RUST_DIR"
    cargo run -p kyuubiki-installer -- "$@"
  )
}

ensure_desktop_cli_ready() {
  local app_dir="$1"
  local app_name="$2"

  if [[ ! -d "$app_dir/node_modules/@tauri-apps/cli" ]]; then
    echo "${app_name} desktop dependencies are missing under ${app_dir}/node_modules" >&2
    echo "run: cd \"$app_dir\" && npm install" >&2
    return 1
  fi
}

desktop_tauri_conf_for_app() {
  local app="$1"
  case "$app" in
    hub-gui) echo "$HUB_GUI_DIR/src-tauri/tauri.conf.json" ;;
    installer-gui) echo "$INSTALLER_GUI_DIR/src-tauri/tauri.conf.json" ;;
    workbench-gui) echo "$WORKBENCH_GUI_DIR/src-tauri/tauri.conf.json" ;;
    *)
      echo "unknown desktop app: $app" >&2
      return 1
      ;;
  esac
}

desktop_product_name_for_app() {
  local app="$1"
  local conf product_name
  conf="$(desktop_tauri_conf_for_app "$app")" || return 1
  product_name="$(sed -n 's/.*"productName":[[:space:]]*"\([^"]*\)".*/\1/p' "$conf" | head -n 1)"
  echo "${product_name:-$app}"
}

desktop_version_for_app() {
  local app="$1"
  local conf version
  conf="$(desktop_tauri_conf_for_app "$app")" || return 1
  version="$(sed -n 's/.*"version":[[:space:]]*"\([^"]*\)".*/\1/p' "$conf" | head -n 1)"
  echo "${version:-0.0.0}"
}

desktop_macos_app_bundle_path_for_app() {
  local app="$1"
  local product_name bundle_dir
  product_name="$(desktop_product_name_for_app "$app")" || return 1
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"
  echo "$bundle_dir/macos/${product_name}.app"
}

desktop_macos_dmg_path_for_app() {
  local app="$1"
  local product_name version arch bundle_dir
  product_name="$(desktop_product_name_for_app "$app")" || return 1
  version="$(desktop_version_for_app "$app")" || return 1
  arch="$(uname -m)"
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"
  echo "$bundle_dir/dmg/${product_name}_${version}_${arch}.dmg"
}

ensure_fallback_macos_dmg_for_app() {
  local app="$1"
  local app_bundle dmg_path product_name

  if [[ "$(host_platform)" != "macos" ]]; then
    return 1
  fi

  app_bundle="$(desktop_macos_app_bundle_path_for_app "$app")" || return 1
  dmg_path="$(desktop_macos_dmg_path_for_app "$app")" || return 1
  product_name="$(desktop_product_name_for_app "$app")" || return 1

  if [[ ! -d "$app_bundle" ]]; then
    return 1
  fi

  if [[ -f "$dmg_path" ]]; then
    return 0
  fi

  mkdir -p "$(dirname "$dmg_path")"
  echo "creating fallback dmg for ${app} at ${dmg_path}"
  if hdiutil create -volname "$product_name" -srcfolder "$app_bundle" -ov -format UDZO "$dmg_path"; then
    return 0
  fi

  echo "fallback dmg creation failed for ${app}; this host session may not support hdiutil disk image creation" >&2
  return 1
}

run_hub_gui_dev() {
  require_dir "$HUB_GUI_DIR"
  ensure_desktop_cli_ready "$HUB_GUI_DIR" "hub-gui"
  (
    cd "$HUB_GUI_DIR"
    npm run tauri:dev
  )
}

run_hub_gui_build() {
  local platform="${1:-$(host_platform)}"
  local build_exit=0
  require_dir "$HUB_GUI_DIR"
  ensure_desktop_cli_ready "$HUB_GUI_DIR" "hub-gui"
  if (
    cd "$HUB_GUI_DIR"
    if [[ "$platform" == "$(host_platform)" ]]; then
      npm run tauri:build
    else
      echo "hub-gui cross-platform bundle build is not performed on this host; staging ${platform} desktop manifest instead"
    fi
  ); then
    build_exit=0
  else
    build_exit=$?
  fi
  if [[ "$platform" == "$(host_platform)" && "$platform" == "macos" && "$build_exit" -ne 0 ]]; then
    if ensure_fallback_macos_dmg_for_app "hub-gui"; then
      echo "hub-gui tauri dmg bundling failed; fallback dmg created from app bundle"
      build_exit=0
    fi
  fi
  run_installer stage-release "$platform"
  return "$build_exit"
}

run_installer_gui_dev() {
  require_dir "$INSTALLER_GUI_DIR"
  ensure_desktop_cli_ready "$INSTALLER_GUI_DIR" "installer-gui"
  (
    cd "$INSTALLER_GUI_DIR"
    npm run tauri:dev
  )
}

run_installer_gui_build() {
  local platform="${1:-$(host_platform)}"
  local build_exit=0
  require_dir "$INSTALLER_GUI_DIR"
  ensure_desktop_cli_ready "$INSTALLER_GUI_DIR" "installer-gui"
  if (
    cd "$INSTALLER_GUI_DIR"
    if [[ "$platform" == "$(host_platform)" ]]; then
      npm run tauri:build
    else
      echo "installer-gui cross-platform bundle build is not performed on this host; staging ${platform} desktop manifest instead"
    fi
  ); then
    build_exit=0
  else
    build_exit=$?
  fi
  if [[ "$platform" == "$(host_platform)" && "$platform" == "macos" && "$build_exit" -ne 0 ]]; then
    if ensure_fallback_macos_dmg_for_app "installer-gui"; then
      echo "installer-gui tauri dmg bundling failed; fallback dmg created from app bundle"
      build_exit=0
    fi
  fi
  run_installer stage-release "$platform"
  return "$build_exit"
}

run_workbench_gui_dev() {
  require_dir "$WORKBENCH_GUI_DIR"
  ensure_desktop_cli_ready "$WORKBENCH_GUI_DIR" "workbench-gui"
  (
    cd "$WORKBENCH_GUI_DIR"
    npm run tauri:dev
  )
}

run_workbench_gui_build() {
  local platform="${1:-$(host_platform)}"
  local build_exit=0
  require_dir "$WORKBENCH_GUI_DIR"
  ensure_desktop_cli_ready "$WORKBENCH_GUI_DIR" "workbench-gui"
  if (
    cd "$WORKBENCH_GUI_DIR"
    if [[ "$platform" == "$(host_platform)" ]]; then
      npm run tauri:build
    else
      echo "workbench-gui cross-platform bundle build is not performed on this host; staging ${platform} desktop manifest instead"
    fi
  ); then
    build_exit=0
  else
    build_exit=$?
  fi
  if [[ "$platform" == "$(host_platform)" && "$platform" == "macos" && "$build_exit" -ne 0 ]]; then
    if ensure_fallback_macos_dmg_for_app "workbench-gui"; then
      echo "workbench-gui tauri dmg bundling failed; fallback dmg created from app bundle"
      build_exit=0
    fi
  fi
  run_installer stage-release "$platform"
  return "$build_exit"
}

run_build_frontend() {
  require_dir "$FRONTEND_DIR"
  (
    cd "$FRONTEND_DIR"
    npm run build
  )
}

run_build_orchestrator() {
  require_dir "$WEB_DIR"
  (
    cd "$WEB_DIR"
    MIX_ENV=prod mix compile
  )
}

run_build_agent() {
  require_dir "$RUST_DIR"
  (
    cd "$RUST_DIR"
    cargo build -p kyuubiki-cli --release
  )
}

run_package_runtime() {
  run_installer stage-release "$@"
}

run_package_desktop() {
  local platform="${1:-$(host_platform)}"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      run_installer stage-release "$target"
    done
    run_hub_gui_build "$(host_platform)"
    run_installer_gui_build "$(host_platform)"
    run_workbench_gui_build "$(host_platform)"
    return 0
  fi

  run_installer stage-release "$platform"
  run_hub_gui_build "$platform"
  run_installer_gui_build "$platform"
  run_workbench_gui_build "$platform"
}

desktop_target_apps() {
  echo "hub-gui installer-gui workbench-gui"
}

desktop_icon_dir_for_app() {
  local app="$1"
  echo "$ROOT_DIR/apps/${app}/src-tauri/icons"
}

desktop_has_icon_pattern() {
  local app="$1"
  local pattern="$2"
  local icon_dir candidate
  icon_dir="$(desktop_icon_dir_for_app "$app")"

  for candidate in "$icon_dir"/$~pattern; do
    if [[ -f "$candidate" ]]; then
      return 0
    fi
  done

  return 1
}

desktop_icon_status() {
  local platform="$1"
  local app="$2"

  case "$platform" in
    macos)
      if desktop_has_icon_pattern "$app" "*.png" && desktop_has_icon_pattern "$app" "*.icns"; then
        echo "ready (.png + .icns)"
      else
        echo "missing macOS icons"
      fi
      ;;
    linux)
      if desktop_has_icon_pattern "$app" "*.png"; then
        echo "ready (.png)"
      else
        echo "missing Linux icons"
      fi
      ;;
    windows)
      if desktop_has_icon_pattern "$app" "*.png" && desktop_has_icon_pattern "$app" "*.ico"; then
        echo "ready (.png + .ico)"
      else
        echo "missing Windows icons"
      fi
      ;;
    *)
      echo "unknown"
      ;;
  esac
}

desktop_manifest_status() {
  local platform="$1"
  local app="$2"
  local manifest="$ROOT_DIR/dist/${platform}/desktop/${app}/manifest.json"

  if [[ -f "$manifest" ]]; then
    echo "present"
  else
    echo "missing"
  fi
}

desktop_bundle_dir_for_app() {
  local app="$1"
  echo "$ROOT_DIR/apps/${app}/src-tauri/target/release/bundle"
}

json_escape() {
  local value="${1:-}"
  value="${value//\\/\\\\}"
  value="${value//\"/\\\"}"
  value="${value//$'\n'/ }"
  echo "$value"
}

desktop_artifact_stage_dir_for_app() {
  local platform="$1"
  local app="$2"
  echo "$ROOT_DIR/dist/${platform}/desktop/${app}/artifacts"
}

desktop_artifact_manifest_path_for_app() {
  local platform="$1"
  local app="$2"
  echo "$ROOT_DIR/dist/${platform}/desktop/${app}/artifacts.json"
}

desktop_artifact_summary_path() {
  local platform="$1"
  echo "$ROOT_DIR/dist/${platform}/desktop/artifacts-summary.json"
}

desktop_build_summary_path() {
  local platform="$1"
  echo "$ROOT_DIR/dist/${platform}/desktop/build-summary.json"
}

desktop_expected_artifact_count() {
  local platform="$1"
  case "$platform" in
    macos) echo 2 ;;
    linux) echo 3 ;;
    windows) echo 2 ;;
    *) echo 0 ;;
  esac
}

desktop_artifact_count_for_app() {
  local platform="$1"
  local app="$2"
  local manifest_path artifact_count
  manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"
  artifact_count=0

  if [[ -f "$manifest_path" ]]; then
    artifact_count="$(sed -n 's/.*"artifact_count":[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$manifest_path" | head -n 1)"
    artifact_count="${artifact_count:-0}"
  fi

  echo "$artifact_count"
}

desktop_build_status_for_app() {
  local platform="$1"
  local app="$2"
  local artifact_count expected_count
  artifact_count="$(desktop_artifact_count_for_app "$platform" "$app")"
  expected_count="$(desktop_expected_artifact_count "$platform")"

  if [[ "$artifact_count" -le 0 ]]; then
    echo "failed"
  elif [[ "$artifact_count" -lt "$expected_count" ]]; then
    echo "partial"
  else
    echo "built"
  fi
}

collect_desktop_artifacts_for_app() {
  local platform="$1"
  local app="$2"
  local bundle_dir dest_dir manifest_path
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"
  dest_dir="$(desktop_artifact_stage_dir_for_app "$platform" "$app")"
  manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"

  mkdir -p "$(dirname "$manifest_path")"
  rm -rf "$dest_dir"
  mkdir -p "$dest_dir"

  local artifacts_json=""
  local artifact_count=0

  add_desktop_artifact() {
    local kind="$1"
    local candidate="${2:-}"

    if [[ -z "$candidate" || ! -e "$candidate" ]]; then
      return 0
    fi

    local name staged_path entry_type
    name="$(basename "$candidate")"
    staged_path="$dest_dir/$name"
    rm -rf "$staged_path"
    cp -R "$candidate" "$staged_path"

    if [[ -d "$candidate" ]]; then
      entry_type="directory"
    else
      entry_type="file"
    fi

    if [[ -n "$artifacts_json" ]]; then
      artifacts_json+=","
      artifacts_json+=$'\n'
    fi

    artifacts_json+="    {"
    artifacts_json+=$'\n'
    artifacts_json+="      \"kind\": \"$(json_escape "$kind")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"name\": \"$(json_escape "$name")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"type\": \"$(json_escape "$entry_type")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"source_path\": \"$(json_escape "$candidate")\","
    artifacts_json+=$'\n'
    artifacts_json+="      \"staged_path\": \"$(json_escape "$staged_path")\""
    artifacts_json+=$'\n'
    artifacts_json+="    }"

    artifact_count=$((artifact_count + 1))
  }

  case "$platform" in
    macos)
      add_desktop_artifact "app" "$bundle_dir/macos/"*.app(N)
      add_desktop_artifact "dmg" "$bundle_dir/dmg/"*.dmg(N)
      ;;
    linux)
      add_desktop_artifact "appimage" "$bundle_dir/appimage/"*.AppImage(N)
      add_desktop_artifact "deb" "$bundle_dir/deb/"*.deb(N)
      add_desktop_artifact "rpm" "$bundle_dir/rpm/"*.rpm(N)
      ;;
    windows)
      add_desktop_artifact "msi" "$bundle_dir/msi/"*.msi(N)
      add_desktop_artifact "nsis" "$bundle_dir/nsis/"*.exe(N)
      ;;
  esac

  cat > "$manifest_path" <<EOF
{
  "schema_version": "kyuubiki.desktop-artifacts/v1",
  "app": "$(json_escape "$app")",
  "platform": "$(json_escape "$platform")",
  "bundle_dir": "$(json_escape "$bundle_dir")",
  "artifact_count": $artifact_count,
  "artifacts": [
${artifacts_json}
  ]
}
EOF
}

collect_host_desktop_artifacts() {
  local platform="${1:-$(host_platform)}"
  local summary_path app manifest_path summary_entries="" artifact_count=0
  summary_path="$(desktop_artifact_summary_path "$platform")"
  mkdir -p "$(dirname "$summary_path")"

  for app in ${(s: :)$(desktop_target_apps)}; do
    collect_desktop_artifacts_for_app "$platform" "$app"
    manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"

    local app_count=0
    app_count="$(desktop_artifact_count_for_app "$platform" "$app")"

    if [[ -n "$summary_entries" ]]; then
      summary_entries+=","
      summary_entries+=$'\n'
    fi
    summary_entries+="    {"
    summary_entries+=$'\n'
    summary_entries+="      \"app\": \"$(json_escape "$app")\","
    summary_entries+=$'\n'
    summary_entries+="      \"artifact_manifest\": \"$(json_escape "$manifest_path")\","
    summary_entries+=$'\n'
    summary_entries+="      \"artifact_count\": ${app_count}"
    summary_entries+=$'\n'
    summary_entries+="    }"

    artifact_count=$((artifact_count + app_count))
  done

  cat > "$summary_path" <<EOF
{
  "schema_version": "kyuubiki.desktop-artifact-summary/v1",
  "platform": "$(json_escape "$platform")",
  "artifact_count": $artifact_count,
  "apps": [
${summary_entries}
  ]
}
EOF

  echo "collected host desktop artifacts under $ROOT_DIR/dist/${platform}/desktop"
}

write_desktop_build_summary() {
  local platform="$1"
  shift

  local summary_path app build_status summary_entries=""
  summary_path="$(desktop_build_summary_path "$platform")"
  mkdir -p "$(dirname "$summary_path")"

  while [[ $# -gt 0 ]]; do
    app="$1"
    build_status="$2"
    shift 2

    if [[ -n "$summary_entries" ]]; then
      summary_entries+=","
      summary_entries+=$'\n'
    fi

    summary_entries+="    {"
    summary_entries+=$'\n'
    summary_entries+="      \"app\": \"$(json_escape "$app")\","
    summary_entries+=$'\n'
    summary_entries+="      \"status\": \"$(json_escape "$build_status")\","
    summary_entries+=$'\n'
    summary_entries+="      \"artifact_manifest\": \"$(json_escape "$(desktop_artifact_manifest_path_for_app "$platform" "$app")")\""
    summary_entries+=$'\n'
    summary_entries+="    }"
  done

  cat > "$summary_path" <<EOF
{
  "schema_version": "kyuubiki.desktop-build-summary/v1",
  "platform": "$(json_escape "$platform")",
  "apps": [
${summary_entries}
  ]
}
EOF
}

desktop_host_bundle_status() {
  local app="$1"
  local bundle_dir
  bundle_dir="$(desktop_bundle_dir_for_app "$app")"

  if [[ -d "$bundle_dir" ]]; then
    echo "present"
  else
    echo "missing"
  fi
}

desktop_host_artifact_status() {
  local platform="$1"
  local app="$2"
  local manifest_path artifact_count
  manifest_path="$(desktop_artifact_manifest_path_for_app "$platform" "$app")"

  if [[ ! -f "$manifest_path" ]]; then
    echo "missing"
    return 0
  fi

  artifact_count="$(sed -n 's/.*"artifact_count":[[:space:]]*\([0-9][0-9]*\).*/\1/p' "$manifest_path" | head -n 1)"
  artifact_count="${artifact_count:-0}"

  if [[ "$artifact_count" -gt 0 ]]; then
    echo "present (${artifact_count})"
  else
    echo "empty"
  fi
}

desktop_runtime_stage_status() {
  local platform="$1"
  local root="$ROOT_DIR/dist/${platform}"

  if [[ -d "$root/bin" && -d "$root/config" && -d "$root/desktop" ]]; then
    echo "present"
  else
    echo "missing"
  fi
}

print_desktop_status_for_platform() {
  local platform="$1"
  local runtime_status
  runtime_status="$(desktop_runtime_stage_status "$platform")"

  echo
  echo "platform: ${platform}"
  echo "  runtime scaffold: ${runtime_status}"

  local app manifest_status icon_status host_bundle_status
  local host_artifact_status
  for app in ${(s: :)$(desktop_target_apps)}; do
    manifest_status="$(desktop_manifest_status "$platform" "$app")"
    icon_status="$(desktop_icon_status "$platform" "$app")"
    printf '  %-16s manifest=%-8s icons=%s\n' "${app}:" "$manifest_status" "$icon_status"

    if [[ "$platform" == "$(host_platform)" ]]; then
      host_bundle_status="$(desktop_host_bundle_status "$app")"
      host_artifact_status="$(desktop_host_artifact_status "$platform" "$app")"
      printf '  %-16s host-bundle=%-8s staged-artifacts=%s\n' "${app}:" "$host_bundle_status" "$host_artifact_status"
    fi
  done

  if verify_desktop_platform "$platform" >/dev/null 2>&1; then
    echo "  verification: ready"
  else
    echo "  verification: needs attention"
  fi
}

print_desktop_next_steps() {
  local platform="$1"
  local host
  host="$(host_platform)"

  echo
  echo "next steps:"

  if [[ "$platform" == "all" ]]; then
    echo "  - Stage every platform scaffold: ./scripts/kyuubiki desktop-stage all"
    echo "  - Build this host's desktop bundles: ./scripts/kyuubiki desktop-build-host"
    echo "  - Verify manifests and icon inputs: ./scripts/kyuubiki desktop-verify all"
    echo "  - Review staged bundle manifests under: dist/<host>/desktop/*/artifacts.json"
    return 0
  fi

  if [[ "$(desktop_runtime_stage_status "$platform")" == "missing" ]]; then
    echo "  - Stage runtime + desktop manifests: ./scripts/kyuubiki desktop-stage ${platform}"
  fi

  if [[ "$platform" == "$host" ]]; then
    echo "  - Build host-native Tauri bundles: ./scripts/kyuubiki desktop-build-host"
    echo "  - Run the full host release pass: ./scripts/kyuubiki desktop-release ${platform}"
    echo "  - Review staged bundle manifests under: dist/${platform}/desktop/*/artifacts.json"
  else
    echo "  - This host only stages ${platform} manifests; build native bundles on a ${platform} machine"
    echo "  - Verify staged rollout descriptors: ./scripts/kyuubiki desktop-verify ${platform}"
  fi
}

run_desktop_status() {
  local platform="${1:-$(host_platform)}"
  local host
  host="$(host_platform)"

  echo "desktop packaging status"
  echo "  host platform: ${host}"
  echo "  dist root: $ROOT_DIR/dist"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      print_desktop_status_for_platform "$target"
    done
    print_desktop_next_steps all
    return 0
  fi

  print_desktop_status_for_platform "$platform"
  print_desktop_next_steps "$platform"
}

run_desktop_stage() {
  local platform="${1:-$(host_platform)}"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      run_installer stage-release "$target"
    done
    return 0
  fi

  run_installer stage-release "$platform"
}

run_desktop_build_host() {
  local platform
  platform="$(host_platform)"
  local hub_status="failed"
  local installer_status="failed"
  local workbench_status="failed"
  local build_failed=0

  if run_hub_gui_build "$platform"; then
    :
  else
    :
  fi

  if run_installer_gui_build "$platform"; then
    :
  else
    :
  fi

  if run_workbench_gui_build "$platform"; then
    :
  else
    :
  fi

  collect_host_desktop_artifacts "$platform"
  hub_status="$(desktop_build_status_for_app "$platform" "hub-gui")"
  installer_status="$(desktop_build_status_for_app "$platform" "installer-gui")"
  workbench_status="$(desktop_build_status_for_app "$platform" "workbench-gui")"

  if [[ "$hub_status" != "built" || "$installer_status" != "built" || "$workbench_status" != "built" ]]; then
    build_failed=1
  fi

  write_desktop_build_summary "$platform" \
    "hub-gui" "$hub_status" \
    "installer-gui" "$installer_status" \
    "workbench-gui" "$workbench_status"

  if [[ "$build_failed" -ne 0 ]]; then
    echo "desktop host build finished with failures; see $(desktop_build_summary_path "$platform")" >&2
    return 1
  fi
}

verify_manifest_bundle_kind() {
  local manifest_path="$1"
  local expected_fragment="$2"

  if ! grep -q "$expected_fragment" "$manifest_path"; then
    echo "missing bundle kind ${expected_fragment} in ${manifest_path}" >&2
    return 1
  fi
}

verify_desktop_icons() {
  local platform="$1"
  local app_dir="$2"
  local label="$3"

  local icon_dir="$ROOT_DIR/apps/${app_dir}/src-tauri/icons"
  local required=()

  case "$platform" in
    macos) required=("*.png" "*.icns") ;;
    linux) required=("*.png") ;;
    windows) required=("*.png" "*.ico") ;;
    *)
      echo "unsupported platform for icon verification: $platform" >&2
      return 1
      ;;
  esac

  local pattern match_found
  for pattern in "${required[@]}"; do
    match_found=0
    for candidate in "$icon_dir"/$~pattern; do
      if [[ -f "$candidate" ]]; then
        match_found=1
        break
      fi
    done

    if [[ "$match_found" -ne 1 ]]; then
      echo "missing ${pattern} icon input for ${label} under ${icon_dir}" >&2
      return 1
    fi
  done

  echo "ok: ${label} icon inputs for ${platform}"
}

verify_desktop_platform() {
  local platform="$1"
  local desktop_root="$ROOT_DIR/dist/${platform}/desktop"

  if [[ ! -d "$desktop_root" ]]; then
    echo "missing staged desktop directory: $desktop_root" >&2
    return 1
  fi

  local apps=("hub-gui" "installer-gui" "workbench-gui")
  local app manifest
  for app in "${apps[@]}"; do
    manifest="$desktop_root/$app/manifest.json"
    if [[ ! -f "$manifest" ]]; then
      echo "missing desktop manifest: $manifest" >&2
      return 1
    fi

    case "$platform" in
      macos)
        verify_manifest_bundle_kind "$manifest" "app"
        verify_manifest_bundle_kind "$manifest" "dmg"
        ;;
      linux)
        verify_manifest_bundle_kind "$manifest" "appimage"
        verify_manifest_bundle_kind "$manifest" "deb"
        verify_manifest_bundle_kind "$manifest" "rpm"
        ;;
      windows)
        verify_manifest_bundle_kind "$manifest" "msi"
        verify_manifest_bundle_kind "$manifest" "nsis"
        ;;
    esac
  done

  verify_desktop_icons "$platform" "hub-gui" "hub-gui"
  verify_desktop_icons "$platform" "installer-gui" "installer-gui"
  verify_desktop_icons "$platform" "workbench-gui" "workbench-gui"

  echo "desktop release verification passed for ${platform}"
}

run_desktop_verify() {
  local platform="${1:-$(host_platform)}"

  if [[ "$platform" == "all" ]]; then
    local target
    for target in macos linux windows; do
      verify_desktop_platform "$target"
    done
    return 0
  fi

  verify_desktop_platform "$platform"
}

run_desktop_release() {
  local platform="${1:-$(host_platform)}"
  run_desktop_stage "$platform"
  run_desktop_build_host
  run_desktop_verify "$platform"
  echo "desktop release artifacts staged under $ROOT_DIR/dist/$(host_platform)/desktop"
}

start_orchestrator_background() {
  local port="${1:-4000}"
  local mode="${2:-default}"
  ensure_run_dir
  require_screen

  if port_in_use "$port"; then
    echo "orchestrator already running at http://127.0.0.1:${port}"
    return 0
  fi

  screen -S "$ORCHESTRATOR_SCREEN" -X quit >/dev/null 2>&1 || true
  screen -dmS "$ORCHESTRATOR_SCREEN" sh -lc "cd \"$WEB_DIR\" && $(storage_mode_exports "$mode") $(deployment_mode_exports "$mode") KYUUBIKI_AGENT_ENDPOINTS=\"$(agent_endpoints_value)\" PORT=\"$port\" mix run --no-halt >> \"$ORCHESTRATOR_LOG\" 2>&1"
  wait_for_port_state "$port" listening 15 || true

  echo "started orchestrator API at http://127.0.0.1:${port} ($(storage_mode_label "$mode"), $(deployment_mode_label "$mode"))"
  echo "log: $ORCHESTRATOR_LOG"
}

start_agent_background() {
  local port="${1:-5001}"
  local log_file="$RUN_DIR/agent-${port}.log"
  local screen_name="${AGENT_SCREEN_PREFIX}_${port}"
  ensure_run_dir
  require_screen

  if port_in_use "$port"; then
    echo "Rust FEM agent already running at tcp://127.0.0.1:${port}"
    return 0
  fi

  screen -S "$screen_name" -X quit >/dev/null 2>&1 || true
  screen -dmS "$screen_name" sh -lc "cd \"$RUST_DIR\" && cargo run -p kyuubiki-cli -- agent --port \"$port\" >> \"$log_file\" 2>&1"
  wait_for_port_state "$port" listening 20 || true

  echo "started Rust FEM agent at tcp://127.0.0.1:${port}"
  echo "log: $log_file"
}

start_agents_background() {
  local agent_ports=("${(@s:,:)$(agent_endpoints_value)}")
  local agent_port_spec agent_port

  for agent_port_spec in "${agent_ports[@]}"; do
    agent_port="${agent_port_spec##*:}"
    start_agent_background "$agent_port"
  done
}

start_frontend_background() {
  local port="${1:-3000}"
  ensure_run_dir
  require_screen

  if port_in_use "$port"; then
    echo "Next.js workbench already running at http://127.0.0.1:${port}"
    return 0
  fi

  screen -S "$FRONTEND_SCREEN" -X quit >/dev/null 2>&1 || true
  screen -dmS "$FRONTEND_SCREEN" sh -lc "cd \"$FRONTEND_DIR\" && npm run dev >> \"$FRONTEND_LOG\" 2>&1"
  wait_for_port_state "$port" listening 20 || true

  echo "started Next.js workbench at http://127.0.0.1:${port}"
  echo "log: $FRONTEND_LOG"
}

hot_screen_running() {
  screen -ls 2>/dev/null | grep -q "[[:space:]]${HOT_STACK_SCREEN}[[:space:]]"
}

hot_mode_label() {
  local mode="${1:-local}"
  case "$mode" in
    cloud) echo "cloud" ;;
    distributed) echo "distributed" ;;
    *) echo "local" ;;
  esac
}

start_hot_stack_background() {
  local mode="${1:-local}"
  ensure_run_dir
  ensure_hot_run_dir
  require_screen

  if hot_screen_running; then
    echo "managed hot-reload loop already running in screen session ${HOT_STACK_SCREEN}"
    return 0
  fi

  screen -S "$HOT_STACK_SCREEN" -X quit >/dev/null 2>&1 || true
  screen -dmS "$HOT_STACK_SCREEN" sh -lc "cd \"$ROOT_DIR\" && $(storage_mode_exports "$mode") $(deployment_mode_exports "$mode") KYUUBIKI_AGENT_ENDPOINTS=\"$(agent_endpoints_value)\" KYUUBIKI_HOT_LOG_DIR=\"$HOT_RUN_DIR\" node ./scripts/hot-dev.mjs stack --mode \"$mode\" --orchestrator-port 4000 --frontend-port 3000 --agent-endpoints \"$(agent_endpoints_value)\" >> \"$HOT_RUN_DIR/stack.console.log\" 2>&1"

  echo "started managed hot-reload loop (${mode}) in screen session ${HOT_STACK_SCREEN}"
  echo "logs: $HOT_RUN_DIR"
}

stop_hot_stack_background() {
  require_screen

  if hot_screen_running; then
    screen -S "$HOT_STACK_SCREEN" -X quit >/dev/null 2>&1 || true
    echo "stopped managed hot-reload loop"
  else
    echo "managed hot-reload loop is not running"
  fi
}

show_hot_status() {
  local agent_ports=("${(@s:,:)$(agent_endpoints_value)}")
  local agent_port_spec agent_port agent_pid

  if hot_screen_running; then
    echo "hot-loop: running (${HOT_STACK_SCREEN})"
  else
    echo "hot-loop: stopped"
  fi

  if port_in_use 4000; then
    echo "hot-web: listening on http://127.0.0.1:4000 (pid $(pid_on_port 4000))"
  else
    echo "hot-web: stopped"
  fi

  if port_in_use 3000; then
    echo "hot-frontend: listening on http://127.0.0.1:3000 (pid $(pid_on_port 3000))"
  else
    echo "hot-frontend: stopped"
  fi

  for agent_port_spec in "${agent_ports[@]}"; do
    agent_port="${agent_port_spec##*:}"
    agent_pid="$(pid_on_port "$agent_port")"
    if [[ -n "${agent_pid:-}" ]]; then
      echo "hot-agent[$agent_port]: listening on tcp://127.0.0.1:${agent_port} (pid $agent_pid)"
    else
      echo "hot-agent[$agent_port]: stopped"
    fi
  done

  if [[ -d "$HOT_RUN_DIR" ]]; then
    echo "hot-logs: $HOT_RUN_DIR"
  fi
}

stop_services() {
  local agent_ports=("${(@s:,:)$(agent_endpoints_value)}")
  local agent_port_spec

  for agent_port_spec in "${agent_ports[@]}"; do
    stop_port "${agent_port_spec##*:}"
  done
  stop_port 3000
  stop_port 4000
}

start_services() {
  local mode="${1:-default}"
  if [[ "$mode" != "distributed" ]]; then
    start_agents_background
  fi
  start_orchestrator_background 4000 "$mode"
  start_frontend_background 3000
}

restart_services() {
  local mode="${1:-default}"
  stop_services
  start_services "$mode"
  echo "restart complete"
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
