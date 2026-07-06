.PHONY: start start-local start-cloud start-distributed status stop restart
.PHONY: restart-local restart-cloud restart-distributed
.PHONY: hot-local hot-cloud hot-distributed hot-web hot-agent
.PHONY: export-db install doctor validate-env
.PHONY: smoke worker agent orchestrator playground frontend

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

export-db:
	@$(ENTRYPOINT) export-db

install:
	@$(ENTRYPOINT) install $(ARGS)

doctor:
	@$(ENTRYPOINT) doctor

validate-env:
	@$(ENTRYPOINT) validate-env

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
