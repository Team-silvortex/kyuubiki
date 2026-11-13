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

## Initialization Staging Publication Follow-Up

New initialization records no longer write directly to their final JSON name.
They use a versioned staging name containing canonical session identity and the
SHA-256 of the complete expected record. The record is reconstructible from that
identity and the current canonical cache path. File sync precedes same-directory
hard-link publication, which cannot replace an existing name; the staging name is
then unlinked before session creation. No overwrite/copy fallback is introduced.

Recovery requires both the expected name commitment and a bounded byte-prefix
match. Before publication, neither the final record nor the session may exist.
After publication, both staging and final bytes must be complete and identical.
Only the staging name is removed; session deletion remains the responsibility of
the separate intent/lease janitor. Rejected staging protects its associated
session from both janitors. Foreign-cache commitments, corrupt prefixes, trailing
bytes, oversized files, unexpected sessions, final conflicts and symlinks remain
reported rather than deleted. Old torn JSON records without commitments are still
unproven; the earlier scope was clarified to preserve that legacy boundary.

Owned test children were killed after staging creation, partial write, sync,
publication and staging unlink. The recovery assertions retain no-replay,
idempotence and active-lease protection. A controlled negative experiment replaced
hard-link publication with ordinary rename: the existing-content preservation
test failed. The no-replace implementation was restored and the full suite rerun.
The commitment is not a signature or authentication against hostile local writers.

The new `removed_initialization_staging_count` is separate from removed sessions,
accumulates across explicit rescans and appears in execution/cache lifecycle
receipts. A regression caught the field initially reaching only one receipt
builder; the duplicate builder was removed in favor of the shared function.
The strict JSON schema and example now include the nonnegative field, optional
for older v1 receipts. Native contract checks enforce that optionality. This is
not a promise that old strict validators accept every new field.

Parallel failure testing also exposed a separate execution-store ownership issue.
A duplicated descriptor could retain its OS lock after the logical Store closed.
A deterministic pre-fix test reproduced immediate reopen failure while the
duplicate remained alive. An owning-process RAII lease now explicitly unlocks on
normal closure and initialization failure, without changing identity bytes or
unlocking the next owner. The initial parallel failure is consistent with a
temporary fork/exec descriptor copy, but that exact scheduling interleaving was
not traced. Raw lease fixtures now explicitly release logical ownership before
asserting immediate cleanup; active protection assertions remain unchanged.

Final Mac ARM64 and physical Linux x86_64 runs each passed 382 Agent unit tests
plus the 11 download/contention/real-template integration entries. One ignored
entry is the child-process harness, driven by its parent regressions. Repeated
Mac parallel unit runs also passed. Mac passed all 419 native-tool tests, strict
CLI test-target lint, the updated package lifecycle contract, book/inventory and
Store checks, formatting and the source/document size audit. Both hosts had no
remaining initialization-test directories; daily service identities were unchanged.

The tensor adds `agent-initialization-staging-source-regression` and its separate
verified local-recovery obligation: 492 claims / 479 proven, 27 / 71 named
scenarios met, 44 open. Coordinates remain 48 / 77 met, with 29 evidence gaps and
23 P0 gaps. Revoking staging proof does not revoke the separate committed-intent
recovery proof. The incidental Store lease fix is not whole-store qualification.

This requires local filesystem hard-link support and qualifies process failures,
not directory-entry durability after power loss. Unsupported filesystems fail
explicitly. Network filesystems, Windows, mixed versions, hostile concurrent
filesystem mutation, all I/O failures and installed GUI/SDK remain outside this
claim. No installed App/runtime, daily service restart, commit/push or paid API
was performed.

## Initialization I/O Failure Follow-Up

The target existence probe previously used `symlink_metadata().is_ok()`, treating
inspection failures as absence. A pre-fix regression replaced the sessions parent
with a regular file and demonstrated that initialization still created an intent.
Only `NotFound` now permits creation; permission and other inspection errors return
before staging or intent creation. Existing-target preservation remains unchanged.

The new test-only `initialization/io_failures` module separates three mechanisms:

- An owned subprocess sets a 96-byte file-size limit and ignores its own SIGXFSZ.
  The production writer produces a real partial staging file, but no final record
  or session. The subprocess restores its limit; replacement initialization
  verifies and removes the prefix without replay or damage to a live peer.
