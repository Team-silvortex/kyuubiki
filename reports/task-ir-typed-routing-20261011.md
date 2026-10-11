# Typed TaskIR capability routing and original receipt recovery

Source-only capability routing is verified for explicitly selected
`kyuubiki.task-digest/typed-ieee754-v2` tasks. Orchestra derives the encoding
requirement without rewriting the signed task. Old, unprofiled, role-only and
method-missing peers cannot receive a typed task through the routed client.
Legacy tasks retain existing routing. Default encoding is not changed.

## Implementation

- Rust Agent descriptors advertise the exact encoding ID, role `task_digest`,
  and both task admission and read-only result retrieval methods.
- `OperatorTaskIR.required_agent_capabilities/1` adds the encoding requirement
  alongside the task's runtime capabilities and removes duplicate IDs.
- The dedicated TaskIR client derives routing from the task; the generic RPC
  entry point cannot erase typed requirements through caller options.
- `AgentTaskRouting` owns the existing TaskIR retry and package-host routing
  policy, extracted unchanged from the size-bound TCP client.
- `AgentPoolRouter` requires an explicit capability ID and declared RPC method,
  not a role-name substitute or unknown-metadata fallback.
- HTTP execution returns `no_matching_agent` before dispatch if no candidate
  supports the requirement. A failed capable endpoint does not downgrade to an
  old peer. Advertisements are compatibility metadata, not authentication.
- No new database tables, snapshots or encoded payload copies are retained.
  Dispatch journal and bounded Agent result retention remain the same contracts.

## Actual process chain

The new live test runs one source Rust Agent, real HTTP Orchestra servers and
genuine Rust/Elixir task authors. Descriptor-aware test endpoints read the
actual `describe_agent` RPC rather than inventing capability metadata. Static
unknown endpoints remain a separate negative control.

1. HTTP execution against an unprofiled pool is rejected. The Agent's started
   execution counter remains zero.
2. Ten tasks execute through HTTP: two authors across thermal, electrostatic,
   magnetostatic, thermomechanical and Stokes diagnostics. Outputs match direct
   engine extraction of the same freshly solved inputs. This is execution-path
   consistency, not independent numerical accuracy.
3. Each task has one `observed_executed` dispatch attempt. The counter is ten.
4. A real Orchestra process restart preserves its journal. All ten original
   Agent receipts are recovered through HTTP, retaining task digest and output.
   Retrieval grants no automatic replay and publishes no result.
5. Ten tiny configuration changes, still aliases under fixed-15 hashing, are
   rejected by the restarted HTTP service. The execution counter stays ten.
6. A real Agent restart loses memory-only results. All ten queries return
   unknown outcomes with no completion, replay permission or publication.
   The new Agent execution counter remains zero.

This is bounded read-only receipt recovery, not automatic workflow continuation
or exactly-once computation. Agent restart cannot make unknown output safe to
reuse. The test owns and removes its temporary services and data directories.

## Verification

- Focused Elixir digest/routing/pool/client/API selection: 65 tests passed.
- Expanded SQLite recovery/dispatch selection: 775 listed, 766 passed,
  9 JSON-only skipped, zero failures.
- Isolated JSON recovery/dispatch selection: 478 tests passed.
- Rust protocol library: 140 tests passed, including descriptor advertisement.
- Rust Headless SDK library: 534 tests passed. Agent TaskIR RPC selection:
  33 tests passed.
- Both source typed identity live tests: 2 passed; the new HTTP dispatch/restart
  test also passed independently before the combined run.
- Protocol, CLI, Headless SDK and native script-runner all-target Clippy checks
  passed with warnings denied.
- After routing-policy extraction, both live tests, the 775-case SQLite and
  478-case JSON selections passed again. A 53-case client/pool/routing/process-loss
  selection also passed, retaining the existing checkpoint retry controls.
- Test-environment Elixir compilation with warnings as errors, both format
  checks and `git diff --check` passed.
- Module topology, function matrix, extension standard, runtime API contract,
  TaskIR contract/self-test, documentation inventory/book and version audit
  passed. The organization audit reports zero debt at 800 source / 2000 doc
  lines. The coverage tensor and its self-test pass structurally; its state
  remains 0 structural, 4 maturity, 20 evidence-grade and 15 P0 gaps, with Daji
  qualification blocked.

Reproduce the focused checks with:

```text
cd apps/web
mix test test/kyuubiki_web/playground/task_digest_routing_test.exs test/kyuubiki_web/playground/agent_client_test.exs test/kyuubiki_web/playground/agent_pool_test.exs test/kyuubiki_web/orchestra/task_digest_v2_test.exs test/kyuubiki_web/api/task_digest_v2_api_test.exs
cd ../../workers/rust
cargo test --offline -p kyuubiki-protocol --lib
cargo test --offline -p kyuubiki-cli --test headless_live typed_task_identity -- --nocapture
```

## Remaining scope

Installed typed workflow continuation remains open. Separate
[workflow recovery evidence](workflow-typed-execution-recovery-20261011.md)
now covers bounded source native continuation. The native workflow builder still
uses legacy TaskIR plus its exact invocation guard by default. These tests do not
qualify arbitrary executable package hosts, installed mixed-version binaries,
remote mesh deployments, all database backends or SDK distribution packages.
No old task or receipt is automatically resealed. Source capability routing and
original-receipt recovery are verified claims in the coverage tensor, not an
upgrade to all-author, installed or operational qualification.
