# Workflow Graph Preflight Contract

Date: 2026-09-27

This is bounded local graph-admission evidence, not whole-runtime equivalence or distributed execution qualification.

## Scope And Fix

The Elixir runner previously validated recovery policy but not graph structure
before building scheduler indexes. Duplicate IDs could overwrite nodes, cycles
could stall after independent work had already run, and malformed `nodes`
could crash response-mode sizing before the runner returned a contract error.
Async submission also created a persistent job before these problems surfaced.

`WorkflowGraphPreflight` now validates structure before response sizing and
job creation, and again at the runner boundary for direct callback invocation
and recovery replay. There is no caller-controlled bypass flag. The shared
JSON budget walker stops at its depth/node limits instead of serializing a
second JSON copy. The graph traversal checks all components and counts each
incoming edge, including legitimate parallel edges to different input ports.

- Validate schema, identifier character/byte limits, kinds, required operator
  identifiers, unique ports and artifact types before indexing.
- Validate source/target existence, matching types, unique edge IDs and one
  incoming edge per target port. Check declared entry/output kinds and cycles.
- Validate supplied input keys against actual input nodes. Missing values
  remain subject to the existing fail/skip execution policy, not admission.
- Bound graph nodes to 2,048, edges to 4,096, ports per direction to 32;
  configs/metadata to 20,000 JSON values and each input to 500,000 values.
  Limit JSON depth to 64, string bytes to 1,000,000 and key bytes to 256;
  reject NUL strings/keys and non-JSON values at the direct runner boundary.
- Keep operator implementations out of the graph gate. A custom callback
  operator can pass structural validation without entering a built-in catalog.

## Reproduction

The common fixture is `tests/fixtures/workflow-graph-preflight.json`: 51
structural cases run in both node orders, plus 26 generated at/over-budget
cases. Both runtimes consume the same case descriptions. Generated large
arrays are not stored in the repository. Rust typed-decoder rejection is a
valid rejection outcome for malformed wire shapes; diagnostic strings need
not be identical across runtimes.

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-graph-preflight-contract --out tmp/workflow-graph-preflight-contract.json
```

The profile deliberately uses storage-independent Elixir tests. For HTTP/job
regressions, set `SQLITE_DATABASE_PATH` to a fresh temporary database, then run
from `apps/web`:

```text
mix test test/kyuubiki_web/api/workflow_preflight_api_test.exs
mix test test/kyuubiki_web/workflow*_test.exs test/kyuubiki_web/api/*workflow*test.exs test/kyuubiki_web/orchestra/workflow*_test.exs
mix test test/kyuubiki_web/orchestra/engine_test.exs test/kyuubiki_web/orchestra/lease_store_test.exs
```

Never point these store-resetting tests at a developer or installed-runtime
database. The local validation used a new disposable SQLite database; no
development data was migrated or reset.

## Executed Evidence

| Check | Result |
| --- | --- |
| Broad Elixir workflow, engine facade and lease regression | 447 passed, including the new admission/budget/API tests and prior branch recovery |
| Native focused validation profile | 82 Elixir tests and 2 Rust suites passed |
| Built-in template admission | All 71 templates passed without invoking a solver |
| Shared Rust structural and budget suites | 2 passed, covering the 77 shared case descriptions |
| Rust engine library regression | 632 passed, one pre-existing ignored test |
| Elixir test compilation | Passed with warnings treated as errors |
| Rust engine all-target Clippy | Passed with warnings treated as errors |
| Operator profile structure and self-test | Passed for 56 profiles; only the focused profile was executed here |
| Coverage tensor and self-test | Structurally passed; four maturity, 16 evidence-grade and 11 P0 gaps remain |
| Organization audit | Passed: source limit 800, docs limit 2000, tracked debt zero |

The new API tests exercise synchronous and asynchronous rejection for every
negative structural case, budget overflow before queuing, invalid recovery
policy before persistence, and a healthy synchronous/async follow-up. Rejected
requests leave zero job/result rows. Direct-runner tests fail immediately if
an invalid graph invokes an operator or emits progress. Counts overlap and
are not a project-wide coverage percentage.

The standby lease test formerly submitted an empty graph to reach its rollback
assertion. Its fixture now uses a minimal valid output node so admission
succeeds and the original lost-lease/no-residual-job assertion still executes.
No recovery or physics acceptance assertion was weakened. The tensor adds a
scoped local `verified` claim, not a maturity promotion; readiness stays blocked.

## Limits And Next Gaps

- This does not claim full Elixir/Rust decoder parity. Elixir retains optional
  display fields and omitted-array defaults; Rust uses stricter typed wire
  decoding and a built-in supported-operator check. The generic Elixir runner
  delegates operator availability to callbacks. Entry/output reference lists
  are additionally bounded to 2,048 in Elixir.
- Dataset metadata receives a JSON budget, not the full Rust typed contract.
  This round does not add output-publication budgets or a total request-memory
  limit, nor change transport body limits or pre-normalization handling.
- Local Plug/SQLite checks do not establish remote Agent, installed GUI,
  distributed restart or process-loss behavior for this change. No physics
  qualification or tensor maturity promotion is warranted.
- No performance qualification, release packaging, version bump, commit or
  push is included. Temporary test data is disposable.