- A thread-local test hook returns an error at the staging file-sync boundary.
  Publication remains blocked, and a later healthy open recovers the staging file.
  This is an injected sync error, not a real storage-controller or kernel failure.
- An explicitly invoked non-root Unix test removes permissions only from owned
  fixture directories. It covers target inspection, hard-link publication,
  staging unlink, final intent removal and incomplete-session cleanup. Failed
  cleanup retains records and does not increment successful removal counts.
  Restoring permissions permits explicit retry; rescanning is idempotent and
  active peer metadata remains unchanged. Root execution is rejected, not passed.

Mac ARM64 and physical Linux x86_64 each passed 385 Agent unit tests, the separate
permission qualification, and all 11 related download/contention/template
integration entries. The unit suite has three ignored entries: two subprocess
harnesses driven by parent tests and the separately executed permission test.
Mac also passed 419 native-tool tests, strict CLI test-target lint, lifecycle and
Store contracts, book/inventory, formatting and the organization/line-count audit.
No initialization fixture directories remained on either host; the daily Linux
service process identities were unchanged.

The tensor registers `agent-initialization-io-failure-source-regression` and its
separate local-recovery obligation. Revoking this proof reopens this obligation
without revoking process-termination/staging evidence. Current totals are 493
claims / 480 proven, 28 / 72 named scenarios met, 44 open. Coordinates remain
48 / 77 met with 29 evidence gaps and 23 P0 coordinate gaps; release remains blocked.

This is source-level local-filesystem qualification, not actual disk exhaustion,
physical sync failure, complete Agent startup under every error, all filesystem
errors, power loss, hostile concurrent mutation, network filesystems, Windows or
installed GUI/SDK acceptance. `retained_invalid_session_count` also includes
temporarily unverifiable or unremovable records, not just corruption. Ordinary
completed-session recursive cleanup under partial deletion errors is not closed
by this initialization-only proof. No App/runtime update, daily service restart,
commit/push or paid model API was performed.

## Completed Session Retirement Recovery

A pre-fix Mac permission regression reproduced the remaining full-session cleanup
problem: whole-root `remove_dir_all` deleted the session marker before failing on
an undeletable payload. A subsequent scan could no longer prove the leftover's
identity. This was observed using an owned temporary directory under a non-root
account, not simulated by manually deleting the marker.

The new `operator_package_generation_session/retirement` module keeps an independent
identity link in `agent-runtime-generations/retirements/<session-id>.json` before
payload deletion. Publication uses a no-replace hard link to the validated existing
marker, so it cannot leave a newly half-written record. Both catalog coordination
and an exclusive session lease precede publication. The lease remains held while
payload entries are removed; only then are the lease, marker, empty session root
and finally retirement record unlinked. The record is temporary recovery metadata,
not a retained backup or a new task journal.

Recovery accepts matching identity and legal partial-cleanup states, retains live
lease holders and rejects new downloader attachment to a recorded retirement.
Unknown root contents, foreign/malformed records, conflicting identity and unsafe
symlinks remain reported. A missing marker or lease with payload still present is
not a valid cleanup state. Root/generations symlinks are rejected; nested payload
links are unlinked without touching their external targets. Conflicting creation
and retirement records protect the session from both janitors. Legacy partial
deletions without a record are not retroactively claimed. Removal is counted only
after the final record unlink succeeds; this is not a durable global counter.

Tests inject errors and SIGKILL owned native test children at six boundaries:
record publication, first payload-entry removal, full payload removal, lease
unlink, marker unlink and empty-root removal. The first-entry barrier confirms
that one payload has actually disappeared while another remains. Real non-root
permission tests cover partial payload removal, record publication and final
record unlink. They verify explicit retry, no replay, preserved active peers and
idempotent rescans. Six concurrent openers claim one recorded cleanup exactly once.
The fault controls and subprocess barriers exist only in test builds.

Mac ARM64 and physical Linux x86_64 each passed 395 Agent unit tests, three explicit
permission qualifications (including the prior initialization regression), and
all 11 related download/contention/template integrations. Six unit entries are
ignored by default: three child harnesses driven by their parent tests and three
permission qualifications run explicitly. Mac also passed 419 native-tool tests,
strict CLI test-target lint, lifecycle/Store contracts, documentation checks,
formatting and the source/document size audit. No retirement fixture directories
remained, and the daily Linux service process identities were unchanged.

