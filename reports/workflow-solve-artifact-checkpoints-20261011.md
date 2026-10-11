# Bounded Workflow Solve Artifact Checkpoints

Date: 2026-10-11. Local macOS ARM64 source verification at `6d9e1094`
(`daji 3.5.6`) with a working-tree overlay. Package metadata remains `3.5.0`.
This extends [native receipt-assisted recovery](native-workflow-success-receipt-recovery-20261011.md)
with small successful typed-solve artifacts, without changing Engine physics or
the [unknown-dispatch fence](native-workflow-dispatch-recovery-20261011.md).

## Recovery Contract

A successful legacy typed solve previously restarted from its input after
Orchestra loss, even when only a later diagnostic or score was unfinished.
Progress metadata is not an output checkpoint. Async graph runners now retain
eligible successful solve results before graph publication through an internal
execution callback. Direct graph runs and new job submissions remain independent.
No checkpoint output, callback or storage path is accepted from public request JSON.

The private recovery record contains one optional closed-field
`kyuubiki.workflow-solve-checkpoints/v1` bucket. It binds the actual parent job
and execution-envelope digest. Each entry binds the node snapshot, exact
invocation input/context digest, lossless output JSON/digest and original
producing claim. The bucket has its own digest. Duplicate identical commits are
read-only acknowledgements; conflicting evidence cannot overwrite an entry.

Output JSON is retained as encoded data, not a path. SQL JSON normalization must
not turn `1.0` into `1` or change a later native TaskIR digest. Tiny values and
signed zero retain their original representation. Invocation input hashing is
lossless rather than TaskIR's fixed-decimal formatting; envelope/node metadata
uses the existing integral-number normalization. JSON round trips preserve the
output numbers and do not execute, load files or evaluate supplied content.

Lookup and retention run in the coordinator under the existing active-job,
execution-claim and Orchestra lease checks. Retention uses the shared job/result
transaction without advancing node progress or restarting execution deadlines.
Only a later authorized generation can reuse a matching artifact. The scheduler
then performs its normal output, lineage, progress and final-result commit gates.
It still reconstructs local ancestors and branches rather than restoring an
entire scheduler snapshot. No scientific algorithm is added to the coordinator.

Before recovery admission advances a claim, the retained bucket is checked for
closed fields, identity, digests and budgets. Corruption blocks without claim
advancement. Changed invocation inputs, conflicting evidence or failed writes
cannot become skip-on-error success. A blocked job retains its envelope and
artifacts, with `inspect_solve_checkpoint_before_explicit_resubmit` as the
operator action. Readback or repair inspection is not a resume authorization API.

Existing idempotent/checkpointed replay policy is still mandatory. A successful
solve checkpoint cannot override side-effect restrictions or an ambiguous
original native dispatch. Original native receipts still use their own bounded
Agent cache and completion gates; they are not copied into this solve bucket.

## Retention Limits

Each parent retains at most 32 entries, 512 KiB of encoded JSON per output and
2 MiB of encoded checkpoint-container bytes. Outputs have a 65,536 JSON-node
budget; the container has a 250,000-node budget. Existing depth, UTF-8, object-key,
string and output JSON limits also apply. Output byte counting stops when the
cap is crossed, before constructing an oversized encoded copy. Byte counting is
conservative near the boundary. Retention stores one entry per eligible solve,
not historical versions, full scheduler snapshots or separate checkpoint files.

A new oversize/capacity-exceeding result is not retained, without truncating or
failing the original calculation. It remains subject to the original replay
policy on restart. Old entries are not evicted to make space. Oversize output
cannot replace an already retained entry. Budget omission is not a guarantee
that a completed calculation will never repeat.

Completed, failed and cancelled transitions prune checkpoint payloads together
with the execution envelope. Recovery-blocked jobs keep them until explicit
job/result deletion. Database pages remain governed by existing maintenance;
logical pruning is not immediate filesystem compaction. Public recovery exposes
only the read-only policy and retained count, not output JSON or owner sessions.
These are per-parent bounds, not total process memory or global disk limits.

## Actual Process Interruption

Five fresh-solve rounds cover thermal, electrostatic, magnetostatic,
thermomechanical and Stokes quad models. Each public graph job runs a real typed
solver and native diagnosis on an owned Rust Agent, then pauses before native
scoring admission through the existing test-only gate. The gate delegates real
Agent calls and never fabricates a scientific receipt or calculation.

