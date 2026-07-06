SHELL := /bin/sh
ENTRYPOINT := ./scripts/kyuubiki

.PHONY: help tree
.PHONY: build-frontend build-orchestrator build-agent
.PHONY: build-hub-gui build-installer-gui build-workbench-gui
.PHONY: package-runtime package-desktop package
.PHONY: desktop-status desktop-stage desktop-build-host desktop-release desktop-verify
.PHONY: desktop-linux-remote desktop-linux-remote-install-deps desktop-linux-remote-preflight
.PHONY: operator-package-preflight sync-desktop-shared
.PHONY: build-installation-docs build-update-catalog
.PHONY: check-doc-book sync-doc-book-version check-toolchains check-elixir-self-host
.PHONY: check-language-packs check-ui-automation-contract check-version-line
.PHONY: build-operator-qualification-readiness
.PHONY: capture-line-field-qualification-provenance capture-line-field-qualification-release-evidence
.PHONY: check-line-field-closed-form-baseline check-line-field-qualification-release-evidence
.PHONY: check-operator-reliability-rules check-operator-reliability-schemas
.PHONY: check-operator-reliability audit-rust-lines audit-project-organization
.PHONY: audit-dependencies architecture-check verify
.PHONY: start start-local start-cloud start-distributed status stop restart
.PHONY: restart-local restart-cloud restart-distributed
.PHONY: hot-local hot-cloud hot-distributed hot-web hot-agent
.PHONY: hot-hub-gui hot-installer-gui hot-workbench-gui
.PHONY: export-db install doctor validate-env
.PHONY: hub-gui-dev hub-gui-build installer-gui-dev installer-gui-build
.PHONY: workbench-gui-dev workbench-gui-build
.PHONY: test test-web test-rust test-frontend workflow-preflight test-sdk
.PHONY: test-agent-capability-smoke test-playground
.PHONY: test-hub-gui test-installer-gui test-workbench-gui
.PHONY: test-integration test-integration-api test-integration-cluster
.PHONY: test-integration-direct-mesh test-integration-desktop-gui
.PHONY: test-integration-benchmark-profile-index
.PHONY: test-integration-direct-mesh-docker test-integration-remote-ssh-fixture
.PHONY: test-integration-direct-mesh-docker-compare
.PHONY: test-integration-direct-mesh-docker-report
.PHONY: test-integration-direct-mesh-docker-nightly
.PHONY: test-integration-workflow-mesh test-integration-workflow-mesh-nightly
.PHONY: test-integration-workflow-catalog-compare
.PHONY: test-integration-workflow-catalog-report
.PHONY: test-integration-workflow-catalog-nightly
.PHONY: test-integration-ui-mechanical test-integration-ui-thermal
.PHONY: format format-web format-rust tdd-web tdd-rust
.PHONY: smoke worker agent orchestrator playground frontend
.PHONY: benchmark benchmark-physics-coverage benchmark-profile-remote
.PHONY: benchmark-profile-report benchmark-profile-index
.PHONY: benchmark-baseline benchmark-compare benchmark-report
.PHONY: benchmark-standard-baselines benchmark-standard-compare
.PHONY: benchmark-standard-report benchmark-standard-nightly regression-gate-report

