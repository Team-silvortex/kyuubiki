# Original Receipt Availability Boundaries

Date: 2026-10-07. Base checkout: `76aaf10d`, with the prior acknowledgement-loss
follow-up preserved. Original-result retrieval now rejects incomplete or
contradictory envelopes, while real cache eviction, endpoint loss and Agent
restart keep the old attempt unknown without automatic execution or publication.
Evidence is source-chain verification, not durable recovery or installed acceptance.

## Receipt Contract Repairs

Test-first regressions reproduced three acceptance gaps: the Rust SDK accepted
an unknown reply missing the required `completion` field, ignored invalid optional
execution identity on unknown replies, and Orchestra accepted an incomplete
native envelope as a legitimate unavailable result. These replies did not pass
as successful computation, but violated the existing v1 formats and could conceal
malformed transport or an incompatible Agent.

The Rust SDK now requires all nine public-envelope fields and rejects keys
outside the nine required plus three optional incarnation fields. Optional
request/process IDs must be bounded strings, process ID cannot be `unavailable`,
and an optional generation must be a positive `u64`; recovered receipts require
all three. All nine unavailable statuses must retain unknown outcome, explicit
null completion and false replay/publication flags. Valid optional identity
metadata is still accepted according to the existing contract.

Orchestra now requires all 13 native-envelope fields, valid UTF-8 process identity,
null or positive `u64`-bounded generation, and the exact fixed retention policy.
The policy constants are read from `agent-task-result-retention.schema.json`
at compilation with an external-resource dependency, not from a runtime path.
Missing `response` no longer counts as explicit null. Bad policy, extra fields,
invalid generation and malformed process identity become `agent_receipt_invalid`
with unknown outcome and no completion. Existing task-bound completion/failure
gates remain unchanged. This is application validation of the fixed envelope,
not a general JSON Schema engine, authentication or cryptographic provenance.

Changed production files:

- `workers/rust/crates/headless-sdk/src/service_executor_dispatch_result.rs`
- `apps/web/lib/kyuubiki_web/orchestra/operator_dispatch_result.ex`

## Real Availability Loss Chains

`real_count_eviction_keeps_original_unknown_without_touching_peer_or_downstream`
starts a real TaskIR bar computation through Rust SDK and Orchestra. A test relay
loses its actual terminal transport reply; a separate query first recovers the
original result and checks the independent reference `FL/EA`. The test explicitly
admits 64 fresh correlated native computations, also checked against that
reference, to exercise the real 64-entry cap. Reading the oldest receipt just
before the last admission does not promote its reservation order. It is then
`not_retained`, not cancelled, completed or replayable.

Repeated original-result reads leave one unknown journal attempt, unchanged
journal bytes and project listing, and zero peer executions. The owner's 65
executions are exactly the original plus 64 explicit pressure admissions. Only
an explicitly requested rerun adds the 66th execution and a new journal attempt;
its separately recovered healthy result has a newer generation. The old receipt
stays unavailable. Direct pressure admissions do not fabricate journal records,
override cache policy or represent automatic recovery.

`real_endpoint_removal_and_agent_restart_never_substitute_the_original_receipt`
also begins with an actual computed, separately recovered original receipt.
Stopping its owned Agent yields `original_endpoint_unreachable`; restarting only
Orchestra with the peer configured instead yields `original_endpoint_not_configured`
without querying the old relay or executing on the peer. Restarting the same
owned Agent and restoring its endpoint yields `not_retained` because its cache
is empty. These stages keep the original journal bytes and unknown attempt,
no project creation and no automatic execution. A fresh explicit attempt can
return a healthy result with a different process instance and request ID, but
cannot replace the old attempt's absent receipt.

Two new native Store tests separately check reservation-origin expiry: repeated
reads do not renew TTL or eviction order, and a computation finishing at the
600-second boundary cannot resurrect its expired reservation. These use explicit
monotonic test instants, not a production clock override or a ten-minute wall-time
experiment. Existing count/byte/oversize, ambiguity and descriptor-policy tests
remain in the seven-test cache suite.

## Verification

Final macOS ARM64 source verification:

- Complete `headless_live`: 23/23 passed twice, in 17.60 and 25.60 seconds,
  including the previous three acknowledgement-loss cases and both new chains.
- Native `kyuubiki-cli` binary unit suite: 182/182 passed in 0.80 seconds.
- Complete Rust Headless SDK: 308 unit tests in 31.35 seconds and 25 integration
  tests passed. Protocol: 114 unit tests in 0.48 seconds and seven integrations
  passed. No ignored or filtered cases in these complete targets.
- Related Orchestra cancellation/result/journal/API/legacy-routing suite:
  47/47 passed in 4.6 seconds with warnings denied.
- CLI/SDK/Protocol test-target Clippy passed with warnings denied; formatting,
  contract/API, topology, documentation and organization checks passed.

Remote Ubuntu x86_64 used a disposable, unprivileged Docker source-test container
with Rust 1.88.0, Elixir 1.19.5/OTP 28, 8 CPUs, 8 GiB memory, 512 PIDs, four Rust
test threads and two Erlang schedulers. No installed service was replaced or
restarted. Final results:

- Complete live source chain: 23/23 passed in 19.36 seconds, after a 28.15-second
  cold test build. This is correctness evidence, not a performance benchmark.
- The same selected Web suite: 47/47 passed in 2.2 seconds. An earlier attempt
  stopped before tests because five public root-level workflow fixtures were
  missing from the temporary copy; those explicit files were then supplied.
- SDK result-validation subset: 5/5 passed; 303 other SDK tests were filtered.
  Native cache subset: 7/7 passed in 0.19 seconds; 175 other binary tests filtered.
  These subsets are not a claim of full remote SDK or native unit coverage.
- SHA-256 matched for 13 selected production/test/fixture-support files, schemas
  and lockfiles between final local and remote copies. Lockfiles are unchanged.

Commands from `workers/rust`:

```text
cargo test --locked --offline -p kyuubiki-cli --test headless_live
cargo test --locked --offline -p kyuubiki-cli --bin kyuubiki-cli
cargo test --locked --offline -p kyuubiki-headless-sdk -p kyuubiki-protocol
cargo clippy --locked --offline -p kyuubiki-cli -p kyuubiki-headless-sdk -p kyuubiki-protocol --tests -- -D warnings
```

Remote live testing adds `-- --test-threads=4`. Sources and fixtures are temporary;
addresses, credentials and deployment configuration are not retained in this report.

## Qualification Limits

Safe unknown after restart is not successful recovery of the original result.
No durable cache, replay policy, cancellation-intent store, initial-process
attestation, installed App test, authenticated multi-host deployment or additional
Python/Elixir SDK parity is introduced. Numerical checks qualify only the tested
bar reference, not arbitrary materials or solver families. The broader standalone
Web and all-package CLI integration suites were not run. Existing API versions,
solver/engine separation and bounded in-memory retention remain unchanged.

The tensor remains verified execution/contract/recovery evidence: four maturity
gaps, 19 evidence-grade gaps, 14 P0 qualification gaps and blocked release status.
No version bump, commit, push, App rebuild or installed-service restart occurred.

The disposable test container was removed automatically. Final source hashes
matched before deleting the owned temporary source/build directory (about
483 MiB); its absence was verified. Prior deployments, shared toolchains,
unrelated caches and research data were not removed. An old template build cache
encountered during source synchronization was excluded from the temporary copy,
not deleted from the development machine. Only tests, repairs, guidance and
scoped evidence are retained.
