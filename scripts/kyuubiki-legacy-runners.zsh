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