help:
	@echo "Available targets:"
	@echo "  make tree        Print the repository scaffold"
	@echo "  make build-frontend Build the Next.js workbench production bundle"
	@echo "  make build-orchestrator Compile the Elixir control plane in production mode"
	@echo "  make build-agent Build the Rust solver agent release binary"
	@echo "  make build-hub-gui Build the Tauri hub desktop bundles for PLATFORM=<host|macos|linux|windows>"
	@echo "  make build-installer-gui Build the Tauri installer desktop bundles for PLATFORM=<host|macos|linux|windows>"
	@echo "  make build-workbench-gui Build the Tauri workbench desktop bundles for PLATFORM=<host|macos|linux|windows>"
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
	@echo "  make hot-local  Run the local full-stack dev loop with hot reload/watch"
	@echo "  make hot-cloud  Run the cloud/postgres full-stack dev loop with hot reload/watch"
	@echo "  make hot-distributed Run the distributed control-plane dev loop with hot reload/watch"
	@echo "  make hot-web    Run the Elixir control plane with restart-on-change"
	@echo "  make hot-agent  Run the Rust solver agent with restart-on-change (PORT=5001 by default)"
	@echo "  make hot-hub-gui Run the Tauri Hub shell in dev/HMR mode"
	@echo "  make hot-installer-gui Run the Tauri installer shell in dev/HMR mode"
	@echo "  make hot-workbench-gui Run the Tauri workbench shell in dev/HMR mode"
	@echo "  make export-db   Export the current database snapshot as JSON"
	@echo "  make install     Run the cross-platform installer/bootstrap utility"
	@echo "  make doctor      Check local prerequisites for this platform"
	@echo "  make validate-env Validate required .env.local configuration"
	@echo "  make package     Stage a portable release directory under dist/"
	@echo "  make package-runtime Stage the portable runtime release scaffold under dist/"
	@echo "  make package-desktop Build/stage all Tauri desktop shells for PLATFORM=<host|macos|linux|windows|all>"
	@echo "  make desktop-status Show host/platform desktop packaging readiness for PLATFORM=<host|macos|linux|windows|all>"
	@echo "  make desktop-stage Stage the desktop/runtime release scaffold for PLATFORM=<host|macos|linux|windows|all>"
	@echo "  make desktop-build-host Build all three desktop shells for the current host platform"
	@echo "  make desktop-release Stage, build, and verify desktop release output for PLATFORM=<host|macos|linux|windows|all>"
	@echo "  make desktop-verify Verify staged manifests and icon inputs for PLATFORM=<host|macos|linux|windows|all>"
	@echo "  make desktop-linux-remote Build Linux desktop packages on kyuubiki-lab"
	@echo "  make desktop-linux-remote-install-deps Install Linux desktop apt dependencies on kyuubiki-lab with sudo -n"
	@echo "  make desktop-linux-remote-preflight Check kyuubiki-lab Linux desktop build prerequisites"
	@echo "  make operator-package-preflight Run read-only external operator package admission checks"
	@echo "  make sync-desktop-shared Refresh shared desktop UI helper files into each Tauri app"
	@echo "  make build-installation-docs Regenerate installation integrity HTML docs from the shared JSON contract"
	@echo "  make build-update-catalog Regenerate the unified update catalog JSON and HTML docs"
	@echo "  make check-doc-book Verify the centralized book and Hub mirrors for version/link/text drift"
	@echo "  make sync-doc-book-version Sync hand-maintained book entry pages to the current shipping version"
	@echo "  make check-toolchains Verify Docker, Mix, Node, Rust, and lab defaults against config/toolchains.json"
	@echo "  make check-elixir-self-host Verify Elixir/Mix/OTP and self-host orchestrator env contracts"
	@echo "  make check-language-packs Validate shipped Workbench/Hub language support packs"
	@echo "  make check-ui-automation-contract Verify product-owned Workbench automation selector contracts"
	@echo "  make check-version-line Verify release, package, docs, and language-pack version contracts"
	@echo "  make check-operator-reliability-rules Verify pure operator reliability rule helpers"
	@echo "  make check-operator-reliability-schemas Verify operator reliability config/schema version contracts"
	@echo "  make build-operator-qualification-readiness Write operator qualification readiness report to OUT=tmp/operator-qualification-readiness.json"
	@echo "  make capture-line-field-qualification-provenance Write line-field qualification provenance JSON to OUT=tmp/line-field-qualification-provenance.json"
	@echo "  make capture-line-field-qualification-release-evidence Run and retain line-field qualification evidence output under OUT=tmp/line-field-qualification-release-evidence.json"
	@echo "  make check-line-field-closed-form-baseline Verify line-field closed-form qualification baseline artifact"
	@echo "  make check-line-field-qualification-release-evidence Verify release evidence JSON from IN=tmp/line-field-qualification-release-evidence.json"
	@echo "  make check-operator-reliability Verify physics-coverage operator reliability evidence"
	@echo "  make audit-rust-lines Enforce the Rust source file line-count ceiling"
	@echo "  make audit-project-organization Enforce repository-wide source/docs line-count organization"
	@echo "  make audit-dependencies Run npm production and RustSec lockfile dependency audits"
	@echo "  make architecture-check Run the lightweight new-architecture organization and TaskIR contract check"
	@echo "  make hub-gui-dev         Run the Tauri Hub GUI in development mode"
	@echo "  make hub-gui-build       Build the Tauri Hub GUI bundles"
	@echo "  make installer-gui-dev   Run the Tauri installer GUI in development mode"
	@echo "  make installer-gui-build Build the Tauri installer GUI bundles"
	@echo "  make workbench-gui-dev   Run the Tauri desktop workbench shell in development mode"
	@echo "  make workbench-gui-build Build the Tauri desktop workbench shell bundles"
	@echo "  make test        Run all project tests"
	@echo "  make test-web    Run Elixir tests"
	@echo "  make test-rust   Run Rust workspace tests"
	@echo "  make test-frontend Run frontend typecheck and production build"
	@echo "  make workflow-preflight Run workflow topology plus layout/search guards (requires frontend dev server)"
	@echo "  make test-sdk    Run Python / Elixir / Rust SDK smoke tests"
	@echo "  make test-agent-capability-smoke Run solver-agent advertised-capability smoke against AGENT_HOST/AGENT_PORT"
	@echo "  make test-hub-gui Run Hub desktop shell smoke tests"
	@echo "  make test-installer-gui Run installer desktop shell smoke tests"
	@echo "  make test-workbench-gui Run desktop workbench shell smoke tests"
	@echo "  make test-integration Run the current cross-process integration smoke suite"
	@echo "  make test-integration-api Run the local orchestrator + agent + API integration smoke test"
	@echo "  make test-integration-cluster Run the protected cluster registration/heartbeat integration smoke test"
	@echo "  make test-integration-direct-mesh Run the direct_mesh_gui LAN agent solve + chunk smoke test"
	@echo "  make test-integration-desktop-gui Run Hub + Installer + Workbench desktop preview regression checks"
	@echo "  make test-integration-benchmark-profile-index Run benchmark profile index contract smoke tests"
	@echo "  make test-integration-direct-mesh-docker Run the direct_mesh_gui benchmark harness inside Docker and export repeat summaries"
	@echo "  make test-integration-remote-ssh-fixture Run the explicit local Docker sshd fixture probe"
	@echo "  make test-integration-workflow-mesh Run the distributed workflow mesh regression trio in sequence"
	@echo "  make test-integration-workflow-mesh-nightly Run the remote workflow mesh regression flow on kyuubiki-lab and pull logs back"
	@echo "  make test-integration-ui-mechanical Run the Playwright Workbench UI smoke for representative mechanical samples"
	@echo "  make test-integration-ui-thermal Run the Playwright Workbench UI smoke for representative thermal and thermo-mechanical samples"
	@echo "  make verify      Run formatting checks and tests"
	@echo "  make format      Format all code"
	@echo "  make smoke       Run the Elixir -> Rust smoke flow"
	@echo "  make worker      Run the Rust mock worker CLI"
	@echo "  make agent       Run the Rust FEM TCP agent"
	@echo "  make orchestrator Run the Elixir orchestrator API"
	@echo "  make playground  Legacy alias for the orchestrator API"
	@echo "  make frontend    Run the Next.js workbench UI"
	@echo "  make benchmark   Run the Rust solver benchmark suite"
	@echo "  make benchmark-physics-coverage Run the 1.15.x broad physics smoke matrix"
	@echo "  make benchmark-profile-remote Run one remote benchmark profile/matrix smoke (PROFILE=400k MATRIX=thermal-core CASE=heat-plane-quad-400k REPEAT=1)"
	@echo "  make benchmark-profile-report Regenerate a profile Markdown summary from an existing local JSON report"
	@echo "  make benchmark-profile-index Rebuild the retained exploratory profile run index under tmp/benchmark-profile"
	@echo "  make benchmark-baseline Write a benchmark baseline snapshot (PROFILE=10k by default; 100k/200k/300k/400k supported)"
	@echo "  make benchmark-compare Compare current benchmark output against a checked-in baseline (PROFILE=10k/15k/20k/100k/200k/300k/400k)"
	@echo "  make benchmark-report Write a Markdown comparison report against a checked-in baseline (PROFILE=10k/15k/20k/100k/200k/300k/400k)"
	@echo "  make benchmark-standard-baselines Write the 1.9 standard matrix baselines for PROFILE=<10k|15k|20k|100k|200k|300k|400k>"
	@echo "  make benchmark-standard-compare Run the 1.9 standard matrix regression gate trio for PROFILE=<10k|15k|20k|100k|200k|300k|400k>"
	@echo "  make benchmark-standard-report Write per-matrix reports plus a merged standard comparison report"
	@echo "  make benchmark-standard-nightly Run the remote kyuubiki-lab standard benchmark regression flow and pull reports back"
	@echo "  make regression-gate-report Rebuild the normalized regression catalog and compact gate report under tmp/"
	@echo "  make test-integration-direct-mesh-docker-compare Compare a Docker direct-mesh summary against the checked-in baseline"
	@echo "  make test-integration-direct-mesh-docker-report Run the Docker direct-mesh benchmark and write a baseline comparison report"
	@echo "  make test-integration-direct-mesh-docker-nightly Run the remote direct-mesh Docker regression flow against the checked-in baseline"
	@echo "  make test-integration-workflow-catalog-compare Compare a workflow catalog benchmark summary against the checked-in baseline"
	@echo "  make test-integration-workflow-catalog-report Run the workflow catalog benchmark and write a baseline comparison report"
	@echo "  make test-integration-workflow-catalog-nightly Run the remote workflow catalog regression flow against the checked-in baseline"
	@echo "  make tdd-web     Run a focused Elixir test by FILE=... or TEST=..."
	@echo "  make tdd-rust    Run focused Rust tests with FILTER=..."
	@echo "  ./scripts/kyuubiki help        Show the unified local entrypoint"
	@echo "  KYUUBIKI_STORAGE_BACKEND=postgres DATABASE_URL=ecto://postgres:postgres@127.0.0.1:5432/kyuubiki_dev make start"
	@echo "  cp .env.example .env.local && make restart"

