.PHONY: check-doc-book check-doc-inventory sync-doc-book-version check-toolchains check-elixir-self-host check-commercial-readiness check-moxi-handoff check-install-update-disk-hygiene check-component-integrity-protocol
.PHONY: check-make-modules check-module-topology check-module-function-matrix check-module-function-coverage-tensor check-module-extension-standard check-contracts-runtime-api-surface check-verification-evidence-surface check-central-store-contract check-central-database-readiness build-central-readiness-report check-central-readiness-report build-module-topology-report check-native-script-audit
.PHONY: check-language-packs report-full-language-pack-coverage check-full-language-pack-coverage check-language-pack-coverage export-language-pack-translation-batch draft-language-pack-translation-batch apply-language-pack-translation-batch check-ui-automation-contract check-gui-runtime-capability-contract check-version-line
.PHONY: check-workflow-dataset-contract check-material-card-contract check-material-score-contract check-materialization-plan-contract check-material-study-execution-plan-contract check-material-exploration-chain-contract check-material-research-bundle-contract check-material-study-sdk-examples check-operator-task-ir-contract check-operator-package-dynamic-smoke-contract
.PHONY: build-operator-qualification-readiness check-operator-qualification-readiness
.PHONY: check-operator-qualification-release-records check-operator-qualification-review-decision
.PHONY: capture-line-field-qualification-provenance capture-line-field-qualification-release-evidence capture-beam-frame-qualification-release-evidence
.PHONY: capture-thermal-plane-qualification-release-evidence capture-electromagnetic-plane-qualification-release-evidence capture-modal-frame-qualification-release-evidence capture-stokes-flow-screening-release-evidence capture-acoustic-bar-qualification-release-evidence capture-advection-diffusion-bar-qualification-release-evidence capture-magnetostatic-bar-qualification-release-evidence capture-spring-1d-qualification-release-evidence capture-spring-vector-qualification-release-evidence capture-nonlinear-spring-1d-qualification-release-evidence capture-frame-3d-qualification-release-evidence capture-plane-2d-qualification-release-evidence capture-thermal-beam-1d-qualification-release-evidence capture-thermal-frame-2d-qualification-release-evidence capture-thermal-frame-3d-qualification-release-evidence capture-solid-tetra-3d-qualification-release-evidence capture-contact-gap-1d-qualification-release-evidence capture-truss-2d-qualification-release-evidence capture-truss-3d-qualification-release-evidence capture-thermal-truss-3d-qualification-release-evidence capture-thermal-truss-2d-qualification-release-evidence
.PHONY: check-line-field-closed-form-baseline check-line-field-qualification-release-evidence check-beam-frame-qualification-release-evidence
.PHONY: check-thermal-plane-qualification-release-evidence check-electromagnetic-plane-qualification-release-evidence check-modal-frame-qualification-release-evidence check-stokes-flow-screening-release-evidence check-acoustic-bar-qualification-release-evidence check-advection-diffusion-bar-qualification-release-evidence check-magnetostatic-bar-qualification-release-evidence check-spring-1d-qualification-release-evidence check-spring-vector-qualification-release-evidence check-nonlinear-spring-1d-qualification-release-evidence check-frame-3d-qualification-release-evidence check-plane-2d-qualification-release-evidence check-thermal-beam-1d-qualification-release-evidence check-thermal-frame-2d-qualification-release-evidence check-thermal-frame-3d-qualification-release-evidence check-solid-tetra-3d-qualification-release-evidence check-contact-gap-1d-qualification-release-evidence check-truss-2d-qualification-release-evidence check-truss-3d-qualification-release-evidence check-thermal-truss-3d-qualification-release-evidence check-thermal-truss-2d-qualification-release-evidence
.PHONY: check-operator-reliability-rules check-operator-reliability-schemas check-operator-validation verify-operator-validation
.PHONY: capture-material-research-example check-material-research-example verify-material-research-example
.PHONY: build-material-research-bundle check-material-research-bundle verify-material-research-bundle material-research-bundle-index check-material-research-bundle-index check-material-research-bundle-index-contract
.PHONY: remote-material-research-example remote-material-research-summary
.PHONY: check-operator-reliability audit-rust-lines audit-project-organization
.PHONY: audit-dependencies fuzz-smoke check-minimal-industrial-closure architecture-check verify

