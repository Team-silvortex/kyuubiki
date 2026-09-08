# Agent Reply Delivery Research: 2026-09-08

## Verdict and scope

**Final installed candidate: PASS, 302/302 reference cases.** This round fixes
the boundary between computation, response delivery and Agent draining. It
reuses the official Rust Headless SDK and real distributed thermal/structural
workflows from the [installed-service study](thermal-service-research-20260908.md).
No mock solver or local-solver fallback was requested for these research runs.

| Final-build gate | Result |
| --- | --- |
| Layered before/after comparison | 14/14 pass on each build; identical requests and physical artifacts |
| Full reference study | 302/302 pass; all output contracts and immediate readbacks pass |
| Original-job readback before controlled restart | 302/302 pass |
| Original-job readback after both process restarts | 302/302 pass; exact retained JSON values |
| Fresh layered jobs after restart | 14/14 pass |
| Linux native Agent unit tests | 137 pass, zero failures or ignored tests |
| Linux native Agent lifecycle integration tests | 4 pass, including slow-consumer and two-Agent replacement tests |
| macOS ARM64, Rust 1.88 | Offline locked CLI compile check passes; not live-test qualification |

The unique physical matrix is still **302 cases**, not the sum of repeated runs
and readbacks. Its qualification is `synthetic_reference_not_material_certification`.
The 14 layered cases and 288 small plane-stress patches retain their existing
analytical tolerances. This is not experimental validation or a large-mesh
physics benchmark. Earlier reports and candidate results are not overwritten.

## Defects and regression boundaries

1. **Computation finished before result delivery, but the execution was already
   counted as completed.** Draining could report `safe_to_replace` while the
   final response was still pending. Solver and TaskIR requests now retain the
   same execution lease until response writing completes or fails. Two new
   tests failed against the old implementation before the fix. Write failures
   retain request/job/method identity as `result_delivery_failed`; abandoned
   writers record `result_delivery_aborted`. A prior solver error stays primary.
2. **A slow receiver could indefinitely hold a response write.**
   `KYUUBIKI_AGENT_REPLY_TIMEOUT_MS` now sets a total response-write budget,
   default 10,000 ms, valid integer range 1..=300000. The deadline starts before
   response-frame serialization and is shared across progress and final writes;
   each socket write receives only its remaining budget. Serialization is not
   preempted. Heartbeat writes also have a bounded frame budget. Invalid values
   fail startup, and `describe_agent` exposes the effective read-only policy.
3. **Short jobs waited for a sleeping heartbeat thread to exit.** Completion
   and unwinding now wake the parked heartbeat before joining it instead of
   waiting for the one-second interval. A deterministic parked-thread test
   checks wakeup; it is not a solver-throughput threshold.
4. **A failed heartbeat frame could leave a connection usable for another
   frame.** Final review added a red test showing that a subsequent response
   could still be written after a failed frame. Failed frame writes now shut
   down the connection immediately. A partial frame must never be followed by
   a fresh final response on the same stream. All final-build gates above were
   rerun after this additional fix.

The real slow-consumer test solves a 100,000-element bar and stops reading after
the large final-frame header (body over 16 MiB). While delivery is blocked,
drain reports one active execution and refuses replacement/new work, while
ping/control requests remain responsive. A five-second configured write budget
then releases the lease with an explicit delivery failure, not a completed job.
After admission resumes, a fresh bar request returns stress 10 and displacement
0.01. The large body is intentionally not accepted as a physics result: this
test qualifies transport backpressure, not 100,000-element numerical accuracy.

Successful socket writes mean `final_response_written_to_transport`, not a
durable receiver acknowledgement. The descriptor explicitly reports
`durable_receiver_acknowledgement: false`. Database persistence, same-job SDK
readback, automatic retries and exactly-once effects remain separate contracts.

## Measurements and restart evidence

| SDK study | Wall time | Numerical verdict |
| --- | ---: | --- |
| Before: previous installed Agent, layered | 33.22 s | 14/14 pass |
| After: final installed Agent, layered | 5.44 s | 14/14 pass |
| Final installed Agent, full matrix | 74.94 s | 302/302 pass |

The paired layered run reduced end-to-end wall time by about 83.6%. It used the
same SDK, study definition, Orchestra release/database, Agent identity, host,
resource limits and unoptimized native build profile. Before ran first, so
cache/order noise is not eliminated. This single paired measurement is not a
statistical throughput benchmark or a sixfold increase in solver arithmetic
speed. The two solver RPCs per layered case explain the old fixed heartbeat
waits. Intermediate candidates also passed, but do not replace final-build data.

All 14 request digests, numerical gates and seven physical artifacts per case
are identical across the before/final pair. Whole result documents from
different submissions are intentionally not hash-equal: job ids, timings,
progress and recovery provenance differ. In contrast, the read-only verifier
requires exact equality when fetching the **same** persisted job.

