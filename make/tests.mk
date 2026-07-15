.PHONY: test test-web test-rust test-frontend workflow-preflight test-runtime-surfaces test-sdk
.PHONY: test-agent-capability-smoke test-playground
.PHONY: test-hub-gui test-installer-gui test-workbench-gui
.PHONY: test-integration test-integration-api test-integration-cluster
.PHONY: test-integration-direct-mesh test-integration-desktop-gui
.PHONY: test-integration-benchmark-profile-index
.PHONY: test-integration-direct-mesh-docker test-integration-remote-ssh-fixture test-central-database-smoke remote-central-database-smoke
.PHONY: test-integration-direct-mesh-docker-compare
.PHONY: test-integration-direct-mesh-docker-report
.PHONY: test-integration-direct-mesh-docker-nightly
.PHONY: test-integration-workflow-mesh test-integration-workflow-mesh-nightly
.PHONY: test-integration-workflow-catalog-compare
.PHONY: test-integration-workflow-catalog-report
.PHONY: test-integration-workflow-catalog-nightly
.PHONY: test-integration-ui-mechanical test-integration-ui-thermal
.PHONY: format format-web format-rust tdd-web tdd-rust

test: test-web test-rust test-frontend test-sdk test-playground

test-web:
	@cd apps/web && mix test

test-rust:
	@$(ENTRYPOINT) rust-test

test-frontend:
	@cd apps/frontend && npm run typecheck && npm run build

workflow-preflight:
	@cd apps/frontend && npm run check:workflow-preflight

test-runtime-surfaces:
	@cd apps/frontend && npm run test:unit -- hub-runtime-surface installer-runtime-surface workbench-workflow-benchmark-surface
	@cd apps/web && mix test test/kyuubiki_web/orchestra/control_plane_surface_test.exs
	@cd workers/rust && cargo test -p kyuubiki-protocol protocol_benchmark_surface -- --nocapture

test-sdk:
	@$(ENTRYPOINT) sdk-smoke
	@$(MAKE) check-material-study-sdk-examples

test-agent-capability-smoke:
	@$(ENTRYPOINT) agent-capability-smoke --host $${AGENT_HOST:-127.0.0.1} --port $${AGENT_PORT:-5001} --profile $${AGENT_SMOKE_PROFILE:-advertised} --output $${OUTPUT:-tmp/agent-capability-smoke.json} $${AGENT_SMOKE_ARGS:-}

test-playground:
	@$(ENTRYPOINT) playground-fem-node-test

test-hub-gui:
	@cd apps/hub-gui && npm run test:smoke

test-installer-gui:
	@cd apps/installer-gui && npm run test:smoke

test-workbench-gui:
	@cd apps/workbench-gui && npm run test:smoke

test-integration: test-integration-api test-integration-cluster test-integration-direct-mesh test-integration-desktop-gui test-integration-benchmark-profile-index test-integration-ui-mechanical test-integration-ui-thermal

test-integration-api:
	@$(ENTRYPOINT) integration-api-node-test

test-integration-cluster:
	@$(ENTRYPOINT) integration-cluster-node-test

test-integration-direct-mesh:
	@$(ENTRYPOINT) integration-direct-mesh-node-test

test-integration-desktop-gui:
	@$(ENTRYPOINT) integration-desktop-gui-node-test

test-integration-benchmark-profile-index:
	@$(ENTRYPOINT) integration-benchmark-profile-index-node-test

test-integration-direct-mesh-docker:
	@if [ "$${LOCAL_DOCKER:-0}" = "1" ]; then \
		DOCKER_RUN_NETWORK=$${DOCKER_RUN_NETWORK:-host} $(ENTRYPOINT) direct-mesh-benchmark-container --repeat $${REPEAT:-3} --output-dir $${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}; \
	else \
		$(ENTRYPOINT) direct-mesh-benchmark-regression; \
	fi

test-integration-remote-ssh-fixture:
	@$(ENTRYPOINT) remote-ssh-fixture