check-doc-book:
	@$(ENTRYPOINT) check-doc-book

check-doc-inventory:
	@node ./scripts/check-doc-inventory.mjs

sync-doc-book-version:
	@$(ENTRYPOINT) sync-doc-book-version

check-toolchains:
	@$(ENTRYPOINT) check-toolchain-contract

check-elixir-self-host:
	@$(ENTRYPOINT) check-elixir-self-host

check-commercial-readiness:
	@$(ENTRYPOINT) validate-commercial-readiness

check-moxi-handoff:
	@node ./scripts/check-moxi-handoff.mjs

check-install-update-disk-hygiene:
	@$(ENTRYPOINT) check-install-update-disk-hygiene --self-test
	@$(ENTRYPOINT) check-install-update-disk-hygiene

check-component-integrity-protocol:
	@$(ENTRYPOINT) check-component-integrity-protocol --self-test
	@$(ENTRYPOINT) check-component-integrity-protocol --out $${OUT:-tmp/component-integrity-report.json}

check-make-modules:
	@./scripts/kyuubiki check-make-modules

check-module-topology:
	@$(ENTRYPOINT) check-module-topology --self-test
	@$(ENTRYPOINT) check-module-topology

check-module-function-matrix:
	@$(ENTRYPOINT) check-module-function-matrix --self-test
	@$(ENTRYPOINT) check-module-function-matrix

check-module-function-coverage-tensor:
	@$(ENTRYPOINT) check-module-function-coverage-tensor --self-test
	@$(ENTRYPOINT) check-module-function-coverage-tensor

check-module-extension-standard:
	@$(ENTRYPOINT) check-module-extension-standard --self-test
	@$(ENTRYPOINT) check-module-extension-standard

check-contracts-runtime-api-surface:
	@$(ENTRYPOINT) check-contracts-runtime-api-surface --self-test
	@$(ENTRYPOINT) check-contracts-runtime-api-surface

check-verification-evidence-surface:
	@$(ENTRYPOINT) check-verification-evidence-surface

check-central-store-contract:
	@$(ENTRYPOINT) check-central-store-contract --self-test
	@$(ENTRYPOINT) check-central-store-contract

check-central-database-readiness:
	@$(ENTRYPOINT) check-central-database-readiness --self-test
	@$(ENTRYPOINT) check-central-database-readiness --mode $${MODE:-local} --backend $${BACKEND:-sqlite}

build-central-readiness-report:
	@$(ENTRYPOINT) build-central-readiness-report --self-test
	@$(ENTRYPOINT) build-central-readiness-report --mode $${MODE:-local} --backend $${BACKEND:-sqlite} --out $${OUT:-tmp/central-readiness-report.json} --markdown-out $${MARKDOWN_OUT:-tmp/central-readiness-report.md}
	@$(ENTRYPOINT) check-central-readiness-report --self-test
	@$(ENTRYPOINT) check-central-readiness-report --in $${OUT:-tmp/central-readiness-report.json} --markdown-in $${MARKDOWN_OUT:-tmp/central-readiness-report.md}

check-central-readiness-report:
	@$(ENTRYPOINT) check-central-readiness-report --self-test
	@$(ENTRYPOINT) check-central-readiness-report --in $${IN:-tmp/central-readiness-report.json} --markdown-in $${MARKDOWN_IN:-tmp/central-readiness-report.md}

build-module-topology-report:
	@$(ENTRYPOINT) build-module-topology-report --out-dir $${OUT_DIR:-tmp/module-topology}

check-native-script-audit:
	@$(ENTRYPOINT) native-script-audit --self-test
	@$(ENTRYPOINT) native-script-audit

check-language-packs:
	@$(ENTRYPOINT) validate-language-packs

report-full-language-pack-coverage:
	@node ./scripts/report-full-language-pack-coverage.mjs

