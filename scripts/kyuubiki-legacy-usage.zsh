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
  rust-test         Run Rust tests and the Rust line-count audit
  rust-line-audit   Enforce the Rust source file line-count ceiling
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
