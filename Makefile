SHELL := /bin/zsh
ENTRYPOINT := zsh ./scripts/kyuubiki

.PHONY: help tree start status stop restart test test-web test-rust test-playground verify format format-web format-rust tdd-web tdd-rust smoke worker agent orchestrator playground frontend benchmark

help:
	@echo "Available targets:"
	@echo "  make tree        Print the repository scaffold"
	@echo "  make start       Start the orchestrator API, frontend, and solver agent"
	@echo "  make status      Show local service status"
	@echo "  make stop        Stop the orchestrator API, frontend, and solver agent"
	@echo "  make restart     Restart the orchestrator API, frontend, and solver agent"
	@echo "  make test        Run all project tests"
	@echo "  make test-web    Run Elixir tests"
	@echo "  make test-rust   Run Rust workspace tests"
	@echo "  make verify      Run formatting checks and tests"
	@echo "  make format      Format all code"
	@echo "  make smoke       Run the Elixir -> Rust smoke flow"
	@echo "  make worker      Run the Rust mock worker CLI"
	@echo "  make agent       Run the Rust FEM TCP agent"
	@echo "  make orchestrator Run the Elixir orchestrator API"
	@echo "  make playground  Legacy alias for the orchestrator API"
	@echo "  make frontend    Run the Next.js workbench UI"
	@echo "  make benchmark   Run the Rust solver benchmark suite"
	@echo "  make tdd-web     Run a focused Elixir test by FILE=... or TEST=..."
	@echo "  make tdd-rust    Run focused Rust tests with FILTER=..."
	@echo "  zsh ./scripts/kyuubiki help    Show the unified local entrypoint"

tree:
	@find . -maxdepth 3 -type d | sort

start:
	@$(ENTRYPOINT) start

status:
	@$(ENTRYPOINT) status

stop:
	@$(ENTRYPOINT) stop

restart:
	@$(ENTRYPOINT) restart

test: test-web test-rust test-playground

test-web:
	@cd apps/web && mix test

test-rust:
	@cd workers/rust && cargo test

test-playground:
	@node --test apps/web/playground/test/fem.test.mjs

format: format-web format-rust

format-web:
	@cd apps/web && mix format

format-rust:
	@cd workers/rust && cargo fmt

verify:
	@cd apps/web && mix format --check-formatted && mix test
	@cd workers/rust && cargo fmt --check && cargo test
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
