# Daji 3.7.8 Store And Native Execution Tensor Recalibration

Date: 2026-11-13. Source: the working-tree overlay based on `f04ec479`.
Package/book metadata remains `3.7.8`; the profile remains
`daji-3-7-hardening`. Existing uncommitted Store, deployment and isolation
changes are included in this review, not attributed to the last clean commit.
The last documented macOS installation is the November 4 build, not this source.

This is an evidence and acceptance-boundary review. It reuses retained test
results and runs the tensor/checker regressions, not a fresh numerical, installed,
multi-host, CI or paid-model campaign. No daily services are restarted.

## Decisions

- Keep 13 logical modules, 11 function paradigms and 77 required coordinates.
  Store distribution remains part of the existing architecture, not an execution
  authority or a new module created to inflate coverage.
- Preserve all 478 pre-review claims unchanged: 465 proven, 12 partial and one
  open. Add three narrowly scoped verified claims, bringing the total to 481,
  including 468 proven. Broad Store and native-isolation claims remain partial.
- Add 18 named scenarios: four retain bounded verified achievements; fourteen
  are still open. Require recovery as a dimension of Agent validation as well
  as execution/contract. Existing evidence already supplies that dimension.
- Do not change targets, scores, release criticality, weights or advisory mode.
  Old installed ABI execution cannot qualify current per-call isolation; a
  real SSH deployment does not prove native TLS across hosts or a Store login.
- Keep source tests, mocked bridges, installed journeys, backend draft sealing,
  published downloads, process termination and power-loss tests separate.

Registry:
`config/architecture/module-function-coverage-evidence/current-store-native-qualification-scopes.json`.
The existing general, solver and recovery registries stay active.

## Same-Tree Comparison

Both columns are November 13 computations on the same reviewed source, with
and without this new registry. They are not comparisons with an old release.

| Measure | Before | After |
| --- | ---: | ---: |
| Required coordinates | 77 | 77 |
| Retained claims | 478 | 481 |
| Coordinates meeting every configured target | 54 / 77 (70.1%) | 48 / 77 (62.3%) |
| Structural gaps | 0 | 0 |
| Required-dimension presence gaps | 2 | 2 |
| Below-target or scope-incomplete coordinates | 23 | 29 |
| P0 coordinates at target | 37 / 55 | 32 / 55 |
| P1 coordinates at target | 17 / 22 | 16 / 22 |
| Dimension-score progress | 94.0% | 94.0% |
| Named scenarios at target | 16 / 46 | 20 / 64 |
| Named scenarios still open | 30 | 44 |
| External P0 gate gaps | 1 | 1 |
| Release readiness | blocked | blocked |

Six previously target-met coordinates now expose unfulfilled current scopes:
`workbench-shell/runtime_api`, `workbench-shell/security`,
`runtime-agent-cli/deployment_update`, `runtime-agent-cli/sdk_operator`,
`runtime-installer/sdk_operator` and `sdk-operator/sdk_operator`.
The other new obligations belong to already-open coordinates. Do not add
scenario gaps to coordinate gaps as though they were independent defects.

The two dimension-presence gaps remain `runtime-protocol/sdk_operator/security`
and `workbench-shell/sdk_headless/execution`. A missing proof does not prove
missing implementation. PWDT and Headless remain separate control paradigms.

The 62.3% figure is configured coordinate target attainment, not code coverage,
physics coverage or reliability probability. The unchanged 94.0% is a dimension
score that excludes these exact-scenario gaps. Sixty-four scenarios remain a
reviewed subset, not proof that every platform and fault combination is listed.

## Bounded Achievements Retained

| Scenario | Evidence and limit |
| --- | --- |
| `sdk-native-result-local-contract` | Mac ARM64 and Linux x86_64 source SDK checks reject wrong identities and contradictory success envelopes; not physical-result validation. |
| `agent-native-result-local-recovery` | Real source Agents reject three bad result forms, preserve original task identity, reap the child and execute a separate healthy task. |
| `agent-unix-owned-process-local-recovery` | Source-native crash/hang/cancel/parent-loss and owned process-group cleanup; not escaped descendants, Windows or hostile-code sandboxing. |
| `mac-linux-ssh-variant-deployment` | Retained two-host template staging, explicit enablement, Installer-managed restart, exact digest and sum=14, plus conflict rejection; SSH tunnel, not native TLS. |

