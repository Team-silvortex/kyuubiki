# Agent Termination and Research Acceptance: 2026-09-08

## Verdict and scope

**Installed signal-drain candidate: PASS, 302/302 reference cases.** The native
Agent now connects handled termination signals to its existing execution-drain
contract. This extends the [reply-delivery qualification](agent-reply-delivery-research-20260908.md),
without replacing its historical results or claiming that every interruption
of a multi-node research workflow is automatically recoverable.

| Gate | Final result |
| --- | --- |
| CLI unit tests on Linux | 143 pass; zero failures or ignored tests |
| Installer-installed Agent lifecycle tests | 4 pass |
| Installer-installed Agent termination tests | 7 pass |
| Public Rust SDK, real distributed thermal/structural study | 302/302 pass |
| Original-job readback before/after both process restarts | 302/302 pass at each stage |
| Fresh layered study after restart | 14/14 pass |
| Native Agent directly as Docker PID 1, idle | Graceful exit 0; stop command 0.05 s |
| Linux Rust workspace, default features, all targets | Offline locked compile check passes |
| macOS ARM64, Rust 1.88 | Offline locked CLI compile check passes |

There are still **302 unique physical cases**, not the sum of repeated checks.
The qualification remains `synthetic_reference_not_material_certification`.
Numerical expectations, output contracts and tolerances are unchanged from the
[thermal study](thermal-service-research-20260908.md). The full run took 74.90 s;
the fresh recovery layered run took 5.41 s. These are observed end-to-end times,
not a new statistical performance benchmark. No mock or local-solver fallback
was requested for these SDK research runs.

## Failure reproduced and corrected

Five new live termination tests failed against the preceding implementation.
SIGTERM cut the in-flight RPC connection instead of preserving its result; an
existing drain owner had no irreversible shutdown boundary; no structured
deadline failure was retained; idle termination was signal-killed rather than
graceful; and invalid shutdown budgets were ignored. These red results remain
in the evidence rather than being replaced with final successes.

The corrected behavior is:

1. A handled termination request closes execution admission under the existing
   lifecycle lock. An already-admitted execution keeps its lease through result
   delivery. New executions receive `agent_draining`, while ping and inspection
   remain available during draining.
2. Shutdown is irreversible for that process. An existing Installer drain owner
   and generation are preserved, but even that owner cannot resume admission or
   hand it to another controller: the response is `agent_shutdown_in_progress`.
   Repeated signals do not restart the timer or prematurely force success.
3. `KYUUBIKI_AGENT_SHUTDOWN_TIMEOUT_MS` covers draining and background cleanup
   together. Default is 30,000 ms; only integers 1..=300000 are accepted. The
   descriptor and registration/heartbeat payloads expose `shutdown_policy`.
   A blocked cleanup task cannot obtain a fresh timeout window.
4. Graceful completion exits 0. Deadline failure exits 1 and retains structured
   `kyuubiki.agent-shutdown/v1` evidence with phase, reason, unfinished execution
   identities and recent failures. It does not manufacture a successful result.
   Normal RPC acceptance stays blocking/event-driven without a per-request poll.