At interruption the parent is active in generation one, has one retained solve
entry and one `observed_executed` native diagnosis record. BEAM is killed while
the score has no journal reservation or RPC. SQLite, the journal, configured
endpoint identities and the production 15-second ownership lease are retained.
The launcher configures the pool before startup recovery, not after it.

Generation two reuses the durable solve artifact, reconstructs the original
diagnosis TaskIR and retrieves its original receipt, then executes the unfinished
score. The original dispatch file remains byte-identical. The same parent
completes with an unchanged envelope digest; a public Rust SDK wait succeeds,
diagnostic/quality outputs agree with Engine reducers and the independent branch
retains its result. Terminal recovery has zero retained solve entries.

The five fresh-solve rounds require exactly fifteen Agent executions:
one solve, one diagnosis and one score per parent. Neither completed solve nor
diagnosis executes twice. This qualifies recovery consistency on small physical
fixtures, not independent numerical accuracy, convergence or large-model speed.
The existing frozen-result receipt-recovery and Agent-cache-loss cases retain
their own scopes and are not counted as new physical studies.

## Controlled Regression Scope

Fifty-seven new controlled regressions cover closed fields, resealed invalid
entries, changed node/input/authority identity, tiny-number distinctions, signed
zero, numeric representation, conflict/duplicate behavior, count/byte/JSON-node
limits, private public summaries and terminal pruning. Store cases fence stale
generation/attempt/session, lease takeover and terminal jobs. Six application/
runner-loss scenarios reuse a completed sibling, reject damaged evidence, preserve
side-effect replay restrictions, omit oversized output and avoid restoring a
failed skipped solve as success. These runtime clients are controlled fixtures,
not independent physical solvers.

Four new SQLite fault cases reject either the job or result write for checkpoint
retention and blocked transitions. Two JSON/memory fault cases reject the shared
snapshot write. Both preserve the old pair and permit a later retry; no partially
committed checkpoint is acknowledged. An additional mixed guard case retains a
valid successful solve checkpoint while an original native outcome is unknown;
it still blocks before claim advancement and leaves the dispatch file untouched.

## Verification

The expanded SQLite selection passed 552 executed tests with seven
JSON/memory-exclusive cases skipped (559 listed), in 27.6 seconds. The separate
JSON/memory selection passed all 171 tests in 2.2 seconds, including shared-write
rollback cases. The selections overlap and are not summed. Malformed checkpoint
buckets remain safely inspectable without being accepted as empty/valid data.

The focused real-process fresh-solve test passed its five rounds in 83.14 seconds.
The complete Headless live suite passed all 72 cases in 432.65 seconds, including
the fresh-solve rounds, original-receipt recovery and lost-Agent-cache denial.
Other suite cases retain their actual-process, controlled-fixture and injected-
fault scopes; 72 cases are not 72 independent scientific studies.

All-target CLI Clippy passed with warnings denied. Rust formatting, changed-file
Elixir formatting and diff whitespace checks passed. Native topology, module/
function matrix, extension standard, runtime API, coverage tensor/self-test,
documentation inventory/book, version-line and organization checks passed.
The version audit checked 224 exact contracts with zero mismatches. Sources stay
within 800 lines and docs within 2000; the SDK manual remains 1999 newline-
delimited lines (2000 under the audit's counting rule). Test scratch state is
removed and owned live processes clean up their temporary databases/journals.

The new tensor claim is limited to contract/recovery evidence. Acceptance remains
unchanged: zero structural gaps, four maturity gaps, 19 evidence-grade gaps and
14 P0 gaps, with Daji acceptance blocked. No numerical, scale, performance,
installed or remote cell is promoted by this follow-up.

## Remaining Limits

This is bounded successful-solve artifact reuse, not general durable per-node
continuation, whole-graph exactly-once execution or permission to replay unknown
work. A typed solver interrupted before checkpoint acknowledgement still follows
the original replay policy. Local transforms/conditions/exports are reconstructed;
failed/skipped node state and large solve outputs are not durable artifacts here.
Native results still depend on original receipt availability and cannot be
substituted after Agent-cache loss.

Digest binding is integrity checking, not cryptographic producer authentication
against a privileged database editor. Runtime/solver upgrade compatibility,
external checkpoint import, PostgreSQL failover, arbitrary parallel graph races,
power loss, installed/remote operation and scale/performance qualification remain
separate work. No storage migration, runtime package, version bump, Git operation,
desktop installation or remote benchmark is performed in this follow-up.