The tensor adds `agent-session-retirement-source-regression` and its independent
local-recovery obligation: 494 claims / 481 proven, 29 / 73 named scenarios met,
44 open. Removing this proof reopens completed-session cleanup qualification
without revoking initialization-I/O proof. Coordinates remain 48 / 77 met with
29 evidence gaps and 23 P0 coordinate gaps; release status remains blocked.

This qualifies source-level cleanup on local Mac/Linux filesystems, not every
whole-Agent startup boundary, all I/O errors/interleavings, actual disk exhaustion,
directory-entry durability after power loss, hostile local mutation, mixed
versions, network filesystems, Windows or installed GUI/SDK acceptance. Hard-link
support is required; the identity link is not authentication. Filesystem removal
duration is not bounded by the catalog-lock acquisition timeout. No App/runtime
update, daily service restart, commit/push or paid API was performed.

## Session Lifecycle Entry Consistency

Controlled fixtures introduced unresolved initialization records into an otherwise
valid live session. Three pre-fix regressions failed: normal session drop deleted
the protected root, downloader admission accepted a new lease, and generation
preparation continued despite the current session's unresolved record. Startup
scan protection alone therefore did not cover these other session entrypoints.

The shared `lifecycle` check now guards normal close, downloader attachment and
generation preparation under catalog coordination. It rejects final initialization
records, associated v1 staging records and retirement records for the current
session. Staging association reuses the recovery parser rather than a second name
interpretation. The helper validates record namespaces as real directories; a file,
symlink or an unreadable initialization namespace is not evidence of absence.
Preparation publishes the scan counters before returning a refusal, and does not
create a generation directory. Read-only `session_recovery` exposes the admission
and normal-close policies.

Tests verify all three entrypoints, actual generation preparation, explicit retry
after resolving the fixture's record, healthy peer independence and preservation
of already held downloader leases. Valid-but-uncommitted initialization also blocks
new work; after its owner closes, the existing recovery protocol can safely finish
the known incomplete session. This does not revoke existing execution or introduce
automatic replay. Normal close retains ambiguous sessions for a later scan rather
than publishing retirement and deleting them behind the scanner's protection.
Namespace substitution tests preserve outside symlink targets, and a non-root
permission test separately denies enumeration while allowing directory lookup.

Mac ARM64 and physical Linux x86_64 each passed 403 Agent unit tests, four explicit
permission qualifications and all 11 related download/contention/template
integrations. Seven default-ignored entries are three subprocess harnesses driven
by their parent tests and four explicitly executed permission qualifications.
Mac passed 419 native-tool tests, strict CLI test-target lint, lifecycle/Store
contracts, book/inventory, formatting and source/document organization checks.
No admission fixture directories remained; daily Linux service identities did
not change.

The tensor adds `agent-session-lifecycle-admission-source-regression` and an
independent qualification scope: 495 claims / 482 proven, 30 / 74 named scenarios
met, 44 open. Revoking this proof reopens entry consistency without revoking the
separate completed-session retirement proof. Coordinates remain 48 / 77 met with
29 evidence gaps and 23 P0 coordinate gaps; release status remains blocked.

This is current source-level Mac/Linux local-filesystem qualification. It does
not cover every generation-internal cleanup path, hostile concurrent filesystem
mutation, mixed binaries, physical power loss, network filesystems, Windows or
installed GUI/SDK acceptance. A broken shared record namespace can block the cache
as a whole; healthy-peer isolation here concerns per-session records, not unreadable
global metadata. No App/runtime update, daily service restart, commit/push or paid
model API was performed.

## Owned-Generation Cleanup Admission

Four executed pre-fix regressions demonstrated another lifecycle bypass: retired
generation deletion and abandoned-preparation drop ignored unresolved per-session
initialization records, while explicit deletion also accepted a changed session
identity and bypassed a held catalog lock. Protecting startup scans and new
preparation alone did not protect these destructive entrypoints.

Owned-generation removal now enters through the session's
`with_generation_cleanup` guard. It validates the current real session directory,
compares the persisted identity, checks final/staging/retirement records, and
retains catalog coordination through the removal operation. Errors preserve the
generation; explicit cleanup retry works after resolution. A dropped preparation
with unresolved records leaves its payload for later stale-session reclamation,
not task replay. Healthy peer sessions still clean their own generations.
The read-only `session_recovery.generation_cleanup` policy describes this boundary.