check-full-language-pack-coverage:
	@node ./scripts/report-full-language-pack-coverage.mjs --strict

check-language-pack-coverage:
	@test -n "$(LANGUAGE)" || (echo "LANGUAGE is required"; exit 2)
	@node ./scripts/report-full-language-pack-coverage.mjs --strict-language "$(LANGUAGE)"

export-language-pack-translation-batch:
	@test -n "$(LANGUAGE)" && test -n "$(BATCH)" || (echo "LANGUAGE and BATCH are required"; exit 2)
	@node ./scripts/report-full-language-pack-coverage.mjs --language "$(LANGUAGE)" --batch "$(BATCH)" --template-out "$${OUT:-tmp/language-pack-translation-batches/$(LANGUAGE)-$(BATCH).json}"

draft-language-pack-translation-batch:
	@test -n "$(LANGUAGE)" && test -n "$(BATCH)" || (echo "LANGUAGE and BATCH are required"; exit 2)
	@$(MAKE) export-language-pack-translation-batch LANGUAGE="$(LANGUAGE)" BATCH="$(BATCH)" OUT="$${IN:-tmp/language-pack-translation-batches/$(LANGUAGE)-$(BATCH).json}"
	@node ./scripts/draft-language-pack-machine-translations.mjs --in "$${IN:-tmp/language-pack-translation-batches/$(LANGUAGE)-$(BATCH).json}" --out "$${OUT:-tmp/language-pack-translation-drafts/$(LANGUAGE)-$(BATCH).json}" --target "$(LANGUAGE)"

apply-language-pack-translation-batch:
	@test -n "$(INPUT)" || (echo "INPUT is required"; exit 2)
	@node ./scripts/report-full-language-pack-coverage.mjs --apply-from "$(INPUT)"

check-ui-automation-contract:
	@$(ENTRYPOINT) check-ui-automation-contract --self-test
	@$(ENTRYPOINT) check-ui-automation-contract

check-gui-runtime-capability-contract:
	@$(ENTRYPOINT) check-gui-runtime-capability-contract --self-test
	@$(ENTRYPOINT) check-gui-runtime-capability-contract

check-version-line:
	@$(ENTRYPOINT) create-release-snapshot --self-test
	@$(ENTRYPOINT) audit-version-line --self-test
	@$(ENTRYPOINT) audit-version-line

check-workflow-dataset-contract:
	@$(ENTRYPOINT) check-workflow-dataset-contract --self-test
	@$(ENTRYPOINT) check-workflow-dataset-contract

check-material-card-contract:
	@$(ENTRYPOINT) check-material-card-contract --self-test
	@$(ENTRYPOINT) check-material-card-contract

check-material-score-contract:
	@$(ENTRYPOINT) validate-material-score-contract

check-materialization-plan-contract:
	@$(ENTRYPOINT) check-materialization-plan-contract --self-test
	@$(ENTRYPOINT) check-materialization-plan-contract

check-material-study-execution-plan-contract:
	@$(ENTRYPOINT) check-material-study-execution-plan-contract --self-test
	@$(ENTRYPOINT) check-material-study-execution-plan-contract

check-material-exploration-chain-contract:
	@$(ENTRYPOINT) check-material-exploration-chain-contract --self-test
	@$(ENTRYPOINT) check-material-exploration-chain-contract

check-material-research-bundle-contract:
	@$(ENTRYPOINT) check-material-research-bundle-contract --self-test
	@$(ENTRYPOINT) check-material-research-bundle-contract

check-material-study-sdk-examples:
	@$(ENTRYPOINT) check-material-study-sdk-examples --self-test
	@$(ENTRYPOINT) check-material-study-sdk-examples

check-operator-task-ir-contract:
	@$(ENTRYPOINT) check-operator-task-ir-contract --self-test
	@$(ENTRYPOINT) check-operator-task-ir-contract

check-operator-package-dynamic-smoke-contract:
	@$(ENTRYPOINT) check-operator-package-dynamic-smoke-contract --self-test

check-operator-reliability-rules:
	@$(ENTRYPOINT) check-operator-reliability-rules