tree:
	@find . -maxdepth 3 -type d | sort

build-frontend:
	@$(ENTRYPOINT) build-frontend

build-orchestrator:
	@$(ENTRYPOINT) build-orchestrator

build-agent:
	@$(ENTRYPOINT) build-agent

build-hub-gui:
	@$(ENTRYPOINT) build-hub-gui $(PLATFORM)

build-installer-gui:
	@$(ENTRYPOINT) build-installer-gui $(PLATFORM)

build-workbench-gui:
	@$(ENTRYPOINT) build-workbench-gui $(PLATFORM)

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

hot-local:
	@$(ENTRYPOINT) hot-local

hot-cloud:
	@$(ENTRYPOINT) hot-cloud

hot-distributed:
	@$(ENTRYPOINT) hot-distributed

hot-web:
	@$(ENTRYPOINT) hot-web $(MODE)

hot-agent:
	@$(ENTRYPOINT) hot-agent $(PORT)

hot-hub-gui:
	@$(ENTRYPOINT) hot-hub-gui

hot-installer-gui:
	@$(ENTRYPOINT) hot-installer-gui

hot-workbench-gui:
	@$(ENTRYPOINT) hot-workbench-gui

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

package-runtime:
	@$(ENTRYPOINT) package-runtime $(ARGS)

