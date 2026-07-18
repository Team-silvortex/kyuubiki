.PHONY: build-hub-gui build-installer-gui build-workbench-gui
.PHONY: desktop-status desktop-stage desktop-build-host desktop-release desktop-verify
.PHONY: desktop-linux-remote desktop-linux-remote-install-deps desktop-linux-remote-preflight
.PHONY: hot-hub-gui hot-installer-gui hot-workbench-gui
.PHONY: hub-gui-dev hub-gui-build installer-gui-dev installer-gui-build
.PHONY: workbench-gui-dev workbench-gui-build

build-hub-gui:
	@$(ENTRYPOINT) build-hub-gui $(PLATFORM)

build-installer-gui:
	@$(ENTRYPOINT) build-installer-gui $(PLATFORM)

build-workbench-gui:
	@$(ENTRYPOINT) build-workbench-gui $(PLATFORM)

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

hot-hub-gui:
	@$(ENTRYPOINT) hot-hub-gui

hot-installer-gui:
	@$(ENTRYPOINT) hot-installer-gui

hot-workbench-gui:
	@$(ENTRYPOINT) hot-workbench-gui

hub-gui-dev:
	@$(ENTRYPOINT) hub-gui-dev

hub-gui-build:
	@$(ENTRYPOINT) build-hub-gui $(PLATFORM)

installer-gui-dev:
	@$(ENTRYPOINT) installer-gui-dev

installer-gui-build:
	@$(ENTRYPOINT) build-installer-gui $(PLATFORM)

workbench-gui-dev:
	@$(ENTRYPOINT) workbench-gui-dev

workbench-gui-build:
	@$(ENTRYPOINT) build-workbench-gui $(PLATFORM)