check-operator-reliability-schemas:
	@$(ENTRYPOINT) check-operator-reliability-schemas --self-test
	@$(ENTRYPOINT) check-operator-reliability-schemas

check-operator-validation:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation

verify-operator-validation:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute

build-operator-qualification-readiness:
	@$(ENTRYPOINT) check-operator-qualification-readiness --self-test
	@$(ENTRYPOINT) build-operator-qualification-readiness --out $${OUT:-tmp/operator-qualification-readiness.json}
	@$(ENTRYPOINT) check-operator-qualification-readiness --in $${OUT:-tmp/operator-qualification-readiness.json}

check-operator-qualification-readiness:
	@$(ENTRYPOINT) check-operator-qualification-readiness --in $${IN:-tmp/operator-qualification-readiness.json}

check-operator-qualification-release-records:
	@$(ENTRYPOINT) check-operator-qualification-release-records --in $${IN:-releases/qualification-records/1.20.0.json}

check-operator-qualification-review-decision:
	@if [ -n "$$IN" ]; then \
		node ./scripts/check-operator-qualification-review-decision.mjs --in "$$IN"; \
	else \
		node ./scripts/check-operator-qualification-review-decision.mjs --all; \
	fi

capture-line-field-qualification-provenance:
	@$(ENTRYPOINT) capture-line-field-qualification-provenance --out $${OUT:-tmp/line-field-qualification-provenance.json}

capture-line-field-qualification-release-evidence:
	@$(ENTRYPOINT) capture-line-field-qualification-release-evidence --out $${OUT:-tmp/line-field-qualification-release-evidence.json}

capture-beam-frame-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile beam-frame-classic --out $${OUT:-tmp/beam-frame-classic-qualification-release-evidence.json}

capture-thermal-plane-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile thermal-plane-patch --out $${OUT:-tmp/thermal-plane-patch-qualification-release-evidence.json}

capture-electromagnetic-plane-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile electromagnetic-plane-patch --out $${OUT:-tmp/electromagnetic-plane-patch-qualification-release-evidence.json}

capture-modal-frame-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile modal-frame-sanity --out $${OUT:-tmp/modal-frame-sanity-qualification-release-evidence.json}

capture-stokes-flow-screening-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile screening-cfd-boundary --out $${OUT:-tmp/stokes-flow-screening-release-evidence.json}

capture-acoustic-bar-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile acoustic-bar-closed-form --out $${OUT:-tmp/acoustic-bar-closed-form-release-evidence.json}

capture-advection-diffusion-bar-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile advection-diffusion-bar-closed-form --out $${OUT:-tmp/advection-diffusion-bar-closed-form-release-evidence.json}

capture-magnetostatic-bar-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile magnetostatic-bar-closed-form --out $${OUT:-tmp/magnetostatic-bar-closed-form-release-evidence.json}

capture-spring-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile spring-1d-closed-form --out $${OUT:-tmp/spring-1d-closed-form-release-evidence.json}

capture-spring-vector-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile spring-vector-closed-form --out $${OUT:-tmp/spring-vector-closed-form-release-evidence.json}

capture-nonlinear-spring-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile nonlinear-spring-1d-closed-form --out $${OUT:-tmp/nonlinear-spring-1d-closed-form-release-evidence.json}

capture-frame-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile frame-3d-closed-form --out $${OUT:-tmp/frame-3d-closed-form-release-evidence.json}

capture-plane-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile plane-2d-patch-closed-form --out $${OUT:-tmp/plane-2d-patch-closed-form-release-evidence.json}

capture-thermal-beam-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile thermal-beam-1d-closed-form --out $${OUT:-tmp/thermal-beam-1d-closed-form-release-evidence.json}

capture-thermal-frame-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile thermal-frame-2d-closed-form --out $${OUT:-tmp/thermal-frame-2d-closed-form-release-evidence.json}

capture-thermal-frame-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile thermal-frame-3d-closed-form --out $${OUT:-tmp/thermal-frame-3d-closed-form-release-evidence.json}