package-desktop:
	@$(ENTRYPOINT) package-desktop $(PLATFORM)

desktop-status:
	@$(ENTRYPOINT) desktop-status $(PLATFORM)

desktop-stage:
	@$(ENTRYPOINT) desktop-stage $(PLATFORM)

desktop-build-host:
	@$(ENTRYPOINT) desktop-build-host

desktop-release:
	@$(ENTRYPOINT) desktop-release $(PLATFORM)

desktop-verify:
	@$(ENTRYPOINT) desktop-verify $(PLATFORM)

desktop-linux-remote:
	@$(ENTRYPOINT) desktop-linux-remote

desktop-linux-remote-install-deps:
	@$(ENTRYPOINT) desktop-linux-remote install-deps

desktop-linux-remote-preflight:
	@$(ENTRYPOINT) desktop-linux-remote preflight

operator-package-preflight:
	@$(ENTRYPOINT) operator-package-preflight $(or $(PACKAGES_ROOT),$(CURDIR)/workers/rust/templates) $(if $(OUT),--out $(abspath $(OUT)),) $(if $(FAIL_ON_REJECTED),--fail-on-rejected,)

sync-desktop-shared:
	@node ./apps/desktop-shared/scripts/sync-desktop-shared.mjs

build-installation-docs:
	@node ./scripts/build-installation-integrity-docs.mjs

