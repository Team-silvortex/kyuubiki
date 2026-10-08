# Native Domain Diagnostic Chains

Date: 2026-10-08. Source macOS ARM64 verification extends the existing
transport TaskIR adapter to thermal, electrostatic, magnetostatic,
thermo-mechanical and Stokes diagnostics and their quality scores. Orchestra
keeps graph scheduling and objective composition; the Rust Engine keeps the
diagnostic and quality implementations. Numerical solvers are unchanged.

## Reproduced Gaps and Repairs

Before the adapter extension, all three new Rust TaskIR regressions failed,
and three of four expanded Elixir adapter tests failed. Non-transport diagnostic
tasks were admitted but returned `verified_pending_engine_execution`, not a
computed result. The native workflow whitelist now contains six extraction
operators and five scoring operators. Thermo-mechanical diagnostics reuse
`transform.score_thermal_quality`; no fictitious thermo scorer was added.

Exact IDs and kinds are required. Prefix matches and wrong kinds cannot select
another native entrypoint. Preflight never executes. Catalog descriptors declare
unbundled engine built-ins, not central-library packages. Task construction retains
the original node, parent job, authority context, placement and caller configuration.
Stale task identities and pending delivery acknowledgements cannot publish output.

The first real multi-domain workflow run then exposed a separate Stokes dispatch
bug: both workflow solver registry entries omitted `plane` from the Agent RPC
method name. The Agent rejected the unknown request, and the strict transport
reported an unexpected frame; retrying a peer repeated the same mapping defect.
An added Elixir mapping regression failed before repair. Both quad and triangle
methods now match the Rust protocol. Execution-program construction resolves
registered solvers through that same registry instead of deriving their method
from operator-ID spelling. Unknown extension-ID behavior remains unchanged.

## Real Service Evidence

The new study-chain fixture starts an isolated Orchestra and two owned Agents.
For each of five domains it executes:

1. A real single-quad solve -> diagnostic -> quality -> objective graph.
2. A graph over corrupted retained results with explicit `on_error: skip`.
3. A graph over healthy retained results without solving again.

The corrupt record produces one diagnostic failure. Quality, objective and
decision nodes are skipped, with no corresponding summaries. Raw evidence and
an independent output remain available. Recovery is a separate explicit graph;
the original failed trace stays failed. Summaries and scores agree with Engine
reductions, including unchanged floating-point bits, allowing only exact JSON
integer/float spelling differences.

A real Stokes triangle solve/diagnose/score chain additionally protects its
distinct RPC mapping. These sixteen jobs perform exactly 33 Agent executions:
six typed solver RPCs, sixteen diagnostic TaskIRs and eleven scoring TaskIRs.
The diagnostic/scoring journal records retain original task, program, digest,
request and attempt identities. Public SDK inspection observes exactly one
attempt per dispatch. Inspection is evidence-only and never claims that the
terminal payload has been retrieved or that automatic replay is authorized.

The standalone cancellation fixture covers the same five diagnostic domains.
It repeats computed records to make 129-node/element reducer inputs; these are
scan fixtures, not 129-element physical models. Each task is cancelled at the
64-record safe point through its exact process/request/generation target.
No partial summary or downstream project write is published. A same-task
explicit rerun succeeds with a fresh attempt while the old attempt stays failed.
Five cancellations plus five explicit reruns require exactly ten Agent executions.

## Verification

| Suite | Result |
| --- | --- |
| CLI and Agent library | 189 passed |
| New cross-domain native TaskIR unit module | 3 passed, included above |
| Expanded Orchestra, graph, CFD, program and digest tests | 119 passed |
| Complete actual Headless service regression | 63 passed |
| New actual service chain and cancellation targets | 2 passed, 61 filtered |
| Engine library | 649 passed, 1 existing ignored |
| Five Engine diagnostic and stabilization integration targets | 25 passed |
| Headless SDK library | 530 passed |
| Protocol library | 132 passed |

The ignored Engine test requires a separately built operator-template dynamic
library. No dynamic-plugin qualification follows from these runs. Counts overlap
with targeted suites and are not summed into unique feature coverage.

All-target Clippy passed with warnings denied for CLI, Engine, Solver and Headless
SDK. Rust/Elixir formatting and diff whitespace checks passed. Native topology,
matrix, extension standard, runtime API surface, documentation inventory/book,
version, organization and tensor/self-test checks passed. Source files stay within
800 lines and documents within 2000, with zero tracked organization debt.
The tensor retains zero structural gaps, four maturity gaps, 19 evidence-grade
gaps and 14 P0 gaps; Daji acceptance remains blocked. Historical qualification
grades are unchanged. No commit, package rebuild or installation was performed.

## Scope

This is bounded source-service contract and recovery evidence, not new numerical
accuracy, installed/remote execution, mesh scale or material acceptance. Selected
quality targets demonstrate execution policy, not calibrated research criteria.
Stokes and the other study solves still use typed solver RPCs; this extension
does not add TaskIR solver capabilities or move an entire graph into one Agent.
Legacy direct Elixir diagnostic helpers and non-whitelisted workflow operations
are outside the adapter. Public whole-job cancellation, durable continuation,
plugin preemption, JSON processing latency and historical qualification remain
separate gaps. Generated services and records use disposable test directories.