test-central-database-smoke:
	@$(ENTRYPOINT) central-database-smoke --mode $${MODE:-cloud} --backend $${BACKEND:-postgres}

remote-central-database-smoke:
	@$(ENTRYPOINT) remote-central-database-smoke --host $${REMOTE:-kyuubiki-lab} --mode $${MODE:-cloud} --backend $${BACKEND:-postgres}

test-integration-direct-mesh-docker-compare:
	@$(ENTRYPOINT) compare-direct-mesh-benchmark --current $${CURRENT:-tmp/direct-mesh-benchmark-container/latest/summary.json} --baseline $${BASELINE:-tests/integration/benchmarks/direct-mesh-docker-baseline.json} --json-out $${COMPARE_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.json} --report-out $${REPORT_OUT:-tmp/direct-mesh-benchmark-container/latest/compare.md} --fail-on-elapsed-regression-pct $${DIRECT_MESH_ELAPSED_THRESHOLD:-15} --fail-on-rss-regression-pct $${DIRECT_MESH_RSS_THRESHOLD:-20}

test-integration-direct-mesh-docker-report:
	@if [ "$${LOCAL_DOCKER:-0}" = "1" ]; then \
		DOCKER_RUN_NETWORK=$${DOCKER_RUN_NETWORK:-host} $(ENTRYPOINT) direct-mesh-benchmark-container --repeat $${REPEAT:-3} --output-dir $${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}; \
		$(ENTRYPOINT) compare-direct-mesh-benchmark --current $${CURRENT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/summary.json} --baseline $${BASELINE:-tests/integration/benchmarks/direct-mesh-docker-baseline.json} --json-out $${COMPARE_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.json} --report-out $${REPORT_OUT:-$${OUTPUT_DIR:-tmp/direct-mesh-benchmark-container/latest}/compare.md} --fail-on-elapsed-regression-pct $${DIRECT_MESH_ELAPSED_THRESHOLD:-15} --fail-on-rss-regression-pct $${DIRECT_MESH_RSS_THRESHOLD:-20}; \
	else \
		$(ENTRYPOINT) direct-mesh-benchmark-regression; \
	fi

test-integration-direct-mesh-docker-nightly:
	@$(ENTRYPOINT) direct-mesh-benchmark-regression

test-integration-workflow-mesh:
	@$(ENTRYPOINT) workflow-mesh-regression

test-integration-workflow-mesh-nightly:
	@$(ENTRYPOINT) workflow-mesh-regression-remote

test-integration-workflow-catalog-compare:
	@$(ENTRYPOINT) compare-workflow-catalog-benchmark --current $${CURRENT:-tmp/workflow-catalog-benchmark.json} --baseline $${BASELINE:-tests/integration/benchmarks/workflow-catalog-benchmark-baseline.json} --json-out $${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} --report-out $${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md} --fail-on-median-regression-pct $${WORKFLOW_MEDIAN_THRESHOLD:-50} --fail-on-avg-regression-pct $${WORKFLOW_AVG_THRESHOLD:-80}

test-integration-workflow-catalog-report:
	@cd apps/web && mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs
	@$(MAKE) test-integration-workflow-catalog-compare CURRENT=$${CURRENT:-tmp/workflow-catalog-benchmark.json} COMPARE_OUT=$${COMPARE_OUT:-tmp/workflow-catalog-benchmark.compare.json} REPORT_OUT=$${REPORT_OUT:-tmp/workflow-catalog-benchmark.compare.md}

test-integration-workflow-catalog-nightly:
	@$(ENTRYPOINT) workflow-catalog-benchmark-regression

test-integration-ui-mechanical:
	@$(ENTRYPOINT) integration-ui-mechanical-node-test

test-integration-ui-thermal:
	@$(ENTRYPOINT) integration-ui-thermal-node-test

format: format-web format-rust

format-web:
	@cd apps/web && mix format

format-rust:
	@cd workers/rust && cargo fmt

tdd-web:
	@cd apps/web && mix test $(FILE) $(TEST)

tdd-rust:
	@cd workers/rust && cargo test $(FILTER)