build-update-catalog:
	@node ./scripts/build-update-catalog.mjs

regression-gate-report:
	@node ./scripts/build-benchmark-profile-index.mjs
	@node ./scripts/build-regression-lane-catalog.mjs --tmp-root ./tmp
	@node ./scripts/build-regression-gate-report.mjs --tmp-root ./tmp
	@node ./scripts/build-nightly-artifact-overview.mjs --tmp-root ./tmp

check-doc-book:
	@node ./scripts/check-doc-book.mjs

sync-doc-book-version:
	@node ./scripts/sync-doc-book-version.mjs

check-toolchains:
	@node ./scripts/check-toolchain-contract.mjs

check-elixir-self-host:
	@node ./scripts/check-elixir-self-host.mjs

check-language-packs:
	@node ./scripts/validate-language-packs.mjs

check-ui-automation-contract:
	@node ./scripts/check-ui-automation-contract.mjs --self-test
	@node ./scripts/check-ui-automation-contract.mjs

check-version-line:
	@node ./scripts/audit-version-line.mjs

check-operator-reliability-rules:
	@node ./scripts/test-operator-reliability-rules.mjs

check-operator-reliability-schemas:
	@node ./scripts/check-operator-reliability-schemas.mjs --self-test
	@node ./scripts/check-operator-reliability-schemas.mjs

build-operator-qualification-readiness:
	@node ./scripts/build-operator-qualification-readiness.mjs --out $${OUT:-tmp/operator-qualification-readiness.json}

capture-line-field-qualification-provenance:
	@node ./scripts/capture-line-field-qualification-provenance.mjs --out $${OUT:-tmp/line-field-qualification-provenance.json}

capture-line-field-qualification-release-evidence:
	@node ./scripts/capture-line-field-qualification-release-evidence.mjs --out $${OUT:-tmp/line-field-qualification-release-evidence.json}

check-line-field-closed-form-baseline:
	@node ./scripts/check-line-field-closed-form-baseline.mjs

check-line-field-qualification-release-evidence:
	@node ./scripts/check-line-field-qualification-release-evidence.mjs --in $${IN:-tmp/line-field-qualification-release-evidence.json}

check-operator-reliability: check-operator-reliability-rules check-operator-reliability-schemas check-line-field-closed-form-baseline
	@node ./scripts/check-operator-reliability.mjs

audit-rust-lines:
	@node ./scripts/audit-rust-line-counts.mjs --max $${MAX_LINES:-600}

audit-project-organization:
	@node ./scripts/audit-project-organization.mjs --self-test
	@node ./scripts/audit-project-organization.mjs

audit-dependencies:
	@node ./scripts/audit-dependencies.mjs --self-test
	@node ./scripts/audit-dependencies.mjs

