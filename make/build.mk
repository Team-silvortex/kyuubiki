.PHONY: build-frontend build-orchestrator build-agent
.PHONY: package-runtime package-desktop package
.PHONY: operator-package-preflight operator-package-dynamic-smoke check-operator-package-dynamic-smoke sync-desktop-shared
.PHONY: build-installation-docs build-update-catalog

build-frontend:
	@$(ENTRYPOINT) build-frontend

build-orchestrator:
	@$(ENTRYPOINT) build-orchestrator

build-agent:
	@$(ENTRYPOINT) build-agent

package:
	@$(ENTRYPOINT) package $(ARGS)

package-runtime:
	@$(ENTRYPOINT) package-runtime $(ARGS)

package-desktop:
	@$(ENTRYPOINT) package-desktop $(PLATFORM)

operator-package-preflight:
	@$(ENTRYPOINT) operator-package-preflight $(or $(PACKAGES_ROOT),$(CURDIR)/workers/rust/templates) $(if $(OUT),--out $(abspath $(OUT)),) $(if $(FAIL_ON_REJECTED),--fail-on-rejected,)

operator-package-dynamic-smoke:
	@$(ENTRYPOINT) operator-package-dynamic-smoke $(if $(OUT),--out $(abspath $(OUT)),)
	@$(MAKE) check-operator-package-dynamic-smoke $(if $(OUT),IN=$(abspath $(OUT)),IN=tmp/operator-package-dynamic-smoke.json)

check-operator-package-dynamic-smoke:
	@node ./scripts/check-operator-package-dynamic-smoke.mjs --self-test
	@node ./scripts/check-operator-package-dynamic-smoke.mjs --in $${IN:-tmp/operator-package-dynamic-smoke.json}

sync-desktop-shared:
	@node ./apps/desktop-shared/scripts/sync-desktop-shared.mjs

build-installation-docs:
	@node ./scripts/build-installation-integrity-docs.mjs

build-update-catalog:
	@node ./scripts/build-update-catalog.mjs
