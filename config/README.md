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
- `operator-qualification-roadmap.json`
  Planning queue for the first review-level operators that should be hardened
  toward `qualification`. It does not by itself upgrade any operator.
- `operator-qualification-evidence-kits.json`
  Planning-grade artifact checklist for each qualification roadmap candidate.
  A kit describes what must be collected before real `evidence.qualification`
  can be added to the reliability shards.

Run `make check-operator-reliability` after changing any of these files.

## Benchmark And Audit Inputs

- `benchmark-profile-coverage.json`
  Benchmark profile coverage map used by performance and coverage tooling.
- `dependency-audit-lockfiles.json`
  Security-audit lane contract for npm and Rust lockfile checks.

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