These four targets are explicitly `verified`, not `qualified` or `operational`.
The cross-host exercise predates the later isolation/result changes; this review
does not claim that the combined newest source was rerun across both hosts.
The [retained integration record](../docs/operator-store-integration.html)
provides exact commands, dates and limits. It records the latest separate
Mac/Linux source runs as 38 SDK tests, 337 Agent units and 13 real integrations
per platform, plus the Mac Engine suite. Those are prior test results, not new
tests run during recalibration or an additive lifetime test count.

## Explicit Open Acceptance

| Scenario | Required next proof |
| --- | --- |
| `workbench-real-store-account-session` | Installed browser/OIDC callback, session expiry/refresh/logout against the deployed issuer and a registered test account. |
| `workbench-real-store-authorized-download` | Actual entitlement denial, consent, quota-idempotent grant/download, reply loss and App reopen; independent trust pins. |
| `installer-public-signed-store-install` | Real published signed package to exact-platform native installation; draft sealing and local signing fixtures are insufficient. |
| `workbench-pwdt-store-agent-journey` | Matching installed GUI and WASM PWDT from catalog reference to enable/deploy/run/recover, not host Python or mocked bridges. |
| `agent-linux-variant-native-tls` | Actual Mac-to-Linux package recovery over native TLS/token, including credential rotation and changed Agent identity. |
| `agent-linux-arm64-variant-execution` | Real target library and Agent execution/recovery on Linux ARM64, not ELF-header fixtures. |
| `agent-windows-owned-process-isolation` | Current Windows process-tree lifetime/descendant cleanup and recovery, not old in-process ABI tests. |
| `agent-unix-installed-isolation` | Current isolated Agent packaged and installed into disposable owned services, with matching client artifacts and faults. |
| `agent-isolation-whole-call-performance` | Spawn/load/IPC/decode/cleanup latency and sustained capacity/resource bounds; old resident-host or 1M solver timing does not count. |
| `installer-project-operator-controlled-replacement` | Portable bundle pins, explicit managed version replacement and safe recovery without overwriting active packages. |
| `headless-package-deployment-published-parity` | New deployment contract in separately distributed Rust/Python/Elixir packages, not just the workspace Rust helper. |
| `orchestra-store-project-package-supply` | Authorized project-pin admission into Orchestra's catalog and bound-Agent pull/execution; no Store control authority. |
| `installer-multiplatform-store-publication` | Real target-native candidates through author upload, independent review and explicit publication; no automatic listing. |
| `sdk-current-isolated-abi-qualification` | Current independently built plugin/host pairs and valid old-package compatibility across the supported isolation matrix. |

Three open obligations reference existing partial claims; those references
acknowledge implementation progress but cannot satisfy a verified/qualified
target. The others explicitly bind no qualifying proof. None authorize service
restarts, production account activity or fault injection against user data.

## Priority And Next Work

The unchanged release-weighted queue now begins:

1. `sdk-headless/sdk_headless` (585): installed Python/Elixir research, current
   modal production journey and new deployment API distribution/parity.
2. `workbench-shell/validation` (500): installed recovery and a real GUI/PWDT
   Store-to-Agent journey rather than isolated mock/native layers.
3. `runtime-protocol/sdk_operator` (455): exact Worker-SDK protocol security
   evidence, distinct from Headless control and GUI automation.
4. `runtime-agent-cli/sdk_operator` (385): current installed Unix isolation,
   Windows process-tree recovery and real Linux ARM64 execution.
5. `runtime-engine-solver/validation` (380): physical modal publication,
   multimode recovery and independently reassembled requests.