capture-solid-tetra-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile solid-tetra-3d-closed-form --out $${OUT:-tmp/solid-tetra-3d-closed-form-release-evidence.json}

capture-contact-gap-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile contact-gap-1d-closed-form --out $${OUT:-tmp/contact-gap-1d-closed-form-release-evidence.json}

capture-truss-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile truss-2d-closed-form --out $${OUT:-tmp/truss-2d-closed-form-release-evidence.json}

capture-truss-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile truss-3d-closed-form --out $${OUT:-tmp/truss-3d-closed-form-release-evidence.json}

capture-thermal-truss-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile thermal-truss-3d-closed-form --out $${OUT:-tmp/thermal-truss-3d-closed-form-release-evidence.json}

capture-thermal-truss-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --self-test
	@$(ENTRYPOINT) check-operator-validation --execute --profile thermal-truss-2d-closed-form --out $${OUT:-tmp/thermal-truss-2d-closed-form-release-evidence.json}

check-beam-frame-qualification-release-evidence:
	@$(ENTRYPOINT) check-beam-frame-qualification-release-evidence --in $${IN:-tmp/beam-frame-classic-qualification-release-evidence.json}

check-thermal-plane-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/thermal-plane-patch-qualification-release-evidence.json} --profile thermal-plane-patch

check-electromagnetic-plane-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/electromagnetic-plane-patch-qualification-release-evidence.json} --profile electromagnetic-plane-patch

check-modal-frame-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/modal-frame-sanity-qualification-release-evidence.json} --profile modal-frame-sanity

check-stokes-flow-screening-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/stokes-flow-screening-release-evidence.json} --profile screening-cfd-boundary

check-acoustic-bar-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/acoustic-bar-closed-form-release-evidence.json} --profile acoustic-bar-closed-form

check-advection-diffusion-bar-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/advection-diffusion-bar-closed-form-release-evidence.json} --profile advection-diffusion-bar-closed-form

check-magnetostatic-bar-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/magnetostatic-bar-closed-form-release-evidence.json} --profile magnetostatic-bar-closed-form

check-spring-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/spring-1d-closed-form-release-evidence.json} --profile spring-1d-closed-form

check-spring-vector-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/spring-vector-closed-form-release-evidence.json} --profile spring-vector-closed-form

check-nonlinear-spring-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/nonlinear-spring-1d-closed-form-release-evidence.json} --profile nonlinear-spring-1d-closed-form

check-frame-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/frame-3d-closed-form-release-evidence.json} --profile frame-3d-closed-form

check-plane-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/plane-2d-patch-closed-form-release-evidence.json} --profile plane-2d-patch-closed-form

check-thermal-beam-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/thermal-beam-1d-closed-form-release-evidence.json} --profile thermal-beam-1d-closed-form

check-thermal-frame-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/thermal-frame-2d-closed-form-release-evidence.json} --profile thermal-frame-2d-closed-form

check-thermal-frame-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/thermal-frame-3d-closed-form-release-evidence.json} --profile thermal-frame-3d-closed-form

check-solid-tetra-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/solid-tetra-3d-closed-form-release-evidence.json} --profile solid-tetra-3d-closed-form

check-contact-gap-1d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/contact-gap-1d-closed-form-release-evidence.json} --profile contact-gap-1d-closed-form

check-truss-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/truss-2d-closed-form-release-evidence.json} --profile truss-2d-closed-form

check-truss-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/truss-3d-closed-form-release-evidence.json} --profile truss-3d-closed-form

check-thermal-truss-3d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/thermal-truss-3d-closed-form-release-evidence.json} --profile thermal-truss-3d-closed-form

check-thermal-truss-2d-qualification-release-evidence:
	@$(ENTRYPOINT) check-operator-validation --in $${IN:-tmp/thermal-truss-2d-closed-form-release-evidence.json} --profile thermal-truss-2d-closed-form

check-line-field-closed-form-baseline:
	@$(ENTRYPOINT) check-line-field-closed-form-baseline

check-line-field-qualification-release-evidence:
	@$(ENTRYPOINT) check-line-field-qualification-release-evidence --in $${IN:-tmp/line-field-qualification-release-evidence.json}

