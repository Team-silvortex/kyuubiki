# Modal Agent TaskIR Execution and Transport Reliability

Date: 2026-10-02
Version: daji 3.4.3 working tree
Scope: local macOS development-Agent subprocesses, real loopback TCP, Rust SDK preparation

This is bounded live modal-Agent validation, not general Agent solver qualification.

## Reproduced Boundary

The previous [Rust JSON report](modal-json-roundtrip-reliability-20261002.md)
explicitly excludes independent Agent transport. Direct modal RPC could already
solve the retained models, but the Agent TaskIR capability and typed Engine
adapter exposed only `solve.bar_1d`. A valid modal TaskIR therefore failed with
`operator_task_solver_capability_rejected` before numerical execution.

Before the fix, the new Protocol admission test failed and four live TaskIR
journeys failed at that capability boundary. The direct modal RPC check passed.
Those are functionality/admission failures, not newly discovered FEM inaccuracies.
Inspection also found that built-in solver summaries did not check the declared
entrypoint name against the operator ID. Solver dispatch used the ID and ignored
a contradictory entrypoint. A regression now rejects that contradiction.

## Change and Ownership

The explicit Agent TaskIR allowlist gains only `solve.modal_frame_2d` and
`solve.modal_frame_3d`. Their adapters deserialize the corresponding Protocol
request, call Engine, verify the matching result variant and serialize the result.
No numerical kernel, tolerance, refinement budget or solver algorithm changes.
The Agent remains an execution/lifecycle host, not a parallel solver owner.

The headless bridge descriptor publishes its actual `solver_execution_capability`.
Protocol checks the known built-in RPC names, including the original bar route;
it does not impose a naming convention on unknown external solver implementations.
Digest, authoring/program mirrors, admission, offline authority and package-fetch
checks remain in force. Other direct RPC methods are not automatically enabled
as TaskIR routes. Central-fetch modal tasks still cannot use the offline-only
built-in capability to bypass package ownership.

## Retained Coverage

Three new Protocol tests cover both modal routes, unsupported/fetch rejection
and contradictory built-in entrypoints. Seven new live tests use the existing
Agent lifecycle harness; that target also runs its two shared readiness tests.

- Direct RPC and TaskIR compare all received numeric bits with actual Engine
  outputs for 96/100-element planar twenty-mode and 100-element spatial six-mode
  unit-length bending models. Engine baseline agreement is dispatch/transport
  evidence, not an independent physical reference.
- Every received shape is separately checked against test-only Euler-Bernoulli
  assembly, the original `1e-8` physical residual gate, restrained DOFs, shape
  normalization, frequency/period consistency and discrete low-mode roots.
- Rust SDK preparation verifies an executable TaskIR digest. Its configuration
  contains finite subnormal/signed-zero/fraction metadata to exercise text
  readback without treating extreme metadata as physical material inputs.
  Agent receipts preserve the original task digest and report accepted execution.
- Digest tampering, entrypoint contradiction, invalid authority, unattached
  central fetch, unsupported solver and malformed typed input return errors.
  No failure contains a successful or partial modal result.
- Fragmented writes and invalid numeric JSON precede an unresolved 128-element
  six-mode request on one TCP connection. The numerical failure retains its
  error, and a valid 100-element request succeeds on that same connection.
- Cancellation is observed after real sparse-matvec steps through the existing
  explicit fault-injection safe point, not a sleep-based race. It returns a
  non-resumable checkpoint and no modes. Admission returns to zero active
  executions and a healthy task succeeds while the exact-job hold marker remains.
- Descriptor checks admit only the three attached TaskIR routes and bind them
  to existing Engine operator entries. The original bar qualification is rerun.

The 128-element request remains a fail-closed case, not a supported-solve claim.
The spatial check covers three repeated bending pairs, not the full spectrum.
All temporary Agent processes, logs and hold markers are owned by the test
harness and removed by its existing teardown, including failed assertions.

## Verification

