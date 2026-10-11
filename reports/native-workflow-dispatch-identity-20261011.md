# Native Workflow Dispatch Identity and Exact Cancellation

Date: 2026-10-11. Source macOS ARM64 verification at commit `a20f6c8a`
(`daji 3.5.5`) with this working-tree overlay; package metadata remains `3.5.0`.
This extends [dispatch admission fencing](native-workflow-dispatch-fencing-20261011.md)
to the retained inspection, exact cancellation and failure-receipt paths.

## Reproduced Identity Gap

A workflow creates a distinct TaskIR identity for each native operator, such as
`<parent>:diagnose:extract.thermal_result_diagnostics`. The actual Agent RPC binds
that execution to the parent graph job. The v1 journal omitted this actual RPC
job; inspection, exact cancellation and Rust SDK validation instead assumed the
execution job equalled the TaskIR id. A real active child could therefore have
no usable cancellation target, and a correctly observed parent target failed
query validation. Seven new focused regressions failed before the repair.

New `kyuubiki.operator-task-dispatch-record/v2` records retain the actual
`params.job_id`, independently of the task and request identities. Inspection
exposes that nullable job on each attempt and grants an exact target only when
the Agent's active request and recorded job agree. Cancellation separately
validates a TaskIR identity up to 1024 UTF-8 bytes and a bounded four-field Agent
target; the target request and job must match the selected retained attempt.
Process incarnation and generation remain fenced by the Agent. No latest-target
selection, peer retargeting, job-wide fallback or automatic retry is added.

The Rust SDK accepts parent-bound and long child identities, rejects drift in
the retained job, and never treats an explicit null job as target authority.
Old wire responses without the optional field retain their task-bound mirror
check; new servers explicitly return null for unknown or unbound jobs.

## Storage Compatibility

V1 and v2 both keep closed field sets, the existing integrity envelope, atomic
generation-checked writes and a 4-KiB per-record budget. The directory remains
limited to 512 records with at most 128 retained terminal observations. Missing
or malformed v2 job fields cannot silently become a valid legacy record.

V1 files load byte-for-byte unchanged, without migration or additional backups.
Their actual RPC job is unknown: inspection returns no exact cancellation target,
and an attempted cancellation returns `operator_task_dispatch_job_identity_unknown`
without an Agent RPC. Read-only original receipt retrieval still works. Explicit
terminal updates preserve the v1 schema and do not invent parent identity.
An unbound v2 RPC follows the same cancellation restriction.

## Real Native Chain Coverage

Five rounds use owned Orchestra and two actual Rust Agents, covering thermal,
electrostatic, magnetostatic, thermomechanical and Stokes diagnostic/scoring
chains. Each round performs exactly five Agent executions:

- One held owner diagnosis reserves capacity until the generated graph parent
  is known, then finishes normally.
- One graph diagnosis stops at the existing 64-record diagnostic checkpoint.
  Rust SDK inspection returns its child task and real parent target; exact
  retained-dispatch cancellation registers only that process/request/generation.
- One peer axial-bar execution deliberately shares the same parent job while
  having a distinct TaskIR and request. It stays active and uncancelled, then
  completes against independent `FL/EA` displacement.
- Two native operators in an explicitly submitted new graph diagnose and score
  successfully. Full retained summaries match the Engine reference reducers.

The five rounds require exactly 25 Agent executions. Diagnosis cancellation
fails its parent workflow under default error handling, rather than pretending
to be a whole-job cancellation. Scoring, objective and decision have no outputs
and scoring has no dispatch attempt. Read-only receipt retrieval returns the
original cancelled generation and failure cause; successful job waiting rejects
the failed graph. Projects, failed jobs and their single failed attempts remain
unchanged after explicit recovery, with no hidden replay or result publication.

The 129-record payload repeats already computed records to exercise cancellation
cadence. It is not a larger physical mesh or a numerical convergence benchmark.
Engine reducer comparisons validate transport and retention, not independent
scientific accuracy. The peer bar has a separate analytic correctness check.

## Verification

The expanded Elixir selection passed all 341 tests in 21.9 seconds. It includes
eight new journal identity/compatibility cases, existing restart and corruption
tests, API authorization, cancellation, receipt retrieval and workflow fencing.
The focused live five-domain test passed in 3.84 seconds.

The Rust Headless SDK passed 532 unit tests and 25 reliability integration tests.
The complete native Headless live suite passed all 68 tests in 158.46 seconds,
including existing original-receipt recovery, acknowledgement loss and Orchestra
restart cases. Focused reruns are subsets, not additional unique coverage.

Strict all-target CLI and Headless SDK Clippy passed with warnings denied. Rust
and Elixir formatting and diff whitespace checks passed. Native topology, matrix,
extension-standard, runtime API surface, tensor/self-test, documentation
inventory/book, version and organization checks passed. The version audit checked
224 exact contracts with zero mismatches. Source/document limits remain 800/2000
lines; the new test modules have 282/236 lines and the SDK manual stays at 1999.
The tensor still reports zero structural gaps, four maturity gaps, 19
evidence-grade gaps and 14 P0 gaps, with Daji acceptance blocked. This repair
registers source evidence without promoting broader maturity or acceptance.

## Remaining Boundaries

This is local source contract/recovery evidence. It does not qualify installed
applications, authenticated remote clusters, arbitrary parallel workflow races,
power-loss durability or rolling mixed-version deployment. Old binaries that
understand only journal v1 will fail closed on v2 records; no downgrade is claimed.
No Engine physics, TaskIR digest or Agent executable protocol was changed, and
no version bump, Git commit/push, installation or remote benchmark was performed.