architecture-check:
	@$(MAKE) audit-project-organization
	@$(MAKE) check-version-line
	@$(MAKE) check-operator-reliability
	@$(MAKE) check-ui-automation-contract
	@$(MAKE) audit-dependencies
	@$(MAKE) operator-package-preflight
	@jq empty docs/book-manifest.json
	@cd apps/web && mix test test/kyuubiki_web/api/operator_task_api_test.exs test/kyuubiki_web/orchestra/operator_task_executor_test.exs test/kyuubiki_web/orchestra/operator_task_ir_test.exs
	@cd workers/rust && cargo test -p kyuubiki-cli operator_task_ir_rpc
	@cd workers/rust && cargo test -p kyuubiki-cli --test operator_task_live

hub-gui-dev:
	@$(ENTRYPOINT) hub-gui-dev

hub-gui-build:
	@$(ENTRYPOINT) hub-gui-build $(PLATFORM)

installer-gui-dev:
	@$(ENTRYPOINT) installer-gui-dev

installer-gui-build:
	@$(ENTRYPOINT) installer-gui-build $(PLATFORM)

workbench-gui-dev:
	@$(ENTRYPOINT) workbench-gui-dev

workbench-gui-build:
	@$(ENTRYPOINT) workbench-gui-build $(PLATFORM)

test: test-web test-rust test-frontend test-sdk test-playground

test-web:
	@cd apps/web && mix test

test-rust:
	@$(ENTRYPOINT) rust-test

test-frontend:
	@cd apps/frontend && npm run typecheck && npm run build

workflow-preflight:
	@cd apps/frontend && npm run check:workflow-preflight

test-sdk:
	@$(ENTRYPOINT) sdk-smoke

test-agent-capability-smoke:
	@$(ENTRYPOINT) agent-capability-smoke --host $${AGENT_HOST:-127.0.0.1} --port $${AGENT_PORT:-5001} --profile $${AGENT_SMOKE_PROFILE:-advertised} --output $${OUTPUT:-tmp/agent-capability-smoke.json} $${AGENT_SMOKE_ARGS:-}

test-playground:
	@node --test apps/web/playground/test/fem.test.mjs

test-hub-gui:
	@cd apps/hub-gui && npm run test:smoke

test-installer-gui:
	@cd apps/installer-gui && npm run test:smoke

test-workbench-gui:
	@cd apps/workbench-gui && npm run test:smoke

test-integration: test-integration-api test-integration-cluster test-integration-direct-mesh test-integration-desktop-gui test-integration-benchmark-profile-index test-integration-ui-mechanical test-integration-ui-thermal

test-integration-api:
	@node --test tests/integration/orchestrator-agent-api-smoke.test.mjs

test-integration-cluster:
	@node --test tests/integration/distributed-control-plane-smoke.test.mjs

test-integration-direct-mesh:
	@node --test tests/integration/direct-mesh-gui-smoke.test.mjs

test-integration-desktop-gui:
	@node --test tests/integration/desktop-shell-regression.test.mjs tests/integration/workbench-shell-regression.test.mjs

test-integration-benchmark-profile-index:
	@node --test tests/integration/benchmark-profile-index.test.mjs

test-integration-direct-mesh-docker:
	@DOCKER_RUN_NETWORK=$${DOCKER_RUN_NETWORK:-host} $(ENTRYPOINT) direct-mesh-benchmark-container --repeat $${REPEAT:-3} --output-dir $${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}

test-integration-remote-ssh-fixture:
	@bash ./scripts/run-remote-ssh-fixture.sh

test-integration-direct-mesh-docker-compare:
	@node ./scripts/compare-direct-mesh-benchmark.mjs --current $${CURRENT:-tmp/direct-mesh-benchmark-container/latest/summary.json} --baseline $${BASELINE:-tests/integration/benchmarks/direct-mesh-docker-baseline.json} --json-out $${COMPARE_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.json} --report-out $${REPORT_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.md} --fail-on-elapsed-regression-pct $${DIRECT_MESH_ELAPSED_THRESHOLD:-15} --fail-on-rss-regression-pct $${DIRECT_MESH_RSS_THRESHOLD:-20}

test-integration-direct-mesh-docker-report:
	@$(MAKE) test-integration-direct-mesh-docker REPEAT=$${REPEAT:-3} OUTPUT_DIR=$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}
	@$(MAKE) test-integration-direct-mesh-docker-compare CURRENT=$${CURRENT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/summary.json} COMPARE_OUT=$${COMPARE_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.json} REPORT_OUT=$${REPORT_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.md}

