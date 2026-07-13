.PHONY: benchmark benchmark-physics-coverage benchmark-profile-remote benchmark-profile-plan
.PHONY: benchmark-profile-report benchmark-profile-index
.PHONY: benchmark-baseline benchmark-compare benchmark-report
.PHONY: benchmark-standard-baselines benchmark-standard-compare
.PHONY: benchmark-standard-report benchmark-standard-nightly regression-gate-report

regression-gate-report:
	@node ./scripts/build-module-topology-report.mjs --out-dir ./tmp/module-topology
	@node ./scripts/build-benchmark-profile-index.mjs
	@node ./scripts/build-regression-lane-catalog.mjs --tmp-root ./tmp
	@node ./scripts/build-regression-gate-report.mjs --tmp-root ./tmp
	@node ./scripts/build-nightly-artifact-overview.mjs --tmp-root ./tmp

benchmark:
	@$(ENTRYPOINT) benchmark $(ARGS)

benchmark-physics-coverage:
	@case_arg=$$( [ -n "$${CASE:-}" ] && printf -- ' --case %s' "$$CASE" || true ); cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $${PROFILE:-medium} --matrix physics-coverage --repeat $${REPEAT:-1} $$case_arg

benchmark-profile-remote:
	@$(ENTRYPOINT) benchmark-profile-remote

benchmark-profile-plan:
	@$(ENTRYPOINT) benchmark-profile-plan

benchmark-profile-report:
	@REPORT_ONLY=1 $(ENTRYPOINT) benchmark-profile-remote

benchmark-profile-index:
	@node ./scripts/build-benchmark-profile-index.mjs

benchmark-baseline:
	@matrix=$${MATRIX:-core}; profile=$${PROFILE:-10k}; case_arg=$$( [ -n "$${CASE:-}" ] && printf -- ' --case %s' "$$CASE" || true ); case_slug=$$( [ -n "$${CASE:-}" ] && printf '%s' "$$CASE" | tr -c '[:alnum:]_.-' '-' || true ); baseline=$$( if [ "$$matrix" = "core" ]; then [ -n "$$case_slug" ] && printf 'benchmarks/%s-%s-baseline.json' "$$profile" "$$case_slug" || printf 'benchmarks/%s-baseline.json' "$$profile"; else [ -n "$$case_slug" ] && printf 'benchmarks/%s-%s-%s-baseline.json' "$$matrix" "$$profile" "$$case_slug" || printf 'benchmarks/%s-%s-baseline.json' "$$matrix" "$$profile"; fi ); cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $$profile --matrix $$matrix --repeat $${REPEAT:-5} --baseline-out $$baseline $$case_arg

benchmark-compare:
	@matrix=$${MATRIX:-core}; profile=$${PROFILE:-10k}; case_arg=$$( [ -n "$${CASE:-}" ] && printf -- ' --case %s' "$$CASE" || true ); baseline=$$( [ "$$matrix" = "core" ] && printf 'benchmarks/%s-baseline.json' "$$profile" || printf 'benchmarks/%s-%s-baseline.json' "$$matrix" "$$profile" ); cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $$profile --matrix $$matrix --repeat $${REPEAT:-3} --baseline-compare $$baseline --fail-on-median-regression-pct $${BENCHMARK_MEDIAN_THRESHOLD:-25} --fail-on-rss-regression-pct $${BENCHMARK_RSS_THRESHOLD:-20} --min-baseline-median-ms $${BENCHMARK_MIN_BASELINE_MS:-5.0} $$case_arg

benchmark-report:
	@mkdir -p workers/rust/benchmarks/reports
	@matrix=$${MATRIX:-core}; profile=$${PROFILE:-10k}; case_arg=$$( [ -n "$${CASE:-}" ] && printf -- ' --case %s' "$$CASE" || true ); case_slug=$$( [ -n "$${CASE:-}" ] && printf '%s' "$$CASE" | tr -c '[:alnum:]_.-' '-' || true ); baseline=$$( [ "$$matrix" = "core" ] && printf 'benchmarks/%s-baseline.json' "$$profile" || printf 'benchmarks/%s-%s-baseline.json' "$$matrix" "$$profile" ); report=$$( if [ "$$matrix" = "core" ]; then [ -n "$$case_slug" ] && printf 'benchmarks/reports/%s-%s-compare.md' "$$profile" "$$case_slug" || printf 'benchmarks/reports/%s-compare.md' "$$profile"; else [ -n "$$case_slug" ] && printf 'benchmarks/reports/%s-%s-%s-compare.md' "$$matrix" "$$profile" "$$case_slug" || printf 'benchmarks/reports/%s-%s-compare.md' "$$matrix" "$$profile"; fi ); cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $$profile --matrix $$matrix --repeat $${REPEAT:-3} --baseline-compare $$baseline --compare-report-out $$report $$case_arg

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