Eight new tests cover those four regressions, lock lifetime/release after failure,
file and symlink namespace substitution, and actual non-root directory permission
denial. Mac ARM64 and physical Linux x86_64 each passed 410 Agent unit tests,
five explicitly run permission tests and 12 download/cancellation/template
integration tests. Eight default-ignored unit entries comprise three child
harnesses already driven by parent tests and the five permission qualifications.
Mac also passed 419 native-tool tests, strict CLI test-target lint, lifecycle
contracts, documentation checks, formatting and the source/document size audit.
No admission fixture directories remained; daily Linux services kept their
original process identities.

The existing `agent-session-lifecycle-admission-source-regression` claim and its
named scope now require this additional deletion-entry evidence rather than
adding a new completed coordinate. The tensor still reports 29 evidence-grade
gaps, 23 P0 coordinate gaps and blocked release readiness. Source tests and the
actual installed GUI/SDK acceptance remain separate.

Remaining audit candidates include missing lifecycle-record directories being
treated as absent records, and differences between downloader admission and
retirement validation for malformed owner/lease metadata. These are code-review
findings for a subsequent controlled regression, not newly qualified behavior.
Generation-internal partial recursive deletion, power-loss durability, hostile
concurrent mutation, mixed versions, network filesystems and Windows remain
outside this cleanup-admission proof. No App/runtime update, daily service
restart, commit/push or paid model API was performed in this round.

## Generation Partial-Cleanup Recovery

A real non-root permission failure reproduced the missing recovery identity in
generation cleanup on both Mac and Linux. The previous whole-root recursive
removal had no independent record surviving its internal marker. In these
executed pre-fix trials the internal marker still existed: the failed assertion
was absence of the independent record, not an observed early marker deletion.
The subsequent boundary tests exercise the later marker/root removal windows
explicitly rather than assuming a particular filesystem traversal order.

Generation removal now retains the existing session identity/lifecycle/catalog
guard, publishes a no-replace sibling hard link to the validated marker, removes
payload entries, then removes the internal marker, empty root and sibling record
in that order. The record is metadata, not a copy of the package payload, and is
removed immediately on completion. Unsupported hard links fail before payload
deletion. Record reads are bounded, and both persisted identities are compared
against the immutable owned identity, not merely against each other.

Retry can finish an empty root whose internal marker was already removed, or
unlink the final record after the root is gone. An absent root does not bypass
record validation. Runtime retirement retains its owner on failure, including
failure to unlink that final record. Conflicting, malformed, oversized and linked
identity records remain protected; legacy missing identity is not guessed.
Any retirement record blocks a new downloader for that generation under catalog
coordination, while a healthy sibling generation remains usable.

Five boundaries are tested with both injected errors and SIGKILL of an owned
test subprocess: record publication, first payload removal, all payload removal,
internal marker removal and root removal. Stale-session recovery reclaims the
terminated child's residual generation exactly once, preserves a live peer and
does not replay work. Separate ordinary-user permission tests deny payload
deletion, record publication, root deletion and final record deletion; explicit
repair/retry and retirement-queue ownership are verified. Abandoned preparations
whose drop cleanup fails are reclaimed after their session closes, not by a new
live-session orphan queue.

Mac ARM64 and physical Linux x86_64 each passed 419 Agent unit tests, eight
explicit permission tests and 12 related download/cancellation/template
integration tests. The 12 default-ignored unit entries comprise those eight
permission tests and four subprocess entrypoints exercised by their parent
tests. Mac also passed 420 native-tool tests, strict CLI test-target lint,
dynamic-package contract self-tests, book/inventory validation, formatting and
the source/document size audit. No generation-cleanup/queue fixture directories
remained; daily Linux services kept their original process identities.

The independent `agent-generation-partial-cleanup-source-regression` claim and
local-recovery scope bring the tensor to 496 claims / 483 proven and 31 / 75
named scenarios met, with 44 open. Removing this proof reopens generation cleanup
qualification without revoking the separate session-retirement or admission
proof. Coordinate coverage remains 48 / 77, with 29 evidence gaps and 23 P0 gaps;
release readiness remains blocked rather than promoting an entire recovery
coordinate from this narrow result.