| Check | Result |
| --- | --- |
| Protocol suite | 118 passed, 0 failed; 111 unit and 7 integration cases |
| Rust Headless and Operator SDK suites | 329 passed, 0 failed |
| New live Agent target | 9 passed, including 7 new journeys and 2 shared readiness cases |
| Original bar TaskIR qualification | 1 passed |
| In-process headless modal target | 35 passed |
| Lifecycle/orphan/shutdown/solver-cancel and control-plane TaskIR targets | 76 passed |
| Full CLI suite | 488 passed, 0 failed, 4 existing ignored; 43 result groups |
| Engine unit suite | 637 passed, 0 failed, 1 existing ignored |
| Executed modal profile | 27 commands, 241 passed, 0 failed; artifact `ok=true` |
| Focused Protocol/CLI Clippy | Passed, warnings denied |
| Formatting, TaskIR examples, documentation, tensor and organization gates | Passed |

Overlapping runs must not be added as distinct coverage. The CLI ignored cases
are an opt-in projection microbenchmark and three tests requiring a prebuilt
external operator cdylib. Engine also retains one prebuilt-cdylib test. No new
ignored case is introduced; this report does not claim those paths passed.
The full Solver suite is not rerun here; the modal profile reruns its bounded
modal targets, and no numerical implementation is changed in this follow-up.

The compact machine-readable artifact is `tmp/modal-agent-taskir-validation.json`.
Two separately scoped claims are registered in
`config/architecture/module-function-coverage-evidence/runtime-modal-agent.json`
and included by the tensor configuration. Tensor structure passes with zero
structural gaps; global readiness remains `blocked`, with four maturity,
sixteen evidence-grade and eleven P0 gaps. Local modal coverage does not erase
those scope-specific obligations.

## Scope Limits

These are local, bounded development-binary tests. TCP submission is through
the retained test harness after Rust SDK preparation/validation, not a complete
SDK HTTP/control-plane client journey. There is no Installer-managed release
deployment, remote/Linux run, Python/Elixir execution, TLS qualification,
long-running load, exhaustive malformed-frame validation or performance claim.
The existing retained bar-only qualified evidence is not promoted to modal or
all solvers. New tensor claims remain `verified`, not `qualified` or `operational`.

Reproduce through native Rust tools:

```text
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-protocol --lib solver_execution_capability
cargo test --manifest-path workers/rust/Cargo.toml --locked --offline -p kyuubiki-cli --test agent_modal_live --test agent_task_ir_qualification --test modal_spectrum_operator
```

Compact validation reports are intentionally ignored under `tmp/`. Large model
copies and deployment backups are not created for these bounded fixtures.

## Source Fingerprints

SHA-256 of the final route/admission adapters, tests and evidence bindings:

```text
bdb57729d9acb457aaca99f060ecf2443105ca0f7f309a41f3f46029519137ee  workers/rust/crates/protocol/src/solver_execution_capability.rs
f484443334f5da733c2258954c58202834bdd151d8b78d844ba8f0356ba89e6c  workers/rust/crates/protocol/src/operator_task_ir.rs
f995b5519ccfcfed79f9cc6ef96b7378a411e6c0093115374476c391528174ff  workers/rust/crates/protocol/src/tests/solver_execution_capability.rs
e4d324ab622bfdcd67df95d5b817c022a3e4fcaaae3b6a157bf3392e50616a3f  workers/rust/crates/cli/src/agent_headless_bridge.rs
4912bd9c7f158e3c166f2a78b8f4337739e1343fb2a501a2ef749537219bc1c3  workers/rust/crates/cli/src/operator_task_runtime/engine_solver.rs
b9307ee5d2eead31e8e426b6b08ebf8ff70b46f9e8c6786ad173fef453282477  workers/rust/crates/cli/tests/agent_modal_live.rs
d0a287d7de1d4705d6ca1fbe811f239f1eb4708605808595ba9cef48e1d22012  workers/rust/crates/cli/tests/support/modal_agent.rs
5737d79d3e1934b9a6d2241d906206b66db949d4aad29f5c3f337124a8d374db  config/operator-validation-profiles/modal.json
d7163e0a1f2e32aadfcffe53f99c69098918829aa223dce2288309d20193efa5  config/architecture/module-function-coverage-evidence/runtime-modal-agent.json
```

Generated development test executables and incremental caches are reclaimed
with the native `dev-disk` entrypoint after verification. Source, compact
evidence, dependencies and installed release applications are retained.
