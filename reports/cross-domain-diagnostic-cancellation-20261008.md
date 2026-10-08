# Cross Domain Diagnostic Cancellation

Date: 2026-10-08. Source macOS ARM64 Engine verification extends
[diagnostic cancellation](diagnostic-cancellation-reliability-20261008.md)
to Stokes result scans and cross-domain workflow publication guards. The repair
uses the existing execution-control scope; numerical algorithms, diagnostic
metrics, last-record peak ties and quality defaults are unchanged.

## Reproduced Gap and Repair

Five of the initial seven integration regressions failed before the repair.
Stokes node and element loops had no safe points, so cancellation observers
never ran: late corrupt records were read, a healthy CFD graph published a
decision, and a real one-element result completed instead of cancelling.
Pending cancellation before scope entry and shared-domain vector cancellation
already passed. A thermo stress-component fallback regression was then added.

Both Stokes loops now call the existing `checkpoint_diagnostics` helper at
entry, each 64 successfully consumed records and the final record. Scan counts
restart at the element pass. The borrowed streaming reductions, signed means,
peak identities and dissipation totals are unchanged; no result-array copies,
second watchdog or domain-specific control token are introduced.

## Tested Boundaries

The eight new integration tests cover:

- CFD node cancellation before a corrupt pressure record at index 100.
- CFD element cancellation after complete node statistics, before a corrupt
  dissipation record at index 100; no partial result returns.
- Both loop cadences for record counts 1, 63, 64, 65 and 129, with no duplicate
  chunk/final safe points and cancellation at short final records.
- Pending cancellation before scope entry, original malformed-record errors
  in a fresh scope, and healthy execution without leaked cancellation state.
- Thermal, electrostatic, magnetostatic and thermo-mechanical vector scans
  cancelled after earlier scalar groups; healthy and corrupt fresh calls keep
  their respective result/error semantics.
- Thermo stress-component fallback cancelled during the xy scan after x/y
  extrema were accumulated; a fresh complete scan preserves signed peak -5.
- Five Engine diagnostic -> quality -> objective -> decision graphs reject
  cancellation, including graphs configured with `on_error: skip`.
- A real single-quad Stokes solve/diagnose/quality graph supplies a four-node,
  one-element result; cancellation at the final element rejects the diagnostic
  summary, then an explicit reduction rerun agrees with the retained raw fields.

For the error-policy test, the inner Engine trace is captured solely for
inspection: dependent quality/objective/decision nodes are skipped and no
diagnostic summary is present. Its raw/independent artifacts never become a
successful cancelled-call return because the enclosing control scope rejects
publication. A fresh complete graph recovers without changing that failed trace.
This is not a public API for salvaging or continuing a cancelled graph.

## Verification

All runs used the current source on macOS ARM64 and offline Rust dependencies.

| Suite | Result |
| --- | --- |
| Engine library | 649 passed, 1 existing ignored |
| New cross-domain cancellation integration | 8 passed |
| Domain diagnostic integrity workflow | 5 passed |
| CFD/dynamic metric integrity workflow | 3 passed |
| Thermo/transport integrity workflow | 7 passed |
| Transport stabilization workflow | 2 passed |
| Targeted native transport diagnostic cancellation service chain | 1 passed, 60 filtered |
| Stokes component validation profile | 7 commands, 25 tests passed |
| CFD release-candidate validation profile | 6 commands, 24 tests passed |

The five listed Engine integration targets contain 25 tests altogether. Profile
counts overlap with those targets and with each other; they are not summed into
unique coverage. The ignored Engine test still requires a separately built
operator-template dynamic library. Dynamic plugin loading is not qualified here.

Strict all-target Clippy passed with warnings denied for Engine, Solver, CLI
and Headless SDK after repairing one test-helper style warning. Formatting and
diff whitespace checks passed. Native topology, matrix, extension standard,
runtime API surface, documentation inventory/book, organization and version
checks passed. Source and document limits remain 800 and 2000 lines.

Both CFD profiles now include this integration target. Their generated receipts
are retained only under ignored `tmp/` and pass native report validation.
The tensor and its self-test pass structurally: zero structural gaps, four
maturity gaps, 19 evidence-grade gaps and 14 P0 gaps remain, with Daji acceptance
blocked. The new Engine recovery evidence does not change historical grades,
numerical qualification or installed-platform acceptance.

## Scope

Synthetic record fixtures are not physical models or scale benchmarks.
The real fixture covers one quad, not general CFD accuracy or Navier-Stokes.
Polling limits consumed-record cadence, not wall-clock latency. JSON decoding,
cloning, sorting, serialization, bridge mappings, legacy peak extractors and
dynamic plugins remain outside these safe-point claims.

The cross-domain graph runs use the synchronous Engine API, not non-transport
TaskIR routing through an Agent, public Orchestra job cancellation or installed
remote deployment. The separately rerun native transport cancellation fixture
only protects its existing service scope. Durable restart, numerical continuation
and historical industrial qualification are unchanged.

The subsequent [native service follow-up](native-domain-diagnostic-chains-20261008.md)
separately extends routing and standalone diagnostic cancellation to the five
additional domains. It does not change this report's Engine-only evidence scope.