This qualifies current source-level local Mac/Linux cleanup, not every installed
Agent/GUI/SDK path, all I/O failures or interleavings, actual disk exhaustion,
physical power-loss durability, hostile local mutation, mixed binaries, network
filesystems or Windows sharing errors. The identity record is not authentication;
filesystem deletion duration is not bounded by the catalog-lock wait budget.
The earlier lifecycle namespace and downloader metadata audit candidates remain
open. No App/runtime update, daily service restart, commit/push or paid model API
was performed.

## Abandoned Preparation Live Retry

An executed pre-fix regression left a partially removed preparation in the same
live session, cleared the injected failure and prepared another generation.
The old payload still existed at the record-publication boundary: dropping the
preparation lost the only retry owner, and new preparation only scanned stale
sessions. This round closes that specific gap without adopting unknown live
directories or changing runtime host ownership.

Failed preparation drop now records immutable generation paths and identity in
a session-owned retry queue. The queue has no strong reference back to its
session; a lazy worker holds only a weak reference while waiting. It retries at
1000 ms intervals, at most 16 entries per sweep, and exits when empty or when the
session is gone. New preparation also retries and can restart a failed worker
launch. Creating the packages directory now occurs after the prepared ownership
guard exists, so a failure there uses the same cleanup/queue path rather than
an unchecked recursive fallback. Failure while initially writing the identity
marker remains outside this new queue's admission boundary.

Failed entries rotate behind later arrivals, and queued/in-flight roots are
deduplicated. A separate nonblocking sweep guard prevents overlapping retries;
the queue-state mutex is released before filesystem operations. Each batch
reuses the existing session identity, unresolved-record and catalog checks once,
then the same generation cleanup protocol. Identity conflicts stay queued until
explicitly repaired. Committed or successfully removed generations are never
enqueued by preparation drop, and healthy active generations remain untouched.
Neither automatic task replay nor automatic package download is introduced.

The weak worker cannot pin an idle session, although an in-progress batch can
temporarily extend its lifetime. Session closure releases the queue and leaves
remaining files for stale-session recovery. Tests wait for actual reclamation,
not merely Weak expiration: the final reference can expire before the destructor
releases its file lease. This avoids misclassifying a correct concurrent retain
as a recovery failure.

The read-only recovery policy publishes retry interval, batch limit and closure
fallback. Both fetch and execution receipts publish optional nonnegative
`pending_abandoned_generation_count` and `removed_abandoned_generation_count`;
pending includes in-progress entries, while removed counts successful retries
within this session only. They are not durable global totals. The existing v1
schema and example retain compatibility with older receipts that lack them.

New tests cover all five partial-cleanup boundaries before the next preparation,
idle background recovery and worker restart, worker-spawn failure fallback,
deduplication and failure rotation, concurrent sweep admission, immutable identity
and lifecycle refusal, active-generation preservation, weak session release and
real non-root permission repair without closing the session or submitting work.
Mac ARM64 and physical Linux x86_64 each passed 428 Agent unit tests, nine explicit
permission tests and 12 related download/cancellation/template integrations.
The 13 default-ignored entries are those nine permission tests and four child
harnesses driven by parent tests. Mac also passed 421 native-tool tests, strict
CLI lint, formatting, dynamic package contract self-tests, documentation checks
and the source/document size audit. No retry/cleanup/queue fixture directories
remained, and daily Linux service process identities stayed unchanged.

The independent `agent-abandoned-generation-retry-source-regression` claim adds
one named local-recovery scope. The tensor now has 497 claims / 484 proven,
32 / 76 named scenarios met and 44 open. A negative-control test revokes this
proof without revoking partial-generation cleanup, session retirement or lifecycle
admission. Coordinate coverage remains 48 / 77, with 29 evidence gaps, 23 P0 gaps
and blocked release readiness.

This is current source-level Mac/Linux local-filesystem acceptance. It does not
qualify installed GUI/SDK journeys, Windows, physical power loss, arbitrary worker
panic, hostile local mutation, every filesystem error/interleaving, mixed binaries
or network filesystems. The interval and batch size do not bound filesystem I/O
duration. No App/runtime update, daily service restart, commit/push or paid model
API was performed.