Signal registration uses the cross-platform
[ctrlc 3.5.2 interface](https://docs.rs/ctrlc/3.5.2/ctrlc/), rather than adding an
ad hoc unsafe production signal handler. Its dedicated callback thread can
safely enter the lifecycle lock. The dependency lock includes the required nix
and platform dependencies; nix requires a newer libc than the previous locked
0.2.184. The updated lock was checked across the Linux Rust workspace and with
minimum-supported Rust 1.88 on macOS. That is not Windows runtime qualification.

## Live boundary tests

The shared lifecycle fixture now lives in `tests/support/agent_lifecycle.rs`, so
existing replacement/backpressure and new termination tests use the same framed
RPC client and process cleanup. `KYUUBIKI_TEST_AGENT_BINARY` explicitly selects
the sealed, installed executable for acceptance; the default remains Cargo's
test binary. `KYUUBIKI_TEST_AGENT_EVIDENCE_DIR` retains isolated test logs when
requested. These are test-fixture controls, not new runtime deployment settings.

- A job-scoped hold keeps a real bar request admitted while SIGTERM arrives.
  New work is rejected, ping succeeds, repeated SIGINT does not short-circuit
  draining, and releasing the hold allows the real solver to return stress 10
  and displacement 0.01 before graceful exit. The hold is before numerical
  computation; this is not solver-iteration checkpoint or preemption evidence.
- An existing Installer-owned drain cannot be resumed after termination. Its
  original owner and generation remain identifiable, and admitted work finishes.
- A held job exceeding a 500 ms shutdown budget exits 1. Its timeout event
  retains `signal-timeout-job`, `solve_bar_1d`, and one active execution. The
  interrupted request fails rather than producing a result. An explicitly
  started fresh process then accepts and solves a new request correctly.
- A 100,000-element bar creates a final response body larger than 16 MiB. The
  receiver deliberately stops after its header, then sends SIGTERM. The Agent
  remains non-replaceable and records `signal-pending-response` on the 1,000 ms
  shutdown timeout, without a completion event. This is a transport-backpressure
  test; the intentionally unread large result is not numerical acceptance.
- Idle termination does not wait for a client holding half a request header.
  Additional idle runs exercise SIGINT and SIGHUP through the graceful gate.
- Zero, nonnumeric and over-limit shutdown budgets fail before listening.

The final installed-test log contains 22 structured shutdown events, including
six completed shutdowns and two explicit deadline failures. The two measured
deadline events report elapsed times of 500 and 1,000 ms respectively. Those
observations are not a hard real-time guarantee under arbitrary kernel or I/O
failure. Blocking-background-cleanup and repeated-request behavior also have
focused unit coverage. Interrupted cleanup under a real mesh partition remains
a separate operational test, not something inferred from the unit test.

## Installed research and process lifecycle

Native Installer sealed and installed a separate headless payload at activation
generation 1. Both services ran from that immutable installation in read-only,
unprivileged, capability-dropped Docker containers with CPU/memory limits and
separate writable state. Research ports 6410/6411 were host-loopback only. This
uses Installer payload validation, but not a complete remote Installer rollout.

Before the controlled restart, jobs, execution leases and queued requests were
all zero. Agent PID changed from 1611041 to 1623942; Orchestra PID changed from
1610821 to 1623868. Container ids and installed mounts stayed the same. Both
processes exited 0, neither OOM-killed. Agent's stop command took 0.12 s; its
event recorded completed cleanup at 77 ms. Orchestra's fencing token advanced
from 1 to 2 and its session identity changed.

The original 302 persisted jobs passed read-only verification before and after
that restart, with exact JSON values, graph contracts and numerical gates. No
replacement jobs were submitted by the verifier. A separate fresh 14-case run
then passed. Identical readback-report hashes are expected because the same jobs
are compared; the separate lifecycle records prove the restart occurred.

A separate installed-binary probe listened on loopback port 6412 with Docker
init disabled, then handled termination and exited 0 in a 0.05 s stop command.
This closes the earlier idle PID-1 signal-handling defect, not child-process
reaping or in-flight hard-kill recovery. Production containers should still use
init. Supervisors must permit longer than the Agent's configured shutdown budget,
for example a 40-second supervisor window for a 30-second Agent budget.

## Provenance and reproduction

Base source is `b385f2fa03144f741598d6ccdd01b24002454d01` plus the retained
reply-delivery work and this signal-drain change. Candidate payload is **3.1.1**,
the unchanged complete Mix release is from the prior **3.1.0** candidate, and
Rust manifests/public SDK remain **3.0.0**. These are recorded version labels,
not an assertion of repository-wide version alignment or production promotion.
The native Linux build used Rust 1.95.0. All large artifacts remain in managed
server state under `research-runs/agent-shutdown-20260908/`, not in Git.

| Evidence | SHA-256 |
| --- | --- |
| Installed Agent executable | `e3387c75fc88e74743a2d9e04a7bf99552fd089fd28b70a184a3beb8dcabd3a5` |
| Installed payload manifest | `e069ceae9cf183fc6cd82dc7370e9b595df098de81245b8b24cac10ce638dc2f` |
| CLI source and workspace lock archive | `88f58a903058d12003510fddbd016023223a7eecb4619901c8a18b40e8a071a3` |
| Full 302-case report | `eb9ae4b95cc1daf44b26e3cdbf8077eb8eb3fb35a7ee85ceb94ad7f1fbe81efd` |
| Before/after readback report, each | `c888390424382920b20d7fd4e52193ce08162d2735b3b67962df8c6fefbaf044` |
| Fresh recovery report | `02aa0e42868d0dbf71921b7e05ceee94cfe327c469f2eb31f359a48274435d6f` |
| Final installed-test shutdown events | `4a6b9d1a7d1edecda4af8adf5c1a6b2cebc9f5ffdc8bbc55d22c688cd25accdc` |

The final directories are `evidence/all`, `evidence/readback-before-restart`,
`evidence/readback-after-restart`, `evidence/recovery-layered`, and
`evidence/installed-agent-final`; build, deployment, lifecycle and governance
records are in `evidence/verification`. Earlier `installed-agent` results and
the red tests remain retained. The first full-workspace compile attempt lacked
a historical Installer fixture in the selectively staged checkout; copying that
existing fixture allowed the final check to pass without changing product code.
Digests identify files; they are not externally signed attestations.

All three temporary test containers were removed after their logs and final
inspection records were retained. The isolated installed payload, database and
research evidence remain on the server for reproduction. The pre-existing Agent,
Orchestra and deployment-server processes retained their original identities;
this round did not restart or replace them.

Documentation inventory, the 25-page HTML book check, Rust workspace formatting,
project organization, and tensor self-test/structure checks passed. Source and
documentation limits remain 800 and 2,000 lines. The tensor's separate overall
`daji status` remains `blocked`; passing these specific anchors does not promote
the global release or qualify the untested recovery boundaries below.

```sh
cargo test --locked --manifest-path workers/rust/Cargo.toml \
  -p kyuubiki-cli --bin kyuubiki-cli --test agent_lifecycle_live --test agent_shutdown_live
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example layered_thermal_research -- research-round-005 all
cargo run --locked --manifest-path sdks/rust/Cargo.toml \
  --example verify_layered_thermal_research -- research-round-005 research-readback-003
```

Configure `KYUUBIKI_BASE_URL` for a reviewed isolated pair and use new output
directories. The verifier does not restart services; preserve separate process
identities when performing a controlled restart after jobs finish.

## Remaining boundaries

This proves bounded graceful termination of admitted Agent work, not seamless
continuation of every future workflow node. The Agent does not automatically
replay a timed-out request. Existing Orchestra replay/checkpoint contracts and
[distributed recovery evidence](../config/architecture/module-function-coverage-evidence/runtime-recovery.json)
remain relevant but must be exercised with real mixed research workflows before
claiming this complete shutdown-to-takeover path. SIGKILL, physical power loss,
database corruption, durable receiver acknowledgements, exactly-once effects,
Windows service lifecycle, and material/experimental validation are not qualified
by this round. The tensor gains explicit anchors, not a blanket maturity upgrade.
