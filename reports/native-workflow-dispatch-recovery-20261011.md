# Native Workflow Ambiguous Dispatch Recovery Fence

Date: 2026-10-11. Local macOS ARM64 source verification at commit `a20f6c8a`
(`daji 3.5.5`) with the working-tree overlay. Package metadata remains `3.5.0`.
This extends [parent dispatch identity](native-workflow-dispatch-identity-20261011.md)
to automatic graph recovery after process or runner loss.

## Reproduced Gap

A retained native TaskIR dispatch remained `dispatch_boundary_unconfirmed`, but
the recovery coordinator reported one recovered job and launched another graph
generation. Its replay policy checked graph idempotence or checkpoint metadata,
not whether the original Agent execution had a confirmed outcome. A focused
coordinator regression failed before the repair. Mathematical repeatability is
not confirmation that an old execution finished or permission to duplicate it.

The coordinator now prepares its digest-verified claim through a dedicated
dispatch guard before any durable claim write or runner launch. This reuses the
existing envelope verification once rather than hashing large inputs twice.
Normal startup requires a genuinely pending generation/attempt zero and checks
the same native dispatch fence: rollback to pending cannot create a first-start
bypass. Parent identity must agree with the verified execution context.

The journal checks at most its existing 512 in-memory records, with no per-job
directory reload, Agent probe, file mutation, pruning or result publication.
Both `dispatch_boundary_unconfirmed` and `outcome_unknown` deny admission when
the recorded RPC parent matches. A newer terminal attempt never erases an older
ambiguous attempt. Legacy/unbound records can conservatively deny admission only
by an exact generated native graph child identity, not a prefix. This association
does not grant cancellation authority or invent a missing RPC parent.

Blocked recovery retains the original envelope, generation and attempt, records
`workflow_dispatch_outcome_unconfirmed`, and supplies
`inspect_original_dispatch_before_explicit_resubmit`. Idempotent or verified
checkpoint policy cannot override ambiguity. A corrupt/unavailable journal
fails closed with `workflow_dispatch_journal_unavailable` and the action
`restore_dispatch_journal_before_recovery`; purely local graphs remain independent.
Terminal observations add no ambiguity fence but still require the original
replay-policy, lease, job and claim checks. They are not cached node results.

## Actual Process Restart Chain

Five rounds cover thermal, electrostatic, magnetostatic, thermomechanical and
Stokes diagnostic/scoring chains with two owned real Rust Agents and an Orchestra
BEAM process. Electrostatic and thermomechanical rounds use verified-shaped
checkpoint policies; the other three use idempotent policies.

Each round reserves capacity with one owner diagnosis, admits a graph and holds
its native diagnosis at the existing 64-record cancellation checkpoint. The
test kills the actual BEAM process while the parent is still running. Its SQLite
database, dispatch directory and Orchestra lease name survive the restart; the
new process has a different session and lease owner. The production 15-second
lease TTL is not shortened or forced expired.

The live launcher now starts with `--no-start` and configures its owned journal
before application startup. Recovery cannot first inspect an empty temporary
journal. Ordinary tests still use fresh isolated databases; stable SQLite is
explicitly opt-in for this restart test. Launcher helpers are split into a
dedicated test module, keeping source files below the 800-line limit.

After lease takeover, all five parents are `recovery_blocked`, with no new claim,
diagnostic/scoring attempt or successful dependent output. The original attempt
is byte-stable and owner/peer execution counts do not increase. Closed transport
cancels the original held execution; read-only receipt retrieval returns its
original generation and cancellation cause. Retrieval neither updates the job
nor journal, and successful SDK waiting rejects the blocked parent.

Five explicit new submissions then complete diagnosis, quality, objective and
decision chains. Full retained results, not SDK previews, match the Engine
reference reducers. Old blocked jobs and ambiguous attempts remain unchanged.
The five rounds require exactly 20 Agent executions: five capacity holders,
five interrupted diagnoses and ten native operators in explicit new graphs.

The payload repeats 129 computed records to exercise control checkpoints. This
is not a large physical mesh, numerical convergence benchmark or independent
scientific accuracy qualification. Scratch databases, journals and owned child
processes are removed by the harness; no backup or installed-runtime tree is added.

## Verification

The focused recovery selection passed 49 tests, including 34 new dispatch-recovery
cases and existing envelope/restart checks. The expanded SQLite selection passed
417 executed tests with four backend-exclusive memory tests skipped (421 listed)
in 28.7 seconds. A JSON/memory-backend selection passed 66 tests in 3.3 seconds,
including those memory persistence cases. These selections overlap and must not
be summed as independent coverage.

The final complete Headless live suite passed all 69 tests in 291.44 seconds.
Its new restart case contains the five actual native rounds described above;
other live cases retain their existing mock/fixture and fault-injection scopes.
An earlier complete run exposed two startup-scan races in opt-in incomplete-result
fixtures. A coordinator startup barrier before fixture seeding repaired both,
and focused reruns plus the final complete run passed. This barrier does not
disable production recovery or shorten lease takeover.

Rust Headless SDK verification passed 532 unit tests, 14 composite-feedback tests
and 11 composite-heat validation tests. The native CLI passed 189 unit tests.
Strict all-target CLI and Headless SDK Clippy, Rust/Elixir formatting and diff
whitespace checks passed.

Native topology, module/function matrix, extension-standard, runtime API,
tensor/self-test, documentation inventory/book, version and project-organization
checks passed. The version audit checked 224 exact contracts with zero mismatches.
The organization audit reports no tracked debt under the 800-source/2000-document
limits. The tensor still has zero structural gaps, four maturity gaps, 19
evidence-grade gaps and 14 P0 gaps, with Daji acceptance blocked. The new claim
registers bounded source recovery evidence without promoting broad acceptance.

## Boundaries

This qualifies local source restart denial and explicit new submission, not
durable per-node continuation, exactly-once execution, cryptographic provenance,
power-loss durability, PostgreSQL failover, shared-directory multi-writer use,
arbitrary parallel graph races or installed/authenticated remote acceptance.
The journal still covers only the bounded native TaskIR workflow adapter, not
legacy typed solver RPCs, local operators or external package cleanup. Agent
receipt expiry/restart can lose readback; absence remains unknown.

A verified retained receipt does not automatically commit a lost node or resume
its successors. That needs a separate identity-bound continuation protocol.
Neither Engine physics nor Agent executable protocols change here. No version
bump, Git commit/push, installation or remote benchmark is performed.