This ranking is a planning heuristic: registering more distinct obligations
can increase a coordinate's priority. It is not measured defect density.
For the active Store/native workstream, first qualify native TLS deployment and
measure per-call isolation overhead on owned source test services. Then close
the installed GUI/PWDT path and published SDK deployment parity. Real deployed
account/download acceptance needs the backend handoff below; it need not block
independent execution reliability work.

Backend handoff remains public CA and fingerprint, registered OIDC public
client/callback/scopes, dedicated test account/entitlement cases, independent
signing-key fingerprint and published matching Mac/Linux variants. No private
keys or Agent tokens belong in that handoff. Store is responsible for immutable
distribution and access entitlement, not Agent lifecycle or execution authority.

Keep numerical production, authenticated Mesh, whole-generation restore,
PostgreSQL checkpoint migration, typed recovery and Windows installed scopes
open at their original targets. No numerical threshold was relaxed.

## Verification Of This Review

- Three new tensor regressions failed before the registry existed, then passed.
- All 28 tensor tests passed, including preservation of every earlier claim,
  same-policy comparison, no historical/transport evidence borrowing and
  reopening the exact obligation when its supporting proof is withdrawn.
- The full native script-runner suite passed: 419 tests, zero failures/skips.
- Tensor, matrix/topology, documentation, Store contract, formatting, strict lint
  and project organization checks passed for the final working tree; success is
  structural/configuration verification, not release qualification.

No product tests, remote CI, app installation, remote service update or paid
model API call were performed by this recalibration. The generated full tensor
stays in ignored `tmp/`; this compact record is retained instead of a source,
database or binary snapshot.

## Same-Day Native TLS Follow-Up