capture-material-research-example:
	@$(ENTRYPOINT) capture-material-research-example --out $${OUT:-tmp/material-research-example.json}

check-material-research-example:
	@$(ENTRYPOINT) check-material-research-example --in $${IN:-tmp/material-research-example.json}

verify-material-research-example:
	@$(MAKE) capture-material-research-example OUT=$${OUT:-tmp/material-research-example.json}
	@$(MAKE) check-material-research-example IN=$${OUT:-tmp/material-research-example.json}

build-material-research-bundle:
	@$(ENTRYPOINT) build-material-research-bundle --study $${STUDY:-heat-spreader} --out $${OUT:-tmp/material-research-bundle.json}

check-material-research-bundle:
	@$(ENTRYPOINT) check-material-research-bundle --self-test
	@$(ENTRYPOINT) check-material-research-bundle --in $${IN:-tmp/material-research-bundle.json}

verify-material-research-bundle:
	@$(MAKE) check-material-card-contract
	@$(MAKE) build-material-research-bundle OUT=$${OUT:-tmp/material-research-bundle.json}
	@$(MAKE) check-material-research-bundle IN=$${OUT:-tmp/material-research-bundle.json}

material-research-bundle-index:
	@$(ENTRYPOINT) check-material-research-bundle-index-contract --self-test
	@$(ENTRYPOINT) check-material-research-bundle-index-contract
	@$(ENTRYPOINT) build-material-research-bundle-index --self-test
	@$(ENTRYPOINT) build-material-research-bundle-index --ensure-bundles --out-dir $${OUT_DIR:-tmp/material-research-bundles}
	@$(ENTRYPOINT) check-material-research-bundle-index --self-test
	@$(ENTRYPOINT) check-material-research-bundle-index --in $${OUT_DIR:-tmp/material-research-bundles}/index.json

check-material-research-bundle-index:
	@$(ENTRYPOINT) check-material-research-bundle-index --self-test
	@$(ENTRYPOINT) check-material-research-bundle-index --in $${IN:-tmp/material-research-bundles/index.json}

check-material-research-bundle-index-contract:
	@$(ENTRYPOINT) check-material-research-bundle-index-contract --self-test
	@$(ENTRYPOINT) check-material-research-bundle-index-contract

remote-material-research-example:
	@$(ENTRYPOINT) remote-material-research-example --profile $${PROFILE:-100k} --matrix $${MATRIX:-compound-core} --repeat $${REPEAT:-1}

remote-material-research-summary:
	@$(ENTRYPOINT) build-remote-material-benchmark-summary --self-test
	@$(ENTRYPOINT) build-remote-material-benchmark-summary
	@$(ENTRYPOINT) check-remote-material-preconditioner-health --self-test
	@$(ENTRYPOINT) check-remote-material-preconditioner-health
	@$(ENTRYPOINT) check-remote-material-stage-health --self-test
	@$(ENTRYPOINT) check-remote-material-stage-health

check-operator-reliability: check-operator-reliability-rules check-operator-reliability-schemas check-line-field-closed-form-baseline build-operator-qualification-readiness check-operator-qualification-review-decision check-operator-qualification-release-records
	@$(ENTRYPOINT) check-operator-reliability

audit-rust-lines:
	@$(ENTRYPOINT) rust-line-audit --max $${MAX_LINES:-800}

audit-project-organization:
	@$(ENTRYPOINT) audit-project-organization --self-test
	@$(ENTRYPOINT) audit-project-organization

audit-dependencies:
	@$(ENTRYPOINT) audit-dependencies --self-test
	@$(ENTRYPOINT) audit-dependencies

check-minimal-industrial-closure:
	@$(ENTRYPOINT) validate-minimal-industrial-closure

fuzz-smoke:
	@cd workers/rust && cargo test -p kyuubiki-engine workflow_security_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-protocol operator_task_ir_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-protocol rpc_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-operator-sdk operator_package_manifest_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer installer_update_catalog_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_artifact_manifest_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer credential_storage_contract_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_host_trust_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_ssh_fixture_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-installer remote_deployment_metadata_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-script-runner central_store_contract_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-script-runner central_readiness_report_fuzz_smoke -- --nocapture
	@cd workers/rust && cargo test -p kyuubiki-script-runner language_pack_fuzz_smoke -- --nocapture

