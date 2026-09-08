# Interrupted Thermal Research: 2026-09-08

## Verdict and scope

**Final installed candidate: PASS, 6/6 bounded recovery scenarios and 302/302 reference cases.**
The qualification is `mixed-thermal-inflight-recovery`, extending the
[Agent shutdown study](agent-shutdown-research-20260908.md) with actual mixed
research interrupted after heat and temperature mapping, during admitted
structural execution. All research submissions and checks use the public Rust
Headless SDK. Docker controls the faults separately; no mock or local-solver
fallback was requested by these research examples.

This is one high-contrast layered fixture under six operational scenarios, not
six new independent physical models. The complete regression remains 14 layered
plus 288 patch cases. Qualification remains
`synthetic_reference_not_material_certification`.

| Final gate | Result |
| --- | --- |
| Orchestra, transport recovery, workflow and watchdog tests | 125 pass |
| Linux CLI unit tests | 144 pass |
| Installer-installed Agent lifecycle / shutdown tests | 4 / 7 pass |
| Interrupted research example unit tests | 4 pass |
| Installed mixed-workflow scenarios | 6/6 expected outcomes |
| Fresh distributed thermal/structural reference matrix | 302/302 pass |
| Exact original-job readback before / after idle process restart | 302/302 at each stage |
| Six original recovery-job outcomes after the same restart | 6/6 preserved |
| macOS ARM64 CLI compile check, Rust 1.88 | Pass |

## Four defects, with retained red evidence

1. Workflow solve nodes discarded `retry_safety` and `replay_checkpoint` before
   dispatch. Explicit checkpoint-required work could therefore inherit a pure
   solver's retry permission. Node policy, its alias and checkpoint evidence now
   reach transport recovery; the canonical key takes precedence even when invalid.
2. Transport classification treated invalid explicit policies, including
   `checkpointed` without verified evidence, like an absent policy. Only an absent
   policy may inherit the method default. Invalid explicit values now require a
   checkpoint. A connection failure before dispatch may still select another
   Agent; uncertain send/receive/protocol failures must respect replay safety.
3. Workflow execution threw away the Agent progress callback. A real held
   structural job failed `watchdog_stalled` after 30 seconds despite live Agent
   heartbeats. Activity now refreshes job liveness at most once per second,
   without advancing completed graph nodes, rewriting result artifacts or
   resetting the original execution start. Ownership, lease and execution claim
   are checked even when a heartbeat is throttled. Stale activity, cancellation
   and the absolute execution deadline still fail closed.
4. Multi-stage restart replay emitted lower progress for its first node. The
   legitimate monotonic Job guard rejected it, and a recovered job then failed
   as `workflow execution was fenced`. Replay now retains the durable progress
   and iteration high-water marks while recording `generation`, `attempt` and
   raw `execution_progress` for each event. The monotonic guard remains enabled;
   recovery does not obtain a fresh total execution budget.

Initial policy tests had 4 failures out of 11. Activity tests reproduced two
failures. The progressed-workflow restart regression failed against the earlier
candidate, then passed after the fix. Setup attempts with missing Hex cache or
an incorrect Cargo path are retained separately and are not counted as product
failures. Failed research jobs remain unchanged historical evidence:
`2c29c41c08a1385a` (watchdog) and `b73ef17213207086` (replay progress).

## Final real fault matrix

Each request has seven graph nodes and uses the same 153-node, 128-Q4-cell
two-material fixture: conductivity 2/200 W/(m K), reference 293.15, rise 20 K.
The exact generated job id and `solve_thermal_plane_quad_2d` identify the hold.
The heat solve and bridge must already be visible in persisted progress before
any signal is sent. The hold is before structural numerical computation, not
inside a solver iteration or a numerical checkpoint.

| Evidence directory under `evidence/` | Original job id | Observed outcome |
| --- | --- | --- |
| `qualified-baseline` | `65d75cc9f91d9f46` | Completed, generation 1 |
| `qualified-graceful` | `771ce58d5c74eb72` | Live after the stale threshold; SIGTERM drain completes, exit 0 |
| `qualified-agent-loss` | `943ae08b8dae4ef7` | SIGKILL secondary; primary accepts exactly one retry; completed |
| `qualified-agent-blocked` | `e713a286c5883700` | SIGKILL primary; checkpoint-required failure, no backup dispatch |
| `qualified-orchestra-replay` | `f97b923335bbb541` | SIGKILL Orchestra; same job completes in generation/attempt 2 |
| `qualified-orchestra-blocked` | `600c8a4ad51bb2cd` | SIGKILL Orchestra; checkpoint-required replay blocked, generation 1 |

At the long-running observation, job execution elapsed 40,227 ms and the held
Agent execution elapsed 38,346 ms with only 10 ms of activity silence. It was
still solving at durable progress 3/7. The subsequent signal closed admission,
the marker was released, and the admitted structural result survived the drain.

For Agent loss, backup total-started counts changed 3 to 4 when retry was allowed,
and stayed 0 to 0 when it was forbidden. The forbidden result retains
`agent_retry_blocked`, `checkpoint_required`, and no successful structural output.
Intentional kills exited 137 without OOM, not an inferred network or memory fault.