The sections above retain the original recalibration snapshot. A subsequent
execution campaign used the working tree based on `43020db6` (commit label
`daji 3.8.0`, compiled Cargo metadata still `3.7.8`) on Mac ARM64 and the physical
Linux x86_64 test host. No version synchronization or daily installation was
performed. See the [native TLS record and repeatable procedure](../docs/operator-store-integration.html#native-tls-cross-host-qualification).

- A fresh Installer-managed Agent and trusted template completed deploy and
  verify over actual non-loopback TLS/token connections, not an SSH RPC tunnel.
- Wrong token, server name, certificate trust and plaintext probes were denied.
  Deliberately discarding one received commit reply was reconciled by inspection
  without another upload or commit; this was not a network partition injection.
- Explicit test-Agent termination/restart and token rotation rejected both the
  old token and old process identity. The exact installed package became ready,
  executed `sum=14`, and remained unchanged after a conflicting version request.
- The owned host stopped successfully; its temporary installation, package store
  and credentials were removed. The Mac temporary copies were also removed.
  The existing server deployment and Agent retained their original PIDs.

The registry adds one narrowly `verified` claim, now 482 total / 469 proven.
`agent-linux-variant-native-tls` advances from `unassessed` to `verified` but its
`qualified` target stays open. The 48/77 target-met coordinates, 29 evidence
gaps, 23 P0 gaps and 20/64 met scenarios are unchanged. Repeated interruption
campaigns and deployment trust lifecycle still need evidence; this run does
not qualify installed fault isolation, GUI/PWDT, public Store downloads,
other architectures, or whole-call performance. No paid model API was used.

The scope test first failed for missing TLS proof, then all 28 tensor tests
passed. Each host passed 337 Agent main-unit and four API-security tests;
the three default transport/configuration guards passed on both hosts as well.
The actual cross-host deploy, verify and owned-host lifecycle were explicitly
run, not counted merely because their default-ignored tests exist.
Whole-call isolation latency/resource benchmarking remains the next separate
work item; the functional test duration is not that benchmark.

## Same-Day Whole-Call Performance Follow-Up

A subsequent [Linux optimized-build benchmark](native-operator-isolation-benchmark-20261113.md)
records three rounds each of two input sizes and one/four-way concurrency,
plus explicit four-slot saturation/cancellation/recovery. It measures the real
isolated RPC path and compares fixed versus bounded adaptive IPC polling on the
same machine/library. It does not reuse solver-node or old resident-host timing.

The `agent-isolation-whole-call-performance` scope advances to `verified`, not
`qualified`: short campaigns and partly observable resources do not establish
sustained capacity or hard limits. Claims now total 483 / 470 proven; the 48/77
met coordinates, 29 evidence gaps, 23 P0 gaps and 20/64 met scenarios remain
unchanged. The earlier sections preserve their dated pre-follow-up snapshots.

## Same-Day Installed Continuous-Recovery Follow-Up

Two further Linux Installer-managed short-soak campaigns completed 56864 normal
timed calls and sixteen total mixed-fault rounds after reproducing a capacity
handoff rejection. Admission now waits at most a 5 ms budget for a still-owned
delivery lease, without raising concurrency or retrying work. The report retains
exact counts, sampled-resource limits and the first failed attempt separately
from successful evidence. See the [continuous-load record](native-operator-isolation-benchmark-20261113.md#same-day-continuous-load-and-capacity-handoff).

The new `linux-installed-native-soak-recovery` claim raises the broader
`agent-unix-installed-isolation` scope only to `verified`; `qualified` remains
unmet, especially for matching installed GUI/SDK and Mac journeys. Claims now
total 484 / 471 proven. No old claim or release threshold is changed. This
minute-scale run is not a longevity, power-loss or hostile-code qualification.
The legacy unmatched pending-cancellation path remains a separate observed gap,
not covered by these active-job cancellation tests.

## Same-Day Bounded Pending-Cancellation Follow-Up

The previously observed unmatched `cancel_job` memory-growth gap is now bounded
at 1024 distinct process-local intents, each with a 256-byte identity ceiling.
The original failing capacity, byte-bound and concurrent-admission regressions
turned green. No automatic expiry or eviction was introduced: a full registry
rejects only new pending identities, not duplicate intents or active/exact
execution cancellation. Registration failure skips package cleanup; accepted
cancellation survives a separate cleanup failure.

Both Mac ARM64 and physical Linux x86_64 source Agents passed 351 main-unit tests
and 11 integration test entries across API security, exact-target cancellation
and the new pending-cancellation suite. The total includes shared readiness
guards compiled into each integration binary. The opt-in TLS case was explicitly
run on both hosts, not counted from its presence: it retained and consumed all
1024 intents, rejected 32 excess IDs, preserved intents on capacity/drain
rejection and completed healthy follow-ups. Restart explicitly removed the
process-local intent. Package-cleanup nonexecution is unit-tested; this TLS case
does not bind a real job-scoped package and cannot qualify its retention.

The tensor adds `agent-bounded-pending-cancellation-source-regression` and the
matching verified local-recovery obligation. Claims are now 485 / 472 proven;
named scenarios are 21 / 65 met, with the same 44 open scenarios. Coordinate
targets remain 48 / 77 met, 29 evidence gaps and 23 P0 gaps. Release policy and
all historical claims remain unchanged. The new scope test failed before its
registration, then all 419 native tool tests passed. Book/inventory, Store
contracts, formatting, strict targeted Mac lint and source/doc limits passed.
Linux strict lint for the new integration target passed after an equivalent
cleanup of the shared readiness conditional; the earlier whole-binary Linux
lint findings are not claimed resolved by this targeted result.

This does not establish persistent cancellation, unlimited pre-cancellation
availability, installed GUI/SDK closure or hostile-client fairness. No App,
daily runtime, Git commit/push or paid model API was changed. See the
[policy and repeatable command](../docs/operator-store-integration.html#agent-bounded-pending-cancellation).

Performance follow-up: the complete new integration binary took 4.53 seconds on
the Mac debug build and 350.00 seconds on the Linux release build. This is an
observed wall-time discrepancy, not a controlled cross-platform benchmark or
isolated solver/TLS measurement; handshakes, many short requests and test setup
are included. Investigate short-frame transport latency separately before
making a throughput or cryptographic-cost claim.

## Same-Day TLS Latency Follow-Up

The discrepancy above was subsequently isolated with three 32-call cold-TLS
rounds before and after enabling `TCP_NODELAY` on both TLS endpoints. On the same
Linux optimized build environment, whole-call p50 fell from 123.99-124.86 ms to
1.45-1.47 ms; handshake/authentication and frame-exchange timings are retained
separately in the [experiment record](native-operator-isolation-benchmark-20261113.md#cold-tls-control-frame-latency).
The full 1024-intent Linux correctness suite then passed in 1.39 seconds, with
the same calls/assertions and no authentication, resumption or deadline changes.

Only `linux-native-tls-cold-frame-latency` is added at verified grade: 486 total
claims / 473 proven. Scenarios remain 21 / 65 met and coordinates 48 / 77 met,
with 29 evidence gaps and 23 P0 gaps. A regression ensures this cold-frame claim
cannot replace missing native isolated-call timing. Large payloads, real
cross-host latency, sustained throughput and installed GUI/SDK remain separate
acceptance scopes; no whole-journey qualification was promoted.

## Package Cleanup Contention Follow-Up

A real source Agent reproduced cancellation acknowledgment waiting behind a
stalled package-resolution request. The regression failed before the fix:
the two-second acknowledgment watchdog expired while the fixture still held
the HTTP response. Releasing that response let the old cancellation finish.

Job cleanup now validates identity and tries the package lifecycle lock without
waiting. Contention returns `operator_package_job_release_busy` before planning
or mutating retention/generation state. Accepted cancellation remains accepted;
its nested cleanup receipt is `deferred`, explicitly not performed, and names
only `release_operator_package_job` for a later cleanup retry. No automatic
cleanup worker, execution replay or early capacity release was added. Poisoned
lock admission rejects cleanup and requires explicit stopped inspection/restart.
The read-only descriptor states this is a lock-admission boundary, not a total
I/O deadline.

Mac ARM64 and physical Linux x86_64 each passed 355 Agent unit tests and 13
integration test entries: stalled-fetch contention, TLS API security, exact
execution cancellation, bounded pending cancellation, and the existing real
template fetch/cache/version-rotation/release journey. Shared readiness tests
are included in that entry count. The new contention case uses plain loopback,
an empty package store and a controlled 503 response; it proves cancellation
is visible while execution remains active, excess execution is rejected, then
cleanup-only retry and a healthy same-job FL/EA solve work. It does not prove
retention of an already-bound package under contention, TLS contention or an
installed GUI/SDK journey. Existing real-package tests remain separate evidence.

The tensor adds only `agent-package-cleanup-contention-source-regression` and
its verified local-recovery obligation: 487 claims / 474 proven, 22 / 66 named
scenarios met, 44 still open. Coordinates remain 48 / 77 met, with 29 evidence
gaps and 23 P0 gaps. Revoking this claim must reopen its own obligation even
while TLS and pending-cancellation evidence remains present.

All 419 native tool unit tests, documentation book/inventory, Store contract,
formatting and source/doc organization checks passed. Strict Mac lint passed
for the Agent and new test; Linux 1.95 strict dependency-inclusive lint still
finds pre-existing protocol style warnings, while the new test target passed
strict lint with dependency linting excluded. Those dependency findings are
not fixed or waived by this result.

Next boundary: cancellation acknowledgment does not interrupt the underlying
synchronous HTTP fetch or a different execution queued for the lifecycle lock.
Those operations may still occupy execution capacity until they exit. Filesystem
work after cleanup lock acquisition also has no new total-time bound. Neither
power-loss safety nor hostile-client fairness is established here. No installed
App/runtime, daily server process, Git commit/push or paid API was changed.

## Supervised Package Download Follow-Up

The previous section's synchronous HTTP cancellation gap was reproduced before
implementation: cancellation was acknowledged, but the execution could not exit
while the fixture withheld the HTTP response. The Agent now reuses its owned
child supervisor for an Installer-only download request, with no dynamic-library
loading in the credential-bearing process and no detached download thread.
Installer validation and the public direct Installer API remain unchanged.

The default 120000 ms budget is configurable through
`KYUUBIKI_OPERATOR_PACKAGE_FETCH_TIMEOUT_MS` (100..3600000 ms) and visible under
`operator_package_runtime.isolation.package_fetch`. Lifecycle-lock waiting is
cancellable and consumes the same budget; poisoning rejects acquisition. The
parent terminates/reaps the downloader before unwinding the unpublished
generation. Child temporary storage stays inside that generation. The token
travels through private IPC, not argv/environment/config files; request buffers
and the request object's token are cleared on normal drop. Proxy-routing
variables are forwarded only to the downloader, never operator execution.

Mac ARM64 and physical Linux x86_64 passed 359 Agent unit tests and 26 related
integration entries each (including shared fixture checks). Three product
scenarios retain the remote fixture's barrier: stalled headers, partial response
body cancellation, and partial-body deadline without cancellation. Assertions
check terminal error identity/stage, closed download connection, removal of the
unpublished generation and injected incomplete work file, no automatic replay,
and healthy same-job FL/EA follow-up. The actual template-fetch/version-rotation
test and all ten native process-isolation entries were also rerun successfully.

Repeated parallel testing prompted fixture hardening around shared time-based
naming and port-only readiness; it also exposed an accepted socket that could
inherit nonblocking mode on macOS. Fixtures now use distinct scenario directories,
unique Agent identities, serialized port-selection-to-readiness, and explicit
blocking HTTP reads. A new
fragmented-header fixture regression retains this last guard. The final four-test
binary passed five additional consecutive Mac rounds and three Linux rounds;
these are repetitions, not additional independent product coverage.

All 419 native tool tests passed. The new evidence is only
`agent-supervised-package-fetch-source-regression` with its matching verified
local-recovery obligation: 488 claims / 475 proven, 23 / 67 named scenarios met,
44 open. Coordinates remain 48 / 77 met, 29 evidence gaps and 23 P0 gaps. A test
revokes supervised-download proof independently from the cancellation-ack proof,
so one cannot substitute for the other.

Remaining boundaries: parent-side generation copying and cleanup have no new
hard filesystem deadline; stalled HTTPS handshakes, real large-artifact partial
writes, parent loss during download, power loss, Windows and installed GUI/SDK
journeys remain unqualified here. Shared native-parent-loss tests are not a
download-parent-loss experiment. Installed Apps and daily services were not
changed, and no commit/push or paid model API was used.

## Real Artifact Interruption And Restart Follow-Up

The new live test failed before implementation: a held 64 MiB entrypoint response
had transferred 16 MiB, but no actual partial artifact existed on disk. Installer
previously buffered the complete library (up to 512 MiB) before staging it. It
now copies through a 64 KiB buffer into unpublished staging, hashes incrementally,
rejects extra/truncated bytes and mismatched digests, and syncs the verified file
before calling the existing managed installation path. Metadata limits remain
separate. This is a bounded download-copy buffer, not a whole-process RSS limit.

Repeated existing fetch tests also reproduced an independent staging-name race:
two same-process downloads received the same timestamp and one failed with
`failed to create operator download staging: File exists`. An atomic process-local
sequence now supplements PID/time, while directory creation remains exclusive.
A deterministic eight-thread test generates 512 distinct names at an identical
clock value. The five fetch unit entries passed ten consecutive post-fix rounds;
repetitions do not increase independent scenario coverage.

Mac ARM64 and physical Linux x86_64 each passed all four new opt-in integration
tests: complete 64 MiB synthetic artifact verification, cancellation after 16 MiB
was written, premature HTTP EOF after 16 MiB, and owned-Agent SIGKILL at that same
barrier. The interruption cases observe real partial files, reject publication,
verify downloader process exit, and preserve the absence of automatic replay.
After parent loss, the partial file remains for the existing session janitor;
restart removes the old session before a new task. Explicit submission then
re-fetches the real template library and executes sum=14, followed by job release.
Synthetic large-file bytes are never loaded as executable code. The transfer is
loopback on each host, not a cross-host throughput benchmark.

Both hosts also passed the four earlier contention/timeout fixture entries and
the real template fetch/cache/version-rotation journey: nine integration entries
per host in this batch. Both passed 140 Installer unit tests; Mac additionally
passed 359 Agent and 419 native-tool unit tests. The dynamic package smoke entry
now includes `operator_download_recovery`, so the opt-in regressions accompany
template build/load/dispatch qualification rather than remaining manual-only.

Mac strict lint passed for the changed Installer library and new integration
target. Linux 1.95 strict Installer lint still rejects four pre-existing style
sites in `agent_solver_operational_validation.rs`, `integrity.rs`,
`operator_deployment.rs` and `remote_deployment_journal.rs`; they were not edited
or suppressed. The new Linux integration target passed strict lint with
dependency linting excluded. Documentation book/inventory, Store contracts, formatting and
the 800-line source / 2000-line documentation organization audit passed.

The tensor adds only `agent-large-download-restart-source-regression` and its
separate verified local-recovery obligation: 489 claims / 476 proven, 24 / 68
named scenarios met, 44 open. Coordinates remain 48 / 77 met with 29 evidence gaps
and 23 P0 gaps. Revoking this claim reopens its own obligation while the older
partial-metadata cancellation proof remains valid.

Limits remain explicit: SIGKILL is not physical power loss, and this restart
experiment waits for downloader exit before starting the replacement Agent.
Parallel janitors, restart racing a suspended/unresponsive orphan, HTTPS faults,
Windows, installed GUI/SDK and hard filesystem deadlines remain separate work.
No installed App/runtime, daily server process, commit/push or paid API changed.

## Suspended Downloader And Concurrent Janitor Follow-Up

The previous restart proof waited for the downloader to exit. This follow-up
keeps the downloader suspended after a real 16 MiB partial write, kills only
its owned Agent, and starts replacement Agents before resuming the child.
The controlled negative run removed only the new child lease and its preliminary
assertion: restart deleted the in-use directory while the downloader was still
present with PPID=1 and stopped state T. Both changes were restored before the
passing runs. This is a reproduced lifetime race, not an inferred risk.

The fixture also needed correction: POSIX can send SIGHUP/SIGCONT to an orphaned
stopped process group. A test-owned native process anchors that group while the
downloader is paused, so parent death cannot silently resume it and invalidate
the experiment. The fixture confirms the stopped state, retains cleanup guards,
and does not add a production process or shell dependency.

Agent and downloader now independently hold shared session leases. Cleanup
requires an exclusive lease, so parent loss alone no longer makes a live writer's
directory eligible for deletion. A stable cache catalog lock serializes session
creation, child attachment and cleanup across processes; waiting is bounded at
2000 ms, with an explicit retry error. That limit does not impose a filesystem
I/O or directory-scan deadline. The lock file is deliberately reusable metadata,
not an abandoned download, and is never unlinked during ordinary session cleanup.

Busy sessions are retained without PID guessing or killing. Startup and explicit
generation preparation rescan stale sessions; after the orphan exits, the next
explicit task can clean the old directory and fetch again without another Agent
restart. This does not replay a failed task automatically. The recovery policy
is visible in the package-fetch snapshot. Removal counts accumulate while active
and invalid counts describe the latest scan, excluding the caller's own session.

The second new live test starts four separate Agents concurrently over one cache.
Exactly one janitor claims the stale session, all four active directories remain,
and each independently downloads the real template and executes sum=14. The
suspended-child test additionally retains a live peer while resuming the orphan,
then checks deferred cleanup, no automatic download, explicit successful execution
and continued peer responsiveness. Thread-level tests cover shared-lease lifetime,
eight-way session startup/drop, bounded catalog waiting and rejection of late
child attachment after removal without recreating the directory.

Mac ARM64 and physical Linux x86_64 each passed 362 Agent unit tests and all 11
related integration entries: six real-download/restart cases, four contention
entries and one real-template fetch/cache/version journey. Mac also passed all
419 native-tool tests and strict lint for the CLI binary and new integration
target. The new Linux integration target also passed strict lint with dependency
linting excluded; this does not clear the earlier full-package lint limitations.
Both new live scenarios passed one additional round on each host. Book/inventory
checks, Store contracts, formatting and the source/document line-limit audit
passed. No matching large-download temporary directories remained on either host,
and the daily server process identities were unchanged. These source/local-
filesystem tests do not qualify installed Apps or every scheduling interleaving.

Only `agent-orphan-download-coordination-source-regression` and its matching
local-recovery obligation are added to the tensor: 490 claims / 477 proven,
25 / 69 named scenarios met and 44 open. Coordinates remain 48 / 77 met, with
29 evidence gaps and 23 P0 gaps. Revoking this claim reopens its obligation
independently while the earlier wait-for-child-exit restart proof remains valid.

Physical power loss, network filesystems, mixed-version Agents sharing a cache,
Windows, installed GUI/SDK journeys, hard filesystem deadlines and crashes during
initial marker creation remain unqualified. Invalid sessions remain fail-closed.
No daily service, installed runtime, Git commit/push or paid API was changed.

## Incomplete Session Initialization Follow-Up

Session creation previously made its directory before persisting the identity
marker. Terminating the process during that write left an unprovable directory
that the ordinary janitor correctly refused to delete. A negative control with
the new initialization recovery disabled reproduced that gap: the matching-intent
partial-marker regression failed with the owned directory still present. The
recovery call was restored immediately; no safety assertion was weakened.

The new `operator_package_generation_session/initialization` module persists a
bounded creation record before creating a session directory. That record binds
the cache and exact expected session metadata. Session initialization commits by
removing it only after the marker, lease and empty generations directory exist.
Startup and explicit generation preparation recover these records under the
existing catalog lock; an existing session lease must also be exclusively
available. Active holders are retained, not killed.

Cleanup accepts only expected complete metadata or its write prefix and an empty
generations directory. Unknown files, mismatched identities, nonempty payloads,
oversized files and symlinks are retained. Removal is non-recursive, and a crash
during removal leaves its record for a subsequent attempt. Invalid record names
cannot escape the managed catalog; malformed records with a valid associated
name also protect that session from the ordinary janitor. Reads enforce the
16 KiB limit during the read, not just in a preceding metadata check.

The native unit-test executable launches owned child processes running the real
session initialization code. SIGKILL is applied after ten committed-intent
creation checkpoints, two partial-cleanup checkpoints and one torn-intent
checkpoint. The readiness barrier is atomically published so the parent cannot
kill the child before receiving the complete checkpoint identity. These barriers
exist only in test builds, not as production environment-variable controls.
This is not a whole-Agent startup test at every checkpoint.

Mac ARM64 and physical Linux x86_64 passed 371 Agent unit tests; the separately
ignored child entrypoint is driven by the parent regression rather than counted
as an independent passing scenario. Both hosts also passed the existing 11
download/contention/real-template integration entries. Mac passed all 419 native
tool tests and strict CLI test-target lint after fixing one pre-existing redundant
borrow in the capacity-handoff test. Documentation book/inventory, Store contracts,
formatting, tensor validation and the source/document size audit passed.

The tensor adds only `agent-session-initialization-source-regression` and its
matching verified local-recovery obligation: 491 claims / 478 proven, 26 / 70
named scenarios met, 44 open. Coordinates remain 48 / 77 met, 29 evidence gaps
and 23 P0 gaps. Removing the new proof reopens this obligation without revoking
the separate suspended-downloader evidence.

The remaining boundary is deliberate: a record torn before intent completion is
retained and reported, while no corresponding session directory has been created.
Old partial directories without an intent are not retroactively claimed. Physical
power loss, adversarial concurrent filesystem mutation, network filesystems,
mixed-version Agents, Windows and installed GUI/SDK acceptance remain unqualified.
This does not establish directory-entry durability under hardware failure.
No App installation, daily service restart, commit/push or paid API was performed.
