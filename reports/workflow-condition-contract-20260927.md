# Workflow Condition Decision Contract

Date: 2026-09-27. Source base: `b0148c62` plus this reliability follow-up.

This is bounded local condition-decision evidence, not whole-runtime equivalence or distributed recovery qualification.

## Findings And Repairs

The Elixir condition path walker converted an invalid array index to `-1` and
passed negative indexes to `Enum.at/2`. Both could select the last array entry
instead of the missing value returned by Rust. A research gate could therefore
choose different branches on the same input. Malformed predicate objects raised
`BadMapError`; non-object configuration and non-string paths could silently
evaluate the whole input. Rust also defaulted malformed predicate fields.

Rust numeric ordering converted both operands to `f64`, losing integer
distinctions above `2^53`. Numeric equality and array membership did not share
one cross-runtime rule: `1` and `1.0` could compare differently depending on
operation and runtime. Invalid string-containment operands could become a
successful false decision rather than a configuration error.

The initial focused campaign, after correcting its fixture to use an identity
consumer that accepts arbitrary JSON, had 22 failures in 61 Elixir tests and
both Rust graph suites failed. These include diagnostic-contract failures, not
22 distinct production defects. Four additional mixed-number cases were then
added. The existing five API condition tests are retained byte-for-byte; the
four new API cases live in a separate contract test file.

- Condition evaluation is isolated from graph scheduling in small Rust and
  Elixir modules. The engine still accepts decoupled operator implementations.
- Config, predicate, operator and path types produce bounded node errors.
  Unsupported operators retain the existing Rust diagnostic phrase.
- Array paths accept non-negative decimal indexes, including the existing
  leading-plus behavior. Negative, malformed and out-of-range indexes yield
  JSON null, never an element from the end. Object keys such as `-1` still work.
  Elixir accumulates at most a `u64` index, rejecting overflow before constructing
  an unbounded integer; generated 100,000-character paths test this boundary
  while retaining valid long leading-zero indexes.
- Integers are compared without conversion to floats. Mixed comparisons retain
  the integer magnitude and compare against the float's integer/fractional
  parts, including signed/unsigned limits and negative zero.
- JSON equality and array containment share recursive numeric equivalence;
  booleans remain distinct from numbers. Elixir list comparison avoids building
  a second zipped list. Rust no longer clones the whole predicate map.
- Fail-fast errors remain errors. Explicit skip records the failed condition,
  skips blocked consumers, retains independent raw evidence, and publishes no
  branch artifact, decision or lineage for the failed node. Cancellation and
  unrelated exception/exit propagation are unchanged.

## Reproduction And Results

From the repository root:

```text
./scripts/kyuubiki check-operator-validation --execute --profile workflow-condition-contract --out tmp/workflow-condition-contract.json
```

The profile runs storage-independent Elixir graph tests and Rust graph/oracle
tests. `tests/fixtures/workflow-condition-contract.json` contains 64 shared
cases; both runtimes execute each in normal and reversed node order. Invalid
map configurations are also exercised under explicit skip. The fixture covers
all condition operators, mixed nested JSON equality, path boundaries, malformed
configuration, large integers and mixed numeric boundaries.

The two Rust oracle tests check 100 signed/unsigned integer pairs against
`i128` ordering and 33,410 directed integer/quarter-float comparisons against
exact integer arithmetic. This is bounded enumeration, not a general formal
proof of all JSON or numeric representations.

| Executed check | Result |
| --- | --- |
| Rust Engine, `cargo test --locked -p kyuubiki-engine` | 716 passed, 0 failed, 1 existing ignored test |
| Elixir, complete `mix test` with isolated SQLite | 943 tests, 0 failures, 8 existing skipped tests |
| New API contract | 4 passed, included in the complete Elixir run |
| Shared condition fixtures | 64 cases passed in both runtimes; counts overlap the suites above |
| Native condition profile | 94 Elixir tests, 2 Rust graph suites and 4 filtered Rust unit tests passed; includes existing recovery/condition controls |
| Strict checks | Engine all-target Clippy and Elixir test compilation passed with warnings treated as errors |
| Tensor/profile/organization structure | Passed; 58 profiles registered, source 800 lines and docs 2000 lines respected |

For the API/full Elixir run, set `SQLITE_DATABASE_PATH` to a disposable test
database and `KYUUBIKI_STORAGE_BACKEND=sqlite`. The new endpoint tests are in
`apps/web/test/kyuubiki_web/api/workflow_condition_contract_api_test.exs`. They
exercise HTTP decisions and 422 errors, full/compact async failure receipts,
terminal progress, SQLite retention, and corrected fresh-job execution without
altering the previous failed result. Installed developer data is not reset.

## Contract Boundaries

- Missing paths still resolve to null, including invalid array indexes. An
  explicit `falsy` or `eq null` can therefore return true. This is not an
  existence-check policy. Empty path segments retain the existing behavior.
- A missing operator still defaults to `gt`. Missing numeric operands now fail
  consistently; implicit truthiness of malformed/null config is not retained.
- Numeric evidence covers representable `i64`, `u64` and finite binary64
  values, not arbitrary-precision JSON numbers or exact decimal arithmetic.
  Equality is exact numeric equivalence, not a physical tolerance comparison.
- Validation occurs when the condition executes. It does not roll back earlier
  operator side effects, prevent all callback exceptions, or bound aggregate
  retained artifacts. Existing graph/input/output budgets remain unchanged.
- API evidence is local Plug plus SQLite. No remote Agent, process-loss replay,
  installed GUI, PostgreSQL, release build, deployment, commit or push is part
  of this follow-up. Tensor evidence is local `verified`, not `qualified`.
