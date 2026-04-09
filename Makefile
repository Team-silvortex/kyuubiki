SHELL := /bin/zsh
ENTRYPOINT := zsh ./scripts/kyuubiki

.PHONY: help tree start start-local start-cloud start-distributed status stop restart restart-local restart-cloud restart-distributed export-db install doctor validate-env package installer-gui-dev installer-gui-build workbench-gui-dev workbench-gui-build test test-web test-rust test-playground test-workbench-gui test-integration-api verify format format-web format-rust tdd-web tdd-rust smoke worker agent orchestrator playground frontend benchmark benchmark-baseline benchmark-compare benchmark-report

help:
	@echo "Available targets:"
	@echo "  make tree        Print the repository scaffold"
	@echo "  make start       Start the orchestrator API, frontend, and solver agent"
	@echo "  make start-local Start services with SQLite forced for local development"
	@echo "  make start-cloud Start services with PostgreSQL forced for cloud/distributed use"
	@echo "  make start-distributed Start only the control plane for distributed deployments"
	@echo "  make status      Show local service status"
	@echo "  make stop        Stop the orchestrator API, frontend, and solver agent"
	@echo "  make restart     Restart the orchestrator API, frontend, and solver agent"
	@echo "  make restart-local Restart services with SQLite forced for local development"
	@echo "  make restart-cloud Restart services with PostgreSQL forced for cloud/distributed use"
	@echo "  make restart-distributed Restart only the control plane for distributed deployments"
	@echo "  make export-db   Export the current database snapshot as JSON"
	@echo "  make install     Run the cross-platform installer/bootstrap utility"
	@echo "  make doctor      Check local prerequisites for this platform"
	@echo "  make validate-env Validate required .env.local configuration"
	@echo "  make package     Stage a portable release directory under dist/"
	@echo "  make installer-gui-dev   Run the Tauri installer GUI in development mode"
	@echo "  make installer-gui-build Build the Tauri installer GUI bundles"
	@echo "  make workbench-gui-dev   Run the Tauri desktop workbench shell in development mode"
	@echo "  make workbench-gui-build Build the Tauri desktop workbench shell bundles"
	@echo "  make test        Run all project tests"
	@echo "  make test-web    Run Elixir tests"
	@echo "  make test-rust   Run Rust workspace tests"
	@echo "  make test-workbench-gui Run desktop workbench shell smoke tests"
	@echo "  make test-integration-api Run the local orchestrator + agent + API integration smoke test"
	@echo "  make verify      Run formatting checks and tests"
	@echo "  make format      Format all code"
	@echo "  make smoke       Run the Elixir -> Rust smoke flow"
	@echo "  make worker      Run the Rust mock worker CLI"
	@echo "  make agent       Run the Rust FEM TCP agent"
	@echo "  make orchestrator Run the Elixir orchestrator API"
	@echo "  make playground  Legacy alias for the orchestrator API"
	@echo "  make frontend    Run the Next.js workbench UI"
	@echo "  make benchmark   Run the Rust solver benchmark suite"
	@echo "  make benchmark-baseline Write a benchmark baseline snapshot (PROFILE=medium by default)"
	@echo "  make benchmark-compare Compare current benchmark output against a checked-in baseline"
	@echo "  make benchmark-report Write a Markdown comparison report against a checked-in baseline"
	@echo "  make tdd-web     Run a focused Elixir test by FILE=... or TEST=..."
	@echo "  make tdd-rust    Run focused Rust tests with FILTER=..."
	@echo "  zsh ./scripts/kyuubiki help    Show the unified local entrypoint"
	@echo "  KYUUBIKI_STORAGE_BACKEND=postgres DATABASE_URL=ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev make start"
	@echo "  cp .env.example .env.local && make restart"

tree:
	@find . -maxdepth 3 -type d | sort

start:
	@$(ENTRYPOINT) start

start-local:
	@$(ENTRYPOINT) start-local

start-cloud:
	@$(ENTRYPOINT) start-cloud

start-distributed:
	@$(ENTRYPOINT) start-distributed

status:
	@$(ENTRYPOINT) status