architecture-check:
	@$(MAKE) audit-project-organization
	@$(MAKE) check-make-modules
	@$(MAKE) check-module-topology
	@$(MAKE) check-module-function-matrix
	@$(MAKE) check-module-function-coverage-tensor
	@$(MAKE) check-module-extension-standard
	@$(MAKE) check-contracts-runtime-api-surface
	@$(MAKE) check-verification-evidence-surface
	@$(MAKE) check-central-store-contract
	@$(MAKE) check-central-database-readiness
	@$(MAKE) build-central-readiness-report
	@$(MAKE) test-runtime-surfaces
	@$(MAKE) check-native-script-audit
	@$(MAKE) check-version-line
	@$(MAKE) check-workflow-dataset-contract
	@$(MAKE) check-material-score-contract
	@$(MAKE) check-materialization-plan-contract
	@$(MAKE) check-material-study-execution-plan-contract
	@$(MAKE) check-material-exploration-chain-contract
	@$(MAKE) check-material-research-bundle-contract
	@$(MAKE) check-material-research-bundle-index-contract
	@$(MAKE) check-material-study-sdk-examples
	@$(MAKE) check-operator-task-ir-contract
	@$(MAKE) check-operator-package-dynamic-smoke-contract
	@$(MAKE) check-operator-reliability
	@$(MAKE) check-operator-validation
	@$(MAKE) check-commercial-readiness
	@$(MAKE) check-install-update-disk-hygiene
	@$(MAKE) check-component-integrity-protocol
	@$(MAKE) check-ui-automation-contract
	@$(MAKE) check-gui-runtime-capability-contract
	@$(MAKE) audit-dependencies
	@$(MAKE) operator-package-preflight
	@$(MAKE) operator-package-dynamic-smoke
	@jq empty docs/book-manifest.json
	@$(MAKE) check-minimal-industrial-closure
	@cd apps/web && mix test test/kyuubiki_web/api/operator_task_api_test.exs test/kyuubiki_web/orchestra/operator_task_executor_test.exs test/kyuubiki_web/orchestra/operator_task_ir_test.exs
	@cd workers/rust && cargo test -p kyuubiki-cli operator_task_ir_rpc
	@cd workers/rust && cargo test -p kyuubiki-cli --test operator_task_live

verify:
	@$(MAKE) check-toolchains
	@$(MAKE) check-elixir-self-host
	@$(MAKE) check-native-script-audit
	@$(MAKE) check-central-store-contract
	@$(MAKE) check-central-database-readiness
	@$(MAKE) check-language-packs
	@$(MAKE) check-version-line
	@$(MAKE) check-operator-reliability
	@$(MAKE) check-commercial-readiness
	@$(MAKE) check-install-update-disk-hygiene
	@$(MAKE) check-component-integrity-protocol
	@$(MAKE) check-ui-automation-contract
	@cd apps/web && mix format --check-formatted && mix test
	@cd workers/rust && cargo fmt --check && cargo test
	@$(MAKE) audit-rust-lines
	@$(MAKE) audit-project-organization
	@$(MAKE) audit-dependencies
	@$(MAKE) operator-package-preflight
	@$(ENTRYPOINT) sdk-smoke
	@cd workers/rust && cargo run --release -q -p kyuubiki-benchmark -- --profile $${PROFILE:-10k} --repeat $${REPEAT:-3} --baseline-compare benchmarks/$${PROFILE:-10k}-baseline.json --fail-on-median-regression-pct $${BENCHMARK_MEDIAN_THRESHOLD:-25} --fail-on-rss-regression-pct $${BENCHMARK_RSS_THRESHOLD:-20} --min-baseline-median-ms $${BENCHMARK_MIN_BASELINE_MS:-5.0}
	@$(ENTRYPOINT) playground-fem-node-test