test-integration-direct-mesh-docker-nightly:
	@$(ENTRYPOINT) direct-mesh-benchmark-regression

test-integration-workflow-mesh:
	@bash ./scripts/run-workflow-mesh-regression.sh

test-integration-workflow-mesh-nightly:
	@$(ENTRYPOINT) workflow-mesh-regression-remote

test-integration-workflow-catalog-compare:
	@node ./scripts/compare-workflow-catalog-benchmark.mjs --current $${CURRENT:-tmp/workflow-catalog-benchmark.json} --baseline $${BASELINE:-tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json} --json-out $${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} --report-out $${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md} --fail-on-median-regression-pct $${WORKFLOW_MEDIAN_THRESHOLD:-50} --fail-on-avg-regression-pct $${WORKFLOW_AVG_THRESHOLD:-80}

test-integration-workflow-catalog-report:
	@cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs
	@$(MAKE) test-integration-workflow-catalog-compare CURRENT=$${CURRENT:-tmp/workflow-catalog-benchmark.json} COMPARE_OUT=$${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} REPORT_OUT=$${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md}

test-integration-workflow-catalog-nightly:
	@$(ENTRYPOINT) workflow-catalog-benchmark-regression

test-integration-ui-mechanical:
	@node --test tests/integration/workbench-ui-mechanical-smoke.test.mjs

test-integration-ui-thermal:
	@node --test tests/integration/workbench-ui-thermal-smoke.test.mjs

format: format-web format-rust

format-web:
	@cd apps/web && mix format

format-rust:
	@cd workers/rust && cargo fmt

verify:
	@$(MAKE) check-toolchains
	@$(MAKE) check-elixir-self-host
	@$(MAKE) check-language-packs
	@$(MAKE) check-version-line
	@$(MAKE) check-operator-reliability
	@$(MAKE) check-ui-automation-contract
	@cd apps/web && mix format --check-formatted && mix test
	@cd workers/rust && cargo fmt --check && cargo test
	@$(MAKE) audit-rust-lines
	@$(MAKE) audit-project-organization
	@$(MAKE) audit-dependencies
	@$(MAKE) operator-package-preflight
	@$(ENTRYPOINT) sdk-smoke
	@cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $${PROFILE:-10k} --repeat $${REPEAT:-3} --baseline-compare benchmarks/$${PROFILE:-10k}-baseline.json --fail-on-median-regression-pct $${BENCHMARK_MEDIAN_THRESHOLD:-25} --fail-on-rss-regression-pct $${BENCHMARK_RSS_THRESHOLD:-20} --min-baseline-median-ms $${BENCHMARK_MIN_BASELINE_MS:-5.0}
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

benchmark-physics-coverage:
	@cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $${PROFILE:-medium} --matrix physics-coverage --repeat $${REPEAT:-1}

benchmark-profile-remote:
	@$(ENTRYPOINT) benchmark-profile-remote

benchmark-profile-report:
	@REPORT_ONLY=1 $(ENTRYPOINT) benchmark-profile-remote

benchmark-profile-index:
	@node ./scripts/build-benchmark-profile-index.mjs

benchmark-baseline:
	@matrix=$${MATRIX:-core}; profile=$${PROFILE:-10k}; baseline=$$( [ "$$matrix" = "core" ] && printf 'benchmarks/%s-baseline.json' "$$profile" || printf 'benchmarks/%s-%s-baseline.json' "$$matrix" "$$profile" ); cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $$profile --matrix $$matrix --repeat $${REPEAT:-5} --baseline-out $$baseline

benchmark-compare:
	@matrix=$${MATRIX:-core}; profile=$${PROFILE:-10k}; baseline=$$( [ "$$matrix" = "core" ] && printf 'benchmarks/%s-baseline.json' "$$profile" || printf 'benchmarks/%s-%s-baseline.json' "$$matrix" "$$profile" ); cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $$profile --matrix $$matrix --repeat $${REPEAT:-3} --baseline-compare $$baseline --fail-on-median-regression-pct $${BENCHMARK_MEDIAN_THRESHOLD:-25} --fail-on-rss-regression-pct $${BENCHMARK_RSS_THRESHOLD:-20} --min-baseline-median-ms $${BENCHMARK_MIN_BASELINE_MS:-5.0}

