# Stabilized Transport Research Verification

Date: 2026-10-08. Scope: source macOS, steady 1D upwind transport and native
Headless -> Orchestra -> Rust Agent research execution. No installed App,
remote-scale acceptance or historical qualification promotion is included.

## Reproduced Gaps and Repairs

Eight newly added engine regressions initially failed: stabilization diagnostics
were discarded, malformed records were ignored, and optional quality terms were
not recognized. Diagnosis now preserves the three measured stabilization groups,
checks complete finite records and scaled local flux identity, and exposes opt-in
quality terms without changing the original four-term policy.

Two Headless typed-graph regressions also initially failed. Graph serialization
emitted optional `null` fields; dataset roundtrips lost the required schema marker
and emitted null metadata. The protocol now omits unset authoring values and owns
the validated v1 dataset marker without changing Rust dataset struct construction.
Headless still rejects an explicitly null raw dataset contract.

Actual service execution exposed a missing transport diagnostic dispatch, beyond
the successful in-process workflow tests. Orchestra now delegates transport
diagnosis and scoring through TaskIR to the existing Rust engine implementation.
Only those two operator IDs are admitted by the new adapter, with kind checks,
parent-job and node context, routing constraints and task-bound completion.
The scheduler's optional four-argument callbacks preserve node context while
existing three-argument clients remain supported.

A refined negative-velocity solver result then exposed TaskIR digest disagreement:
Elixir's runtime fixed-decimal formatter encoded `-7.1431743528760725` with last
digit `3`, while Rust correctly used last digit `2` at 15 decimal places.
Elixir now rounds the exact IEEE significand using integer arithmetic and
ties-to-even, matching the existing Rust contract. A shared 15-case fixture
covers the reproduced flux, positive/negative exact halfway values, signed zero,
tiny and large finite values. Physical payload values are not changed.

## Bounded Numerical and Recovery Evidence

The source workflow regression compares two/eight-element uniform paths, both
velocity signs and both connectivity orders. Independently derived conservative
flux and `abs(v)*h/2` diagnostics agree with actual solves. A caller-owned
artificial-diffusivity target of 1 and readiness limit 2 blocks the two-element
example (5) and accepts the eight-element example (1.25). This is not a
boundary-layer accuracy acceptance rule.

Corrupting each of the three stabilization fields blocks dependent decisions.
The explicit skip policy retains raw results and an independent branch; fresh
healthy evidence restores the chain. A late corrupt record at index 4095 cannot
hide behind earlier healthy peaks. Scaled identity tests cover 1e308 and 1e-300
cancellation and reject contradictions without overflowing intermediate sums.

The live regression submits four workflow jobs: coarse blocked, refined ready,
corrupt retained evidence isolated, and fresh healthy recovery. These require
three solver calls, four diagnostic calls (one fails), and three quality calls:
ten Agent executions, not four full-graph Agent executions. Objective composition
and scheduling remain in Orchestra. Retained batch/report evidence reconstructs
research metrics without another solve, and a changed input batch cannot claim it.

Batch normalization may spell integral model/result parameters as integers instead
of floats. The regression checks full typed input equality and bit-exact numerical
equality of all remaining raw fields; only integer/float spellings within the exact
integer range may differ. It does not claim unchanged JSON token spellings.
A recovered partial workflow with a completed job is not a ready research result.

## Verification

All checks below ran against source on macOS ARM64. Counts describe executed
test suites, not a percentage of physical or product coverage.

| Check | Result |
| --- | --- |
| Engine library | 645 passed, 1 existing ignored test requiring a prebuilt operator-template cdylib |
| Thermo/transport and stabilized research integration | 7 + 2 passed |
| Agent/CLI binary unit tests | 186 passed, including 4 native transport TaskIR tests |
| Complete live Headless/Orchestra/Agent regression | 60 passed |
| Final bit-exact retained research regression | 1 passed after tightening integer-rewrite bounds |
| Transport validation profile | 8 commands, 47 passed: 37 solver + 8 diagnostic + 2 workflow |
| Protocol qualification | 132 passed, 61 RPC methods, 5 checked TaskIR examples |
| Headless qualification | 546 passed: 530 SDK + 16 CLI library |
| Elixir canonical/TaskIR/runtime/recovery/API regression | 86 passed |
| Rust all-target strict Clippy and formatting | Passed |
| Elixir formatting and test compile with warnings as errors | Passed |

The executed transport report and both qualification reports also passed their
native readback validators. Reports are retained only under ignored `tmp/`:
`advection-diffusion-research-validation.json`,
`protocol-validation-qualification-report.json` and
`headless-workflow-qualification-report.json`; no machine state or large result
payload is promoted into version control.

Topology, module/function matrix, extension standard, runtime API contracts,
tensor self-test/evidence checks, documentation inventory/book, project
organization, version alignment and all 59 operator-profile declarations pass.
The tensor remains structurally complete but maturity-blocked: 0 structural
gaps, 4 maturity gaps, 19 evidence-grade gaps and 14 P0 coordinates.
Source files remain within 800 lines and documentation within 2000 lines.

Reproduction uses the native runner's `check-operator-validation --profile
advection-diffusion-bar-closed-form --execute` and the Rust test targets
`kyuubiki-engine` (library and the two named integration targets),
`kyuubiki-cli --bin kyuubiki-cli` and `kyuubiki-cli --test headless_live`.
Use serial live tests (`-- --test-threads=1`) and isolated local service ports;
the live fixtures clean up their own processes and temporary workspaces.

## Limits

This closes a bounded research chain, not multidimensional/transient transport,
reaction coupling, arbitrary junction materials, calibrated material research,
installed GUI behavior or remote large-mesh qualification. The legacy fixed-15
TaskIR digest is still lossy and is not the exact execution-input fingerprint or
an authenticated provenance signature. Historical qualification records remain
unchanged. Added diagnostic scanning is linear with fixed peak state and lazy
error formatting; no new large-model benchmark claim is made.

Contract and usage: [stabilized transport research](../docs/advection-diffusion-research-chain.md).

Subsequent shared-scan cancellation and its expanded regression results are
recorded separately in [diagnostic cancellation verification](diagnostic-cancellation-reliability-20261008.md).
