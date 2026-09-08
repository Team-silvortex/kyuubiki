# Orphaned Execution And Capacity: 2026-09-08

## Verdict and scope

**Final installed candidate: PASS, 3/3 research outcomes and 302/302 reference cases.**
The qualification is `mixed-thermal-orphan-capacity-recovery`, following the
[interrupted thermal study](interrupted-thermal-research-20260908.md). That earlier
candidate fenced stale results but allowed old and new structural executions to
overlap after an Orchestra restart. This round closes a bounded part of that gap:
native execution admission, guard-scoped cancellation and safe capacity waiting.

All research jobs use the public Rust Headless SDK against Installer-installed
Linux services, with distributed dispatch and transitional adapters disabled.
Faults are controlled separately. There is no requested local-solver or mock
fallback. The physical qualification remains
`synthetic_reference_not_material_certification`.

| Gate | Result |
| --- | --- |
| Orchestra, Agent client/gate, workflow recovery and watchdog tests | 136 pass |
| Linux CLI unit tests | 147 pass |
| Installed Agent lifecycle / orphan / shutdown live tests | 4 / 6 / 7 pass |
| Existing TaskIR solve, tamper rejection and subsequent recovery over TCP | 1 pass |
| New installed research baseline / allowed restart / blocked restart | 3/3 expected outcomes |
| Fresh distributed layered / patch physical cases | 14/14 + 288/288 pass |
| Exact 302-job readback before / after idle restart | 302/302 at each stage |
| Three original research results after idle restart | Exactly unchanged |
| macOS ARM64 CLI compile check, Rust 1.88 | Pass |

These are not 305 independent material models. The three fault scenarios use
one seven-node workflow and the same 153-node, 128-Q4-cell high-contrast fixture.
The 302-case suite is the existing bounded analytical layered/patch matrix.

## Defects and fixes

1. **Agent capacity was not enforced locally.** Orchestra's scheduling ledger
   can lose in-flight ownership across restart; direct clients can also bypass
   that ledger. Native admission now shares the locked lifecycle lease count
   for solver and TaskIR requests. `KYUUBIKI_AGENT_MAX_ACTIVE_EXECUTIONS` defaults
   to 1, permits 1..=1024 and rejects invalid startup configuration. Slots remain
   reserved through execution and final response delivery. A full Agent returns
   `agent_at_capacity` before admitting work; control requests remain usable.
2. **Transport cancellation did not leave the pre-computation hold.** A failed
   heartbeat now marks cancellation on the exact execution guard. Cooperative
   checks run before TaskIR decoding / solver computation, after solver input
   decoding, and inside the test hold. An old guard cannot poison a reused
   request id or another execution of the same job. Explicit cancellation also
   exits the hold without marker release and is checked before invalid inputs
   can be decoded or evaluated.
3. **Orchestra treated native saturation as a terminal RPC failure.** This
   definite non-admission may now wait or select another eligible Agent without
   consuming replay permission or penalizing Agent health. One deadline starts
   at the first capacity rejection, using the existing queue timeout. All-busy
   retries pause up to 50 ms; expiry returns `agent_capacity_timeout`. Dispatch
   authorization is checked again, and leases are released on every attempt.
   Uncertain transport failure still follows the existing replay/checkpoint rules.

The new live suite first reproduced four failures against the pre-fix native
candidate. The control-plane capacity suite likewise had four failures against
the old client. After fixing the product paths, all six expanded native tests
and all four control tests pass. The suite includes 12 concurrent rejected
clients behind one admitted task, explicit capacity 2 with two executions of
the same job, cancellation before solver/TaskIR decoding, invalid capacity, and
successful work after the failure. Unit tests additionally retain a stale guard
while a request id is reused, and keep a slot until the last response lease drops.

Execution handling moved from `rpc.rs` into `rpc_execution.rs`; envelope validation
and method routing remain in `rpc.rs`. This is an internal split, not a new
public SDK or a change to the TaskIR execution format. The capacity descriptor
is read-only `kyuubiki.agent-execution-admission/v1`.

## Installed process-fault evidence

| Directory under `evidence/` | Original job id | Outcome |
| --- | --- | --- |
| `baseline` | `48a0404f68d8885c` | Completed at generation 1; exact physical match to the prior qualified baseline |
| `orchestra-replay` | `8889871ad9cea56e` | Original job completes at generation/attempt 2, one terminal commit |
| `orchestra-blocked` | `690a93599fd8baee` | `checkpoint_required` replay blocked, generation/attempt 1, no successful structure output |

Both fault scenarios first retain an `await-held` snapshot: upstream heat and
bridge progress already exist, and the Agent has accepted the matching
`solve_thermal_plane_quad_2d` request. The exact-job hold marker is bounded to
120 seconds and sits before numerical computation, not inside a solver iteration.
The owned Orchestra container is then killed with SIGKILL and restarted. Both
kills exit 137 without OOM; the Agent process is not restarted during either fault.

In the replay case, old request `5c65a57ec6a57315` records
`cancelled: execution cancelled before computation`. Without releasing the
marker, generation 2 completes its heat/bridge nodes and holds new structural
request `8fcf90711cc96e4a`. The old request is absent from active executions;
the native admission count is 1 at the held observations, with the same Agent
process identity. Marker content was compared with the original job id before
removal. After release, the new generation completes and active count returns
to zero. Single observations alone cannot prove every intermediate count;
the atomic admission unit and concurrent live tests exercise that invariant.

