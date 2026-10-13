# Single-Agent Workbench durable run closure

Version: daji 3.6.0. Source-tree validation, no release installation or version change.

## Implemented

- Exactly one direct endpoint routes the ordinary Run action and PWDT `job/run` to
  asynchronous durable admission, not the legacy synchronous solve/cache. The Agent
  must advertise an enabled execution ledger. No uncertain submission is retried or
  sent to another Agent. Multi-endpoint mesh selection is unchanged.
- Caller identities are committed before dispatch. Native desktop storage is a fixed,
  private, locked metadata journal (64 entries / 64 KiB); browser mode uses a separate
  IndexedDB journal. No input/result snapshots or credentials are stored. Native host
  failure cannot silently downgrade durability. One unresolved gateway/endpoint
  reservation blocks new IDs, including from another view.
- Automatic observation is bounded. Completed results can populate the current run;
  changed observation ownership cannot. A changed receipt producer/digest/request is
  rejected. Query errors retain the original handle without resubmitting.
- Ordinary Cancel uses the durable handle and freshly inspected process/digest, not
  Orchestra cancellation. Registration is not terminal confirmation and does not
  release the observation token or clear results.
- System > Runtime > Direct Execution Recovery offers local pending identities after
  reload, through `directExecution/pending`. Matching terminal inspection can retire
  the exact caller reservation without changing the model/result or deleting the
  Agent receipt. Unknown/not-found records remain pending.
- Desktop-managed Agent startup derives separate directories under runtime state,
  configurable with `KYUUBIKI_AGENT_EXECUTION_STORE_ROOT`, and reports the root in
  runtime status. Existing processes are not adopted or silently restarted.

## Verification

- Frontend regression: 179 passing tests, including replaced receipt identity and
  endpoint admission checks.
- Native desktop-runtime: 74 passed, one existing whole-stack test ignored. Includes
  real child-process force termination after journal synchronization, reopening,
  duplicate admission prevention, lock release, capacity, corruption and symlink tests.
- Shared desktop bridges: eight passing tests, including origin/source fencing,
  metadata-only discovery, exact command allowlisting and missing-host fail-closed.
- Chromium: four passing scenarios, including ordinary run completion and lost reply
  followed by page reload, duplicate blocking and explicit terminal reconciliation.
  These exercise actual source GUI/PWDT routing and browser storage with mocked HTTP,
  not a real Agent or loaded Python interpreter in the browser.
- The first browser run failed because the new fixture omitted required runtime
  descriptor fields. The fixture now matches the Agent descriptor contract; rerun passed.
- Native strict unknown-field regression caught serde unit-variant acceptance of extra
  list fields; the request uses an empty struct variant and now rejects them.
- Workbench macOS Tauri `cargo check --offline`, native Clippy with warnings denied,
  frontend typecheck, shared-asset synchronization, documentation inventory/book,
  and 800/2000 source/document organization checks passed. No full app rebuild/install.
- Browser recovery and bridge tests are registered in the native test runner's UI
  suites; the browser entry is also in the desktop UI validation contract.
- Tensor structure passed with zero structural gaps. Global maturity/evidence gaps
  remain (four maturity gaps, twenty evidence-grade gaps, fifteen P0 gaps); this
  bounded source evidence does not relabel the entire system as operationally proven.

## Remaining Boundaries

Not qualified: physical power loss, browser storage eviction, installed WebView through
the full live Agent stack, remote hosts, full durable workflow graphs or multi-Agent
durable selection. Tests do not certify numerical accuracy beyond existing solver tests.
The raw Agent listener still needs a trusted network boundary. No automatic replay,
mid-solver continuation, arbitrary journal deletion control or operational-grade
tensor promotion is introduced. Clearing an unknown reservation is not offered as a
normal retry action. Native caller identities store gateway-address hashes, not auth
tokens; server authorization is still required for every read/cancel.