Before the final controlled restart, active jobs, execution leases and queued
requests were all zero. Agent PID changed from 1566563 to 1573003; Orchestra PID
changed from 1558048 to 1572927. Their container identities and mounted payloads
were retained. The Orchestra fencing token advanced from 2 to 3 and its session
id changed. Agent exited 143 and Orchestra exited 0, neither OOM-killed. These
containers use Docker `--init`; this is idle signal delivery, not automatic
SIGTERM-driven task draining. The 302 original jobs then passed read-only
verification without resubmission, followed by 14 new successful layered jobs.

## Provenance and reproduction

Base source is `b385f2fa03144f741598d6ccdd01b24002454d01` plus this round's
Agent changes. The final native Agent was built on Linux with Rust 1.95.0,
sealed and installed by the native Installer into a separate activation store
(generation 1). Candidate payload version is **3.1.1**; Rust manifests/public SDK
still say **3.0.0**, and the unchanged complete Mix release was reused from the
previous **3.1.0** candidate. This is explicit test provenance, not a claim that
repository-wide version alignment or production promotion has been completed.

The final Agent uses the separately installed `config-final` payload; Orchestra
continues using the identical Mix release from the earlier `config` payload.
Both run read-only under an unprivileged user, dropped capabilities,
no-new-privileges and CPU/memory limits. Only host-loopback ports 6410/6411 are
used. Writable cache, SQLite database and release state are separate. Docker
supervision is not a substitute for a complete remote Installer rollout test.

Raw evidence stays in managed server state under
`research-runs/agent-reply-20260908/`, not in Git. The final directories are
`evidence/final-layered`, `evidence/final-all`,
`evidence/final-readback-before-restart`, `evidence/final-readback-after-restart`
and `evidence/final-recovery-layered`. `evidence/before-layered` retains the old
Agent comparison; `after-*` and unprefixed `readback-*` retain intermediate
candidate evidence. Build/test/activation/lifecycle logs and comparisons are
retained in `evidence/verification`. File hashes identify evidence, not an
external signature or independent attestation.

| Evidence | SHA-256 |
| --- | --- |
| Study definition | `245beec6daf0c4647ecaba7f4e5606a9fa17fa325a481e12e34773d7bffe218c` |
| Final installed Agent | `dbee8524581f617472bccdb2d78b4407f1d77d18ce0e210a326fc15d53464984` |
| Final installed payload manifest | `afa89e03b0d85f853ffee8e968d7d6bf96262d6d03710a250b9f498d62e5d2c2` |
| Changed Agent source archive | `6053c7640c45b680589d79c7e667bd062beb08a7f2d6a5f7e3dd59929e3e36ba` |
| Before layered report | `9143ac204df9da6920f4ac2dc955d01560c7b6a64fb326b9bac2b6d830da4bee` |
| Final layered report | `847bf92f604953b9a8803ad691ebf2ad9119e485f05569cf07da60591cb84ce1` |
| Final full report | `3b186dba4b56d640ecd5ffae19dcadad59e8516974d7311aedbff68f4ea861e0` |
| Final before/after readback report, each | `f22932a13307182f1c9f0687fad55acfa31f1bcd466279b5e31276b0faf5280f` |
| Final fresh recovery report | `86b9623d4142eb3c4a63e2be94b03ef4c0b7120619c6e1147db54a5369a3ff85` |

Run against a reviewed isolated service pair with `KYUUBIKI_BASE_URL` configured:

```sh
cargo test --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-cli --bin kyuubiki-cli --test agent_lifecycle_live
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example layered_thermal_research -- research-round-004 all
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example verify_layered_thermal_research -- research-round-004 research-readback-002
```

Use a new evidence directory for every run. Capture health/process identities
before and after an externally controlled restart; the verifier never restarts
services. Keep package sealing and installation sequential. An early install
attempt during final resealing correctly failed the Agent digest check without
activation; installation succeeded only after sealing completed. An initial
HTTP-fixture run lacked the selectively staged `apps/web` directory; supplying
the real fixture tree and offline Mix test build yielded 3/3 Headless HTTP and
1/1 TaskIR HTTP fixture passes. Those use fixture agents, not research solvers.
Both setup failures remain in retained logs rather than being concealed.

Final local and remote changed-source digests match. Documentation inventory,
HTML book checks, tensor self-test/structure checks, project organization,
Rust formatting and diff whitespace checks pass. The organization audit reports
no source/doc line-limit debt. The tensor's global `daji status` remains
`blocked`; passing structural checks do not clear its separate release gates.

After retaining logs, all four candidate containers were stopped and removed.
Installed payloads, database and research evidence remain on the server.
Existing Agent, Orchestra and deployment services stayed active with unchanged
PIDs 3463, 3462 and 3461. None was upgraded or restarted by this round.

## Remaining boundaries

The coverage tensor links these contract, validation and bounded-performance
anchors without promoting global maturity, release, power-loss or recovery
claims. There is still no automatic OS-signal-to-drain adapter in the native
Agent. Mid-computation kill/cancel recovery, durable receiver acknowledgements,
exactly-once retries, power loss/database corruption, multi-Agent research
scaling, Windows runtime execution and full remote Installer promotion need
their own evidence. Material calibration, external-solver/experiment agreement
and general nonlinear/distorted-mesh convergence are also not established by
these small synthetic references.