In the blocked case, old request `2a859cfc63c3dbeb` cancels while its marker is
still present. The native active count reaches zero. Total-started remains 8
and total-completed remains 6 across the restart, so no replacement execution
or late successful computation is inferred from a terminal label. Releasing
the marker afterward does not change the failed job or its blocked result.

Positive outcomes validate output contracts, analytical temperature, reference
mapping, expansion and flux, then compare all physical nodal/element arrays and
the complete bridged model with the uninterrupted reference. Maximum errors
remain 6.310e-12 K, 6.807e-17 m and 7.687e-9 W/m2, below unchanged gates
1e-7 K, 1e-9 m and 7.921e-5 W/m2. A safe blocked outcome is not a numerical pass.

After the full 302-case run and exact readback, both idle candidate processes
stop with exit 0 and restart with new identities. Every original matrix result
matches the complete retained JSON value through the read-only SDK verifier. The
three fault-study result files also match before/after exactly. Fresh Agent
total-started is 0: readback and recovery of terminal records do not silently
dispatch new calculations. Each phased research verification also checks
terminal stability after 600 ms; the later process restart extends observation,
not a proof for every possible future fault.

## Provenance and retention

Base revision is `07919a437b2f537b0bea77f7ef5ee51f71a01062` plus the retained
source overlay, not a clean checkout claim. Native Installer seals and installs
the isolated headless payload, activation generation 1, label **3.1.2**.
Rust/SDK manifests and the Mix release still report their actual **3.0.0**
metadata. This round does not align repository versions or promote the package
over the existing system installation.

One remote Linux physical host runs one Orchestra and one Agent container:
non-root uid 1001, read-only installed payload/root filesystem, separate writable
state, init, no capabilities, no-new-privileges, four CPUs each, 4 GiB / 2 GiB
memory limits. HTTP/TCP bind only loopback ports 6430/6431. The control plane
uses SQLite. Build runtimes are Rust 1.95.0 and Elixir 1.19.5 / OTP 28.5.0.2.
This is Installer payload qualification, not an SSH rolling deployment or HA test.

Large files stay in managed server state under
`research-runs/orphan-research-20260908/`. `evidence/` retains inputs, original
job identities, SDK results, held/late snapshots, restart inspection, installed
live-test observations and the full matrix/readback reports. `verification/`
inside it retains red/green logs, process gates and the source overlay. The prior
study's installation and failed evidence are not overwritten. No server
credential or deployment configuration is added to Git.
Both owned candidate containers were stopped successfully and removed after
evidence capture. Original deployment/Orchestra/Agent PIDs 3461/3462/3463 and
their start times are unchanged; unrelated service containers remain running.
The isolated installation, database and evidence remain on the server.

| Retained file | SHA-256 |
| --- | --- |
| Installed Agent | `bef940944d45e74fe409a8e052876f3dd58af72a5d8b40b615662cdcb084c08a` |
| Installed payload manifest | `f0f5cdcd00baf2174752c17dc7d886dcd177ab09bbab318eb5e30eb89bb97120` |
| Installed AgentClient BEAM | `a0efdb8b50dd595fbe908af2bf0841cc61b4891c411a67fc1294f67858868cb3` |
| Production/test/SDK source overlay | `b308ec69bd677e937e62ecc819e0d2e0be2cc41e22cdfdd353f25f3c1ea38973` |
| Executed phased research SDK example | `7721270a29692ecc57ff24600a66dfb7a498009a8d8a7f10cc8082ddb2a5f3b0` |
| Full 302-case report | `cc79255021c680e5ad18de31a25e6f9e2a49f928030f37cf0523704c0a4f4d0f` |
| Original-job readback, before / after restart | `aaf813924abc493496fd9e08bbcbcae8b695243bc8a73ac51525e33e850c5913` |

Digests identify evidence; they are not signatures or material certificates.
The 21 overlaid source/test files match local and remote digests. Documentation
inventory, the 25-page HTML book, Rust/Elixir formatting, tensor self-test and
structure, and project organization checks pass. The global tensor still reports
`daji status=blocked`; this scoped pass does not override that release judgment.

## Reproduction and remaining boundaries

Use the [HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [runtime admission contract](../docs/agent-orchestrator-boundary.md#native-execution-capacity-and-orphaned-work).
For the additional regression, run `cargo test -p kyuubiki-cli --test agent_orphan_live`
in the Rust workspace. `KYUUBIKI_TEST_AGENT_BINARY` can select the Installer-installed
binary; `KYUUBIKI_TEST_AGENT_EVIDENCE_DIR` retains owned test-process observations.
The existing TaskIR qualification test separately uses its Cargo-built CLI.
The control-plane tests live in `test/kyuubiki_web/playground/agent_capacity_retry_test.exs`.
All fault processes must be isolated and explicitly owned; never signal a user's study.

Still unqualified: cancelling an arbitrary running solver iteration, every
job-wide cancellation combination at capacity greater than one, full restoration
of Orchestra's old execution ledger, automatic adoption of native capacity,
network blackholes, multi-host partitions, numerical checkpoints, exactly-once
side effects, physical power loss, database corruption, material calibration,
external-solver agreement, larger mixed meshes and installed Windows/macOS lifecycle.
An uncooperative solver retains its slot until returning; it is not terminated
unsafely to make the counter appear idle. Capacity is not a TCP/thread/byte limit.
Transport-loss cancellation here observes job-bound heartbeats, not every RPC's
connection independently.
The added tensor anchor is scoped evidence, not a blanket module-maturity upgrade.
