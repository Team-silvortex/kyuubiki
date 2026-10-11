# Bounded workflow branch recovery

Date: 2026-10-11. Source baseline: `6d9e1094` (`daji 3.5.6`);
package metadata remains `3.5.0`. No release, commit, installation or remote
qualification is implied by these source checks.

This follow-up preserves successful condition choices across owned workflow
recovery. Replayed local ancestors cannot silently select another route when
their output differs from the first successful condition invocation. It is
not general durable per-node continuation or permission to repeat an ambiguous
Agent dispatch.

## Decision contract

- Internal schema: `kyuubiki.workflow-branch-checkpoints/v1`, stored under
  `_workflow_recovery.branch_checkpoints`, never accepted from graph request JSON.
- Closed bucket and entry shapes bind the parent job, execution-envelope digest,
  full condition-node digest, lossless payload/context invocation digest and
  producing generation, attempt and owner session.
- Each decision contains only `chosen_output` and boolean `predicate_result`.
  The choice must agree with the node's condition-port selection rules, including
  positional fallback. Predicate evaluation and restored-decision validation use
  the same selector as ordinary graph execution.
- Successful choices commit through the lease-fenced atomic job/result write
  before GraphRunner publishes a port, lineage or completion event. Ordinary
  predicate errors and skipped nodes produce no successful decision record.
- A matching later claim reuses the stored choice without reevaluating the
  predicate. The current payload still supplies the selected artifact; no input
  or output artifact is copied into this ledger. Missing entries retain the
  original replay policy. Same-generation reuse and conflicting writes are denied.
- Changed input/context, corruption, invalid callback results and failed commits
  block recovery, including nodes configured with `on_error: skip`. The blocked
  job retains its envelope and original evidence with next action
  `inspect_branch_checkpoint_before_explicit_resubmit`.
- Corruption detected during admission blocks before advancing the durable claim.
  An initial missing coordinator error classification was found by interruption
  tests and corrected, avoiding a job left waiting after a rejected branch record.
- Original graph replay restrictions and the native ambiguity journal still take
  precedence; a retained branch decision cannot override an ambiguous native dispatch.

## Storage and architecture

Each active parent retains at most 64 decision entries and 65,536 encoded bytes.
Container validation also bounds JSON traversal to 8,192 nodes. Capacity pressure
omits new entries without evicting existing decisions. Omitted conditions still
evaluate under the graph's original authorized replay policy; there is no global
claim that every condition is recorded.

Ordinary `completed`, `failed` and `cancelled` transitions prune the ledger.
`recovery_blocked` retains it until explicit deletion. Public inspection exposes
only count and read-only policy, not payloads or owner sessions, and tolerates
malformed private buckets without treating them as valid recovery evidence.
These are logical per-parent limits, not a whole-process/disk cap or immediate
physical database compaction. No extra files, backups or archival history are added.

Solve-artifact and branch-decision storage share `WorkflowCheckpointStore` rather
than duplicate lease/CAS/transaction logic. Node progress persistence is extracted
to `WorkflowNodeProgressStore` to keep the coordinator under the 800-line source
limit; progress throttling and monotonic replay progress remain in place.

## Controlled interruption and fault evidence

Fifty-one new Elixir cases extend the existing recovery selection:

- Lossless input identity, signed zero and tiny differences; changed configuration,
  port layout and authority; corrupt/resealed records; invalid producer claims;
  entry/byte caps; read-only reuse; private inspection and terminal pruning.
- Current claim and lease fencing, terminal jobs, conflicting writes, blocked
  reason whitelists and active-result immutability.
- True/false branch application restarts preserve the chosen path, reconstruct a
  `first_available` join, and leave the unchosen path skipped. These use a controlled
  typed-solve client, not an independent physical solver.
- An intentionally oversized upstream result is not solve-cached. After restart
  its controlled reexecution changes sign; recovery blocks at the condition before
  the dependent solver runs, despite skip-on-error. Original branch evidence remains.
- Corrupt branch records and explicit side-effect replay restrictions block restart
  before any controlled solver is called. Invalid initial choices cannot be published.
- SQLite job/result write rejection and JSON shared-snapshot failure leave no partial
  decision/blocking transition and allow retry after the injected fault is removed.
- An ambiguous native dispatch still blocks without changing its original journal.
- Invalid condition-executor return shapes become explicit node errors rather than
  crashing the scheduler. Invalid checkpoint evaluation shapes block before commit;
  ordinary predicate errors retain their original error/skip semantics.

The new count includes two JSON-only snapshot cases and four SQLite-only write
cases. SQLite and JSON test totals overlap and must not be added as distinct tests.

## Actual service restart evidence

`real_branch_workflow_restart_retains_route_before_native_receipt_recovery`
submits two public asynchronous graph jobs against actual Rust Agents and a source
Orchestra using retained SQLite storage. Thermal chooses `if_true`; electrostatic
chooses `if_false`. The actual BEAM process is terminated after native diagnosis and
branch commit, while native scoring is paused before admission.

Restarted Orchestra recovers the same parents at generation/attempt two. Each
job retains its envelope identity, restores the small typed-solve artifact,
reads the original native diagnosis receipt without changing its journal bytes,
reuses the branch decision and admits scoring once. The unchosen transform and
output stay skipped, no unchosen artifacts appear, final decision/quality values
match the existing Engine reference, and public SDK waiting reports completion.
Terminal recovery inspection shows both checkpoint counts at zero.

The two branch rounds require exactly six Agent executions. Reference comparison
uses the same Engine implementation, not an independent numerical accuracy oracle.
These are small local source service restarts, not scale/performance measurements,
installed-shell, remote, power-loss or PostgreSQL-failover acceptance.

## Verification

- Expanded SQLite recovery selection: 610 listed, 601 executed passed, nine
  JSON-only cases skipped, zero failures; 28.4 seconds.
- JSON/memory focused selection: 218 executed passed, zero failures; 3.5 seconds.
- Actual branch/BEAM restart case: one test, two real graph rounds, passed;
  33.71 seconds.
- Full actual headless service regression: 73 executed passed, zero failures or
  skips; 545.47 seconds. This includes existing solve/receipt recovery, lost Agent
  receipt denial, cancellation, readback and output-publication chains. The focused
  branch case is included in this total, not an additional unique test.
- CLI all-targets Clippy with warnings denied, test-environment Mix compilation
  with warnings denied, Rust/Elixir formatting and diff whitespace checks passed.
- Module topology, function matrix, extension standard, runtime API surface,
  coverage tensor and tensor self-test, documentation inventory/book, version-line
  and project organization checks passed. Version audit: 224 exact contracts,
  zero mismatches. Source/doc limits remain 800/2,000 lines with no tracked debt.
- Tensor: zero structural gaps, four maturity gaps, 19 evidence-grade gaps and
  14 P0 gaps; Daji acceptance remains blocked.

## Remaining boundaries

Local transforms, extracts, exports, input publication, joins and scheduler sets
are still reconstructed, not restored from a complete continuation snapshot.
Large or uncommitted typed results retain their original replay behavior. Native
receipt loss still blocks; this ledger does not repair unknown native execution.
Unretained conditions have no preserved decision to compare against.

Digest checks detect drift/corruption, not malicious edits by a privileged database
writer. Solver/runtime upgrade compatibility, independent numerical validation,
arbitrary parallel races and broader platform/scale acceptance remain separate work.
Tensor evidence is scoped to contract/recovery; broad maturity and Daji P0 acceptance
must not be promoted merely because these bounded restart cases pass.
