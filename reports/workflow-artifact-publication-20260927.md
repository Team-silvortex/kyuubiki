# Workflow Artifact Publication Contract

Date: 2026-09-27

This is bounded local output-publication evidence, not distributed recovery or whole-runtime memory qualification.

## Findings And Fixes

After structural preflight was added, the Elixir runner still published every
successful callback value without an output budget. A valid input could expand
into an oversized JSON report, or a callback could return a PID, invalid UTF-8
or other non-JSON data that reached consumers and failed later serialization.
Unexpected callback return envelopes also escaped as `CaseClauseError` rather
than following the configured node-failure policy.

The first six targeted tests all failed before the fix. Four covered the
solve/transform/extract/export paths, one fail-fast/healthy replay, and one the
callback-envelope crash. Later native-list testing separately reproduced an
`ArgumentError` from `length/1` on an improper graph list.

- A single publication helper checks each successful operator value before
  inserting any output port or lineage. No partial output is published.
- Output budgets match the tested Rust contract: 500,000 JSON values, depth
  64, string length 500,000 bytes, key length 256 bytes and no NUL. Native
  non-JSON values, invalid UTF-8 and improper lists are rejected without
  serializing the entire value into another buffer.
- Validation also runs when an operator has zero declared output ports.
  It is once per callback result, not once per output port. Valid results are
  retained unchanged, not truncated to fit.
- A callback result other than `{:ok, value}` or `{:error, reason}` becomes
  `invalid_operator_result` without logging the arbitrary returned value.
  Normal node fail/skip policy applies; exceptions, exits and cancellation
  are not caught by this new helper.
- The JSON walker and direct graph list-size gate reject improper lists.
  The graph check stops at its size boundary rather than traversing an
  unbounded list merely to count it.

## Tests And Reproduction

`tests/fixtures/workflow-artifact-contract.json` contains a common graph and
21 small output-generation recipes. Boundary-sized arrays and strings are
generated in memory, not committed as large fixtures. Recipes cover null,
false, finite numbers, empty/nested collections, byte-sized Unicode strings
and keys, exact/overflow node counts and depth, NUL and late corrupt leaves.

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-artifact-publication-contract --out tmp/workflow-artifact-publication-contract.json
```

The native profile runs storage-independent Elixir tests and the Rust contract
suite. Elixir injects invalid outputs into all four callback kinds, checks both
node orders and full/compact results, and verifies that consumer callbacks do
not run after rejection. Malformed envelopes are checked across all four kinds
under fail and skip policies. Arbitrary callback operators remain supported.

Rust exercises the shared recipes plus the common graph using real
`transform.first_available` and `export.summary_json` operations, both failure
policies, node orders, healthy/oversized controls and all three artifact
projections (48 graph executions). A separate control preserves large raw
input through a condition and output node in both runtimes.

For API/lifecycle checks, set `SQLITE_DATABASE_PATH` to a fresh isolated
temporary database, then run from `apps/web`:

```text
mix test test/kyuubiki_web/api/workflow_artifact_publication_api_test.exs
mix test test/kyuubiki_web/workflow*_test.exs test/kyuubiki_web/api/*workflow*test.exs test/kyuubiki_web/orchestra/workflow*_test.exs test/kyuubiki_web/orchestra/engine_test.exs test/kyuubiki_web/orchestra/lease_store_test.exs
```

API tests use actual JSON export expansion: a valid 250,000-byte newline string
becomes an over-budget escaped report. Fail-fast returns an error or a failed
async job with no published artifacts. Explicit skip preserves raw evidence,
records the producer failure and skips its two blocked descendants. Full and
compact responses keep failure receipts; SQLite retains honest progress and
the failed result state. A fresh healthy job produces a decodable report.

## Executed Results

| Check | Result |
| --- | --- |
| Native output-publication profile | 33 Elixir tests and 3 Rust suites passed |
| Shared output budget recipes | 21 cases passed in each runtime |
| Broad Elixir workflow, API, engine and lease regression | 483 passed |
| Rust engine library | 635 passed, one pre-existing ignored test |
| Rust release output-publication suite | 3 passed |
| Elixir test compilation | Passed with warnings treated as errors |
| Rust engine all-target Clippy | Passed with warnings treated as errors |
| Operator profile structure and self-test | 57 profiles passed; only the focused profile was executed here |
| Coverage tensor and self-test | Structurally passed; four maturity, 16 evidence-grade and 11 P0 gaps remain |
| Organization audit | Passed; source limit 800, docs limit 2000, tracked debt zero |

Counts overlap and are not an overall coverage percentage. The earlier
483-test run includes the 33 profile tests, two new API tests and the native
improper-graph-list regression, alongside previous workflow tests. The tensor
records local `verified` output-publication evidence without promoting broad
maturity; its readiness assessment remains blocked. The isolated database is
removed after all test VMs exit; no developer data is migrated or reset.

## Boundaries

- Input admission still allows strings up to 1,000,000 bytes. Input, condition
  and output nodes forward already admitted data rather than applying the
  stricter generated-output limit, matching Rust. Do not reinterpret this
  profile as a blanket serialized-response-size limit.
- Checks run after callbacks return. They do not prevent expensive allocation
  or computation inside a callback, bound aggregate retained artifacts, or
  replace process/resource isolation and transport body limits.
- Atomic publication is an in-memory runner property, not rollback of external
  writes or side effects performed by an operator. The implementation cannot
  infer numerical validity from JSON validity.
- API tests use local Plug and isolated SQLite. Remote Agent, installed GUI,
  distributed process-loss recovery, and full decoder/exception parity are
  not qualified here. Elixir pre-normalization of arbitrary native SDK terms
  is a separate boundary from the direct validators tested here.
- No release build, deployment, version bump, commit or push is included.
  Developer databases are not touched; temporary test databases are disposable.