Orchestra PID changed 1756166 to 1778419 during the allowed replay. Generation 2
recorded raw input progress 1/7 while durable progress stayed 3/7, then finished
with one terminal commit. **Both old and new structural executions briefly
coexisted on an Agent.** Their active count later returned to zero; the final
generation stayed unchanged. This is result-commit fencing, not exactly-once
computation or automatic cancellation of all orphaned work.

For forbidden Orchestra replay, Agent process identities stayed unchanged and
started counts stayed 3/3 before and after. Releasing the old execution did not
change the blocked result. The checker observes terminal stability after 600 ms;
separate late observations and a later complete process restart extend that
observation, but do not prove stability for every future failure pattern.

Positive scenarios validate output contracts and analytical temperature,
displacement, mapping and flux, then compare all nodal/element physical arrays
and the bridged model exactly with the uninterrupted baseline. Maximum errors
were 6.310e-12 K for temperature/mapping, 6.807e-17 m for axial displacement and
7.687e-9 W/m2 for flux, below unchanged gates 1e-7 K, 1e-9 m and 7.921e-5 W/m2.
Expected heat flux is 79.20792079207921 W/m2 and tip expansion is
0.000051485148514851464 m. Negative outcomes are safety passes, not numerical passes.

## Deployment, provenance and retention

Base revision is `07919a437b2f537b0bea77f7ef5ee51f71a01062` plus this source overlay.
Native Installer seals and installs the separate headless candidate, activation
generation 1, label **3.1.2**. Rust manifests/public SDK and the Mix release retain
their actual **3.0.0** metadata. These labels do not claim repository-wide version
alignment or promotion of the existing installation.

One Linux host runs one Orchestra and two Agent containers: non-root uid 1001,
read-only installed payload and root filesystem, separate writable state, no
capabilities, no-new-privileges, init, four CPUs per container, 4 GiB for Orchestra
and 2 GiB per Agent. Ports 6420/6421/6422 are loopback-only. Orchestra uses SQLite
and distributed Agent dispatch with transitional adapters disabled. Builds use
Rust 1.95.0 and Elixir 1.19.5 / OTP 28.5.0.2. This exercises Installer validation,
not a complete SSH deployment rollout, multi-host partition or PostgreSQL HA.

Large evidence remains in managed server state under
`research-runs/interrupted-thermal-20260908/`, not Git. The initial `config`,
heartbeat-fixed `config-final`, and fully fixed `config-qualified` installations
are separate; earlier payloads were not overwritten. The `qualified-*` evidence
selects the final installation. Requests, results, snapshots, process inspection,
logs, installed-test outputs and source archives are retained. Test additions
and a usage-text correction followed scenario execution; successful SDK command
behavior and deployed production sources did not change afterward.

| Evidence | SHA-256 |
| --- | --- |
| Installed Agent | `79fe984c6c6e66d41ef0edb10d38e9ac15a7e2843ed710304bfcecde25f43ec6` |
| Installed payload manifest | `0399dae6b477d797917fd6def525189c0fcb613ffab0991f1a55864593805295` |
| Installed recovery coordinator BEAM | `88674981c1999d907048b0a080f74909ed7bad66b768dcc357d3bf436152e26d` |
| Final source overlay archive | `d9bb8cd9ba78a376a0a19d49fc6292f79f45e7fe20b380169ffb5da1afcdfc12` |
| Executed interrupted-research SDK example | `7721270a29692ecc57ff24600a66dfb7a498009a8d8a7f10cc8082ddb2a5f3b0` |
| Full 302-case report | `25cb9cef9511815108f757a9f855785affa3b8f1e91144809e8f83733edb8d3f` |
| Original-job readback, before / after restart | `8dd5fe1abadb18faee5beb92e74820da38f07109b3e666f2885d8538102b7fee` |

SHA-256 identifies retained files, not an external signature. The original
deployment-server, Orchestra and Agent PIDs 3461/3462/3463 were not replaced or
restarted. Temporary research containers are removed after evidence capture;
isolated installations, database and evidence remain for reproduction.

Documentation inventory, the 25-page HTML book, Rust/Elixir formatting, project
organization, and tensor self-test/structure checks pass. The source limit stays
800 lines and documentation stays below 2,000 lines. The tensor's overall
`daji status` remains `blocked`; the new scoped evidence does not promote it.

## Reproduction and next boundaries

Use the [HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [public SDK example](../sdks/rust/examples/interrupted_thermal_research.rs).
`submit`, `capture`, `await-held` and `verify` are this example's phased controls,
not a mandatory generic CLI or a replacement for the Headless SDK. An explicit
`KYUUBIKI_BASE_URL` is required. Every evidence directory must be new. The checker
never restarts services or creates replacement jobs. Fault injection must target
a reviewed isolated runtime, never a user's active study.

Still unqualified by this round: mid-iteration checkpoints, physical power loss,
database corruption, material calibration, external-solver comparison, large
mixed-workflow meshes, Windows lifecycle, multi-host mesh failover, and exactly-once
side effects. TaskIR-specific retry-policy overrides need their own end-to-end
qualification; these solve-node results must not be generalized to every SDK
dispatch path. Orphaned execution cancellation and capacity reconciliation during
Orchestra replay remain a particularly important follow-up, even though stale
results did not overwrite the accepted generation here. The tensor gains scoped
recovery anchors, not a blanket maturity or release-status upgrade.