benchmark-report:
	@mkdir -p workers/rust/benchmarks/reports
	@matrix=$${MATRIX:-core}; profile=$${PROFILE:-10k}; baseline=$$( [ "$$matrix" = "core" ] && printf 'benchmarks/%s-baseline.json' "$$profile" || printf 'benchmarks/%s-%s-baseline.json' "$$matrix" "$$profile" ); report=$$( [ "$$matrix" = "core" ] && printf 'benchmarks/reports/%s-compare.md' "$$profile" || printf 'benchmarks/reports/%s-%s-compare.md' "$$matrix" "$$profile" ); cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $$profile --matrix $$matrix --repeat $${REPEAT:-3} --baseline-compare $$baseline --compare-report-out $$report

benchmark-standard-baselines:
	@$(MAKE) benchmark-baseline PROFILE=$${PROFILE:-10k} MATRIX=mechanical-core REPEAT=$${REPEAT:-3}
	@$(MAKE) benchmark-baseline PROFILE=$${PROFILE:-10k} MATRIX=thermal-core REPEAT=$${REPEAT:-3}
	@$(MAKE) benchmark-baseline PROFILE=$${PROFILE:-10k} MATRIX=compound-core REPEAT=$${REPEAT:-3}

benchmark-standard-compare:
	@$(MAKE) benchmark-compare PROFILE=$${PROFILE:-10k} MATRIX=mechanical-core REPEAT=$${REPEAT:-3} BENCHMARK_MEDIAN_THRESHOLD=$${BENCHMARK_MEDIAN_THRESHOLD:-25} BENCHMARK_RSS_THRESHOLD=$${BENCHMARK_RSS_THRESHOLD:-20} BENCHMARK_MIN_BASELINE_MS=$${BENCHMARK_MIN_BASELINE_MS:-5.0}
	@$(MAKE) benchmark-compare PROFILE=$${PROFILE:-10k} MATRIX=thermal-core REPEAT=$${REPEAT:-3} BENCHMARK_MEDIAN_THRESHOLD=$${BENCHMARK_MEDIAN_THRESHOLD:-25} BENCHMARK_RSS_THRESHOLD=$${BENCHMARK_RSS_THRESHOLD:-20} BENCHMARK_MIN_BASELINE_MS=$${BENCHMARK_MIN_BASELINE_MS:-5.0}
	@$(MAKE) benchmark-compare PROFILE=$${PROFILE:-10k} MATRIX=compound-core REPEAT=$${REPEAT:-3} BENCHMARK_MEDIAN_THRESHOLD=$${BENCHMARK_MEDIAN_THRESHOLD:-25} BENCHMARK_RSS_THRESHOLD=$${BENCHMARK_RSS_THRESHOLD:-20} BENCHMARK_MIN_BASELINE_MS=$${BENCHMARK_MIN_BASELINE_MS:-5.0}

benchmark-standard-report:
	@mkdir -p workers/rust/benchmarks/reports
	@$(MAKE) benchmark-report PROFILE=$${PROFILE:-10k} MATRIX=mechanical-core REPEAT=$${REPEAT:-3}
	@$(MAKE) benchmark-report PROFILE=$${PROFILE:-10k} MATRIX=thermal-core REPEAT=$${REPEAT:-3}
	@$(MAKE) benchmark-report PROFILE=$${PROFILE:-10k} MATRIX=compound-core REPEAT=$${REPEAT:-3}
	@node ./scripts/build-standard-benchmark-report.mjs --profile $${PROFILE:-10k} --output $${OUTPUT:-workers/rust/benchmarks/reports/standard-$${PROFILE:-10k}-compare.md}

benchmark-standard-nightly:
	@$(ENTRYPOINT) standard-benchmark-regression
