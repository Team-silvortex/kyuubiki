# Typed native workflow execution and restart recovery

Explicit source workflow selection now preserves typed numeric identity from
Rust authoring through HTTP admission, durable execution storage, native TaskIR
dispatch and Orchestra restart. Completed solves, branch decisions and original
native receipts are reused without repeating completed work. Missing Agent
receipts still block continuation. Default encoding remains legacy.

## Implementation

- `defaults.native_task_digest_encoding` selects the native diagnostic/scoring
  TaskIR adapter. Null, unknown and non-string selectors fail graph preflight.
- Rust `WorkflowDefaults` exposes `WorkflowNativeTaskDigestEncoding`; missing
  selection decodes to legacy and serializes without a new default field.
  Shared Rust/Elixir graph fixtures cover explicit typed/legacy and invalid values.
- The engine derives the policy from the graph, not mutable environment settings
  or execution callback options. Native tasks are sealed before summary, routing,
  recovery or dispatch. Solve RPCs and local transforms are unchanged.
- New-codec asynchronous jobs use execution envelope v2 with one canonical
  `execution_json` string. Graph, inputs, context and response options are not
  stored again as a parallel numeric map. The outer recovery SHA-256 binds this
  representation. String values prevent JSONB numeric normalization.
- Typed storage retains integer versus float, signed zero, adjacent binary64
  and tiny numeric values before initial execution and after restart. Legacy
  v1 maps retain their existing normalization. No old record is resealed.
- Recovery requires matching graph policy and public encoding metadata. A typed
  graph cannot be wrapped as a legacy envelope, and a v2 envelope cannot silently
  drop its policy or gain a conflicting alternate payload representation.
- Coordinator dispatch, parent fencing and solve/branch checkpoint access use
  the decoded execution. Each checkpoint operation decodes once and passes that
  execution to entry checks, avoiding one full decode per retained entry.
- Existing claim, lease, dispatch and checkpoint integrity gates remain active.
  Terminal pruning removes the single private execution payload and checkpoints;
  blocked recovery retains them for explicit inspection/deletion.
- Existing graph/input and per-parent checkpoint budgets remain unchanged.
  This does not introduce a total process/disk cap or durable Agent result archive.

## Actual process chains

The owned harness starts two real Rust Agents and real HTTP Orchestra processes,
with a dedicated SQLite database and dispatch directory. Descriptor-aware
endpoints read actual `describe_agent` advertisements. A test-only pause holds
scoring before journal/RPC admission; it does not fabricate computation or receipts.

Five chains freshly solve thermal, electrostatic, magnetostatic,
thermomechanical and Stokes inputs. Each chain reaches solve, native diagnosis,
condition, selected-route join and native scoring. Both branch routes are covered.
Configuration includes integer/float pairs, positive and negative floating zero,
a tiny value and an adjacent binary64 value. Rust independently recomputes the
captured Elixir-authored TaskIR digest.

After the original solve/diagnosis/branch complete, the harness loses the actual
Orchestra process and restarts against the same storage. It checks:

1. Recovery generation and attempt advance from one to two; envelope identity
   remains identical.
2. The successful solve and selected branch restore from private checkpoints;
   the original Agent diagnostic receipt is retrieved without another execution.
3. Local joins rebuild only the selected route; rejected-route outputs stay absent.
4. The resumed scoring digest equals the task held before the process loss.
5. Original dispatch file bytes remain unchanged. Each native task has one
   observed execution attempt, and each solved chain uses three Agent executions,
   not a repeated solve or diagnosis.
6. Completion prunes execution and checkpoint payloads; SDK waiting returns
   the completed result.

A raw thermal-result chain additionally verifies typed input representation,
including the negative-zero bit pattern, before initial native admission.
A separate solved/branched thermal chain restarts the Agent too, losing its
memory-only diagnostic receipt. Recovery blocks before diagnostic publication
or scoring, retains the original dispatch evidence, and starts zero new Agent
executions. Neither solve nor branch checkpoints authorize a missing receipt.

These are seven bounded typed workflow rounds. Existing legacy workflow restart
tests remain controls. Domain outputs match direct extraction/scoring of the same
fresh solver results; this is execution-path consistency, not independent
physical accuracy or production research qualification.

## Verification

- Expanded SQLite recovery/routing/API selection: 791 listed, 782 passed,
  9 JSON-only skipped, zero failures after checkpoint decoding refinement.
- Isolated JSON recovery/routing selection: 494 tests passed again after
  checkpoint decoding refinement.
- Rust protocol library: 141 tests passed, including typed policy serialization.
- Rust Headless SDK library: 534 tests passed.
- Shared Rust engine graph preflight selection: 3 tests passed.
- Initial real typed dispatch/workflow selection: 4 tests passed, covering the
  existing standalone original-receipt test plus all three typed workflow tests.
- Final real legacy/typed workflow recovery selection: 7 tests passed in
  334.03 seconds after checkpoint decoding refinement and numeric capture checks.
  This covers 13 legacy and 7 typed bounded workflow rounds, not 20 large studies.
- Protocol, engine, CLI, Headless SDK and native script-runner all-target Clippy
  passed with warnings denied. Test-environment Elixir strict compilation,
  format checks and `git diff --check` passed.
- Module topology, function matrix, extension standard, runtime API surface,
  TaskIR contract/self-test, documentation inventory/book and version audit
  passed. The tensor/self-test pass with 0 structural, 4 maturity,
  20 evidence-grade and 15 P0 gaps; Daji qualification remains blocked.
- The organization audit retains zero debt at 800 source / 2000 document lines.
  The isolated JSON test directory is removed after verification.

## Qualification boundary

This adds verified source macOS HTTP/Agent/SQLite contract and recovery evidence.
The [PostgreSQL follow-up](workflow-typed-postgres-recovery-20261011.md) adds actual
remote JSONB transactions and source restart rounds; it does not rewrite the
SQLite counts above. All-author default adoption, installed mixed-version peers,
remote mesh, arbitrary executable package hosts and distributed
SDK packages remain separate acceptance work. Memory-only Agent receipt retention
is not made durable; there is no exactly-once guarantee or large-model benchmark.
Version metadata, installed applications and Git history are unchanged.
