# Configuration Contracts

This directory contains repository-owned configuration contracts. These files
are intentionally checked in because they define release gates, benchmark
coverage, toolchain expectations, or planning queues that must be visible to
contributors and automation.

## Reliability And Qualification

- `operator-reliability-manifest.json`
  Release-level operator reliability index. It declares the current coverage
  matrix, trust-level vocabulary, minimum release gate, and per-domain shards.
- `operator-reliability/*.json`
  Per-domain solve-operator reliability shards. Each entry maps one
  `physics-coverage` benchmark template to one exported solve operator plus
  evidence, limits, and the current trust level.
- `operator-validation-profiles.json`
  Operator Validation Harness profile contract. It groups operators into
  executable validation profiles with analytic checks, local formal
  invariants, cross-check commands, evidence paths, and the
  `schemas/operator-validation-profiles.schema.json` input shape.
- `operator-qualification-roadmap.json`
  Planning queue for the first review-level operators that should be hardened
  toward stronger trust. Each candidate records target level, evidence phase,
  primary blocker, preferred validation lane, and release-gate impact. It does
  not by itself upgrade any operator.
- `operator-qualification-evidence-kits.json`
  Planning-grade artifact checklist for each qualification roadmap candidate.
  A kit describes what must be collected before real `evidence.qualification`
  can be added to the reliability shards. Command-backed artifacts can pair a
  capture command with a separate check command for generated release bundles.
- `releases/qualification-records/<version>.json`
  Release-bound staging records for qualification evidence bundles. These
  records bind a release snapshot, candidate IDs, capture commands, check
  commands, and evidence bundle paths before any operator trust-level
  promotion.

Run `make check-operator-reliability` and `make check-operator-validation`
after changing any of these files. Use `make verify-operator-validation` when
the profile commands themselves should be executed.

## Benchmark And Audit Inputs

- `architecture/module-topology.json`
  Strict module topology used to map product shells, control plane, runtime,
  SDKs, contracts, and verification gates onto benchmark and security lanes.
  This is the machine-readable architecture map for targeted performance and
  safety testing.
- `architecture/module-function-coverage-matrix.json`
  Module x function-paradigm coverage matrix. It checks that each architecture
  module has explicit coverage status across product surface, runtime API,
  solver execution, workflow composition, validation, benchmark, security,
  persistence, deployment, and headless SDK paradigms.
- `architecture/module-extension-standard.json`
  Machine-readable onboarding standard for new modules, function paradigms,
  service surfaces, evidence lanes, and contract families. It keeps future
  architecture growth tied to topology, matrix, tensor, docs, and gates.
- `architecture/central-store-contract.json`
  Central-server store contract checker input. It lists the schemas, backend
  surfaces, frontend API surfaces, docs, readiness scripts, and text checks
  that keep the future catalog/auth/publish/provenance/database plane aligned
  before write-side publishing exists. Its shape is guarded by
  `schemas/central-store-contract-check.schema.json`.
- `architecture/contracts-runtime-api-surface.json`
  Shared runtime API family map. It records frontend, protocol, orchestra, and
  central-store API sources plus client surfaces and internal service-surface
  bindings such as `central-web-service` under `orchestra-control-plane`.
  Its shape is guarded by `schemas/contracts-runtime-api-surface.schema.json`.
  Use `./scripts/kyuubiki check-contracts-runtime-api-surface` for the native
  gate; the retained `.mjs` script is only a parity helper for explicit
  fixture or negative checks.
- `benchmark-profile-coverage.json`
  Benchmark profile coverage map used by performance and coverage tooling.
- `dependency-audit-lockfiles.json`
  Security-audit lane contract for npm and Rust lockfile checks.

Run `make check-module-topology` after changing architecture topology. Run
`make check-module-function-matrix` after changing module/function coverage.
Run `make check-module-extension-standard` after changing the extension flow.
Run `make audit-dependencies` after changing dependency-audit lanes.

## Toolchains

- `toolchains.json`
  Self-host toolchain expectation map for Elixir, Mix, OTP, and related
  runtime checks.

Run `make check-elixir-self-host` after changing the toolchain contract.

## GUI Runtime Capabilities

- `config/gui-runtime-capabilities/*.json`
  Product-owned GUI-to-runtime capability manifests for Hub, Workbench,
  Installer, and mobile WebView surfaces. They use
  `kyuubiki.gui-runtime-capability-manifest/v1` and
  `schemas/gui-runtime-capability-manifest.schema.json` to keep GUI surfaces
  decoupled from orchestra, agent, mesh, installer runtime, and offline bundle
  implementations. The reference example is
  `schemas/examples.gui-runtime-capability-manifest.json`.

Run `make check-gui-runtime-capability-contract` after changing these manifests.

## Rules

- Keep paths repository-relative.
- Do not store credentials, host-local secrets, or lab-machine configuration
  here.
- Prefer machine-readable JSON contracts plus a matching schema in `schemas/`
  when a config file becomes part of a release or installer gate.