stop:
	@$(ENTRYPOINT) stop

restart:
	@$(ENTRYPOINT) restart

restart-local:
	@$(ENTRYPOINT) restart-local

restart-cloud:
	@$(ENTRYPOINT) restart-cloud

restart-distributed:
	@$(ENTRYPOINT) restart-distributed

export-db:
	@$(ENTRYPOINT) export-db

install:
	@$(ENTRYPOINT) install $(ARGS)

doctor:
	@$(ENTRYPOINT) doctor

validate-env:
	@$(ENTRYPOINT) validate-env

package:
	@$(ENTRYPOINT) package $(ARGS)

installer-gui-dev:
	@$(ENTRYPOINT) installer-gui-dev

installer-gui-build:
	@$(ENTRYPOINT) installer-gui-build

workbench-gui-dev:
	@$(ENTRYPOINT) workbench-gui-dev

workbench-gui-build:
	@$(ENTRYPOINT) workbench-gui-build

test: test-web test-rust test-playground

test-web:
	@cd apps/web && mix test

test-rust:
	@cd workers/rust && cargo test

test-playground:
	@node --test apps/web/playground/test/fem.test.mjs

test-workbench-gui:
	@cd apps/workbench-gui && npm run test:smoke

test-integration-api:
	@node --test tests/integration/orchestrator-agent-api-smoke.test.mjs

format: format-web format-rust

format-web:
	@cd apps/web && mix format

format-rust:
	@cd workers/rust && cargo fmt

verify:
	@cd apps/web && mix format --check-formatted && mix test
	@cd workers/rust && cargo fmt --check && cargo test
	@cd workers/rust && cargo run -q -p kyuubiki-benchmark -- --profile $${PROFILE:-medium} --repeat $${REPEAT:-3} --baseline-compare benchmarks/$${PROFILE:-medium}-baseline.json --fail-on-median-regression-pct $${BENCHMARK_MEDIAN_THRESHOLD:-25} --fail-on-rss-regression-pct $${BENCHMARK_RSS_THRESHOLD:-20} --min-baseline-median-ms $${BENCHMARK_MIN_BASELINE_MS:-1.0}
	@node --test apps/web/playground/test/fem.test.mjs

tdd-web:
	@cd apps/web && mix test $(FILE) $(TEST)

tdd-rust:
	@cd workers/rust && cargo test $(FILTER)

smoke:
	@$(ENTRYPOINT) smoke

worker:
	@$(ENTRYPOINT) worker $(ARGS)

agent:
	@$(ENTRYPOINT) agent $(PORT)

orchestrator:
	@$(ENTRYPOINT) orchestrator $(PORT)

playground:
	@$(ENTRYPOINT) playground $(PORT)

frontend:
	@$(ENTRYPOINT) frontend

benchmark:
	@$(ENTRYPOINT) benchmark $(ARGS)

benchmark-baseline:
	@cd workers/rust && cargo run -q -p kyuubiki-benchmark -- --profile $${PROFILE:-medium} --repeat $${REPEAT:-5} --baseline-out benchmarks/$${PROFILE:-medium}-baseline.json

benchmark-compare:
	@cd workers/rust && cargo run -q -p kyuubiki-benchmark -- --profile $${PROFILE:-medium} --repeat $${REPEAT:-3} --baseline-compare benchmarks/$${PROFILE:-medium}-baseline.json --fail-on-median-regression-pct $${BENCHMARK_MEDIAN_THRESHOLD:-25} --fail-on-rss-regression-pct $${BENCHMARK_RSS_THRESHOLD:-20} --min-baseline-median-ms $${BENCHMARK_MIN_BASELINE_MS:-1.0}

benchmark-report:
	@mkdir -p workers/rust/benchmarks/reports
	@cd workers/rust && cargo run -q -p kyuubiki-benchmark -- --profile $${PROFILE:-medium} --repeat $${REPEAT:-3} --baseline-compare benchmarks/$${PROFILE:-medium}-baseline.json --compare-report-out benchmarks/reports/$${PROFILE:-medium}-compare.md
