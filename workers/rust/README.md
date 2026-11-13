# Rust Workspace

`workers/rust` contains the native runtime, SDKs, deployment and repository
tools. [Cargo.toml](Cargo.toml) owns workspace membership; the
[current architecture map](../../docs/current-architecture-map.md) owns the
cross-language call paths and individual crate responsibilities.

## Find The Owner

| Area | Crates under `crates/` | Boundary |
| --- | --- | --- |
| Shared contracts and transport | `protocol`, `agent-transport`, `platform` | TaskIR/RPC, native TLS/token I/O and OS primitives |
| Execution and numerical methods | `cli`, `engine`, `solver` | Agent admission/lifecycle, reusable dispatch and numerical kernels |
| Native operator extension | `operator-sdk`, `operator-thermal-q9` | Rust authoring/ABI contracts and a concrete native operator |
| UI-free research and data exchange | `headless-sdk`, `project-automation`, `project-bundle`, `kcore` | Control clients, stored automation, editable projects and frozen results |
| Deployment and application lifecycle | `installer`, `desktop-runtime`, `deploy-server` | Installation/integrity, native shell services and artifact delivery |
| Repository verification | `benchmark`, `script-runner` | Measured performance lanes and native development tools |

These groups are navigation, not new crates or services. Engine/Solver stay
independent of GUI and Orchestra. The Agent uses its own executable for
supervised external-operator children; it does not require another installed
service. Child-only IPC/loading lives under
[`cli/src/operator_package_process/`](crates/cli/src/operator_package_process/).
Direct Engine embedding remains in-process. Fault containment is not a
malicious-code sandbox; see the
[isolation limits](../../docs/operator-store-integration.html#native-process-isolation).

The [operator template](templates/operator-crate-template/) is a copyable,
independently built starter, not another workspace member. Keep Worker/Operator
SDK extension separate from Headless SDK control. Neither Store login nor an
installed package grants Agent execution authority.

Use [Headless SDK guidance](../../docs/headless-sdks.md) for research clients and
[Operator SDK guidance](../../docs/operator-sdk.md) for native extensions. The
CLI is a consumer of those contracts, not the mandatory gateway for SDK users.

## Tests And Evidence

Crate-level test expectations stay close to the code:

- `crates/solver/tests/`
  solver behavior, numerical baselines, and progress/result validation
- other crate-local `tests/`
  engine, CLI agent, benchmark, and installer/runtime checks

Prefer colocated crate tests so runtime behavior stays near the code it
validates.

Native sweep controls live in `crates/engine/src/tests/workflow_sweep_*` and use
shared Elixir/Rust fixtures. Run the Engine library suite in this workspace:

```text
cargo test --locked --offline -p kyuubiki-engine --lib
```

Retained evidence stays in the reports rather than becoming a second changelog
here. The following are bounded fresh-run controls, not durable restart, live
Agent, scale or physical qualification:

| Evidence | Scope |
| --- | --- |
| [Selection and mapping](../../reports/native-sweep-selection-recovery-20261012.md) | Calibrated fixed score reference, blocked selection and fresh recovery |
| [Collection and summaries](../../reports/native-sweep-collection-recovery-20261012.md) | Convergence, stale-summary cleanup and provenance |
| [Quality decisions](../../reports/native-quality-decision-recovery-20261012.md) | Admission, partial evaluation and repair planning |
| [Ranking consistency](../../reports/quality-ranking-consistency-recovery-20261012.md) | Canonical order and matching score copies, without silent repair |
| [Coupled readiness](../../reports/coupled-readiness-admission-recovery-20261012.md) | Conflicting aliases, malformed reports and explicit blockers |
| [Sweep request admission](../../reports/quality-sweep-request-admission-recovery-20261012.md) | Readiness aliases and repair intent before stopped/incomplete returns |
| [Plan admission](../../reports/quality-sweep-plan-admission-recovery-20261012.md) | Execution intent, repair metadata and declared axis counts |
| [Expansion admission](../../reports/quality-sweep-expansion-admission-recovery-20261012.md) | Direct callers and preservation of repair evidence |
| [Count consistency](../../reports/sweep-expansion-count-recovery-20261012.md) | Axis counts, deferred declarations and count mirrors |

For numerical optimization and benchmark limits, use the
[solver matrix pack](../../docs/solver-matrix-optimization-pack.md). Test sizes
are profile-specific; a benchmark driver is not proof of every operator's scale.

## Build Storage

Reuse this workspace's `target/` for runtime work. From the repository root,
`make disk-usage` previews regenerable caches and `make clean-dev-cache` removes
them while builds/tests are idle. This retains dependencies, release bundles,
top-level tools and runtime data; subsequent tests may relink. See the
[disk ownership rules](../../docs/project-architecture-organization.md#generated-disk-use)
before deleting ignored directories or creating another build root.

## Layout Skeletons

- `linux/`
  repository-visible Linux install-layout skeleton used to keep expected
  runtime paths explicit even when no packaged output is committed yet
