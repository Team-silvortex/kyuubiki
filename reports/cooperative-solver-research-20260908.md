# Cooperative Numerical Cancellation: 2026-09-08

## Verdict and scope

**Installed numerical cancellation: PASS, 3/3 expected research outcomes.**
The qualification is `mixed-thermal-cooperative-solver-recovery`, following the
[orphan/capacity study](orphan-execution-research-20260908.md). Unlike its
pre-computation hold, this round observes completed numerical steps before
injecting a process fault. A diagnostic checkpoint is not resumable solver state.

All mixed research jobs use the public Rust Headless SDK against an isolated
Installer-installed Linux Orchestra and Agent. Distributed dispatch is enabled;
transitional adapters are disabled. No local solver or mock fallback is requested.
The physical qualification remains `synthetic_reference_not_material_certification`.

| Gate | Result |
| --- | --- |
| Solver unit tests, including 12 control/kernel regressions | 190 pass |
| Heat Q4, thermal Q4, distorted thermal Q4 integration tests | 1 + 1 + 3 pass |
| CLI unit tests, default parallel execution and a separate 32-thread run | 151 pass in each run |
| Installed numerical-cancellation / orphan / lifecycle / shutdown live tests | 6 / 6 / 4 / 7 pass |
| Existing TaskIR solve, tamper rejection and healthy TCP execution | 1 pass, Cargo-built CLI |
| Installed baseline / permitted replay / checkpoint-required rejection | 3/3 expected outcomes |
| Fresh distributed layered and patch reference cases | 302/302 pass, 0 failures |
| Original 302-job exact readback before and after idle process restart | 302/302 at both stages |
| macOS ARM64 CLI and integration-test compile check, Rust 1.88 | Pass |

These are bounded synthetic cases, not 305 independent material models or a
general reliability percentage. The mixed workflow has seven graph nodes and
uses one 153-node, 128-Q4-cell high-contrast fixture. The 302 reference cases
are the existing 14 layered plus 288 triangle/quad patch variations.

## Defects and changes

1. Numerical kernels had no connection to execution cancellation. A GUI- and
   Orchestra-independent Rust `SolverControl` now scopes synchronous execution.
   The Agent installs it around solver RPC and TaskIR execution. Safe points
   cover common PCG, dense LU, banded Cholesky and prepared tridiagonal paths,
   including substitution. Control is sticky, restores on unwind, is isolated
   between threads, and cannot be masked by a nested scope. A new execution uses
   a fresh token. Child threads must explicitly install their own scope.
2. Shared sparse fallback and regularization could treat cancellation as a
   numerical failure and continue trying. They now check cancellation before
   retries. An observed cancellation cannot be converted to partial success.
   Prepared-factor tests prove subsequent substitution returns exactly the
   same values after interruption; the factors are not mutated by substitution.
3. The old job cancellation set was consumed by only one matching execution.
   The control registry now marks each currently registered matching execution.
   A cancellation with no matching execution preserves the existing one-shot
   pre-admission behavior. Watchdog cancellation matches request id AND execution
   generation, so a late timeout cannot poison a reused request id. Registration
   also checks whether its watchdog generation expired during setup.
4. Some parallel RPC tests reset the shared watchdog registry while unrelated
   solvers were running. The new checks exposed this as a magnetostatic RPC test
   returning no progress. Removed the unnecessary global-reset helper/calls;
   tests already identify their own requests and isolated watchdog probes retain
   local state. Both normal and 32-thread unit runs pass without serializing the
   entire suite or weakening product checks.

Read-only `kyuubiki.agent-solver-control/v1` exposes active identities, generation,
cancellation state, and a diagnostic safe point. Error details include
`solver_checkpoint`; an already-recorded watchdog failure reason is preserved.
`completed_steps` belongs to the current kernel invocation, not whole-job
progress or convergence. `resumable=false` is intentional.

The initial control-only candidate had six passing context tests and five
failing numerical tests: dense, banded, tridiagonal, ordinary sparse and prepared
sparse computations ignored cancellation. After kernel integration these pass,
along with an additional factor-reuse/substitution test. Arithmetic and existing
physical tolerance gates were not relaxed to pass the tests.

## In-progress process evidence

| Submission | Job id | Expected outcome |
| --- | --- | --- |
| `baseline` | `f466228fd8c51a07` | Generation 1 completion; exact physical match to the previous qualified baseline |
| `orchestra-replay` | `772d3ac73d7971f3` | Same job completes at generation/attempt 2, one terminal commit |
| `orchestra-blocked` | `aeb7734723a01855` | Generation/attempt 1 remains recovery-blocked, without successful structure output |

Both fault scenarios use the explicit exact-job, structural-method hold with
`KYUUBIKI_AGENT_FAULT_INJECTION_SOLVER_STAGE=dense_factor`. Before SIGKILL, the
snapshot identifies the matching request with three completed decomposition
steps, upstream heat/bridge progress, and the Agent process identity. A merely
active task or `await-held` label would not be sufficient. Each held observation
is bounded to 120 seconds; this is fault injection, not a general pause facility.

For permitted replay, old request `a7331b304adc6f70` records
`solver cancelled at dense_factor after 3 steps`. Its marker was not removed to
make cancellation occur. New request `010a3dc19a447f80` reaches the same numerical
stage under the next workflow generation with one active execution. The Agent
identity stays `agent-instance-7-1788860677408-1`; it was not restarted between
the two requests. After marker release the original job completes.

For checkpoint-required rejection, old request `d654907d4d0fdff9` records the
same in-solver cancellation. While its marker still exists, active count is zero,
total-started remains 8, and total-completed remains 6. There is no replacement
execution inferred from a terminal label alone. Releasing the marker later does
not change the blocked job/result. Safe rejection is not a numerical pass.

Both owned Orchestra SIGKILLs exit 137 without OOM. Positive research outcomes
validate temperature, reference mapping, expansion and flux, and compare full
physical nodal/element arrays and the bridged model against the uninterrupted
reference. Maximum errors remain 6.310e-12 K, 6.807e-17 m and 7.687e-9 W/m2,
below unchanged gates 1e-7 K, 1e-9 m and 7.921e-5 W/m2. The phased verifier also
checks terminal stability after 600 ms; this is not an unbounded stability proof.

Separate installed TCP tests observe at least three PCG steps on a 1,681-node
heat mesh, dense pivots on an 81-node mesh, and tridiagonal elimination on a
601-node heat chain. They exercise explicit cancel, transport loss and two
active solves of the same job, require failure without partial fields, retain
capacity until actual return, and check the next temperature field against its
analytical solution. The two-current-execution case uses native capacity 2.

The fresh 302-case matrix completes with zero failures. All original result
values pass exact readback before and after both idle processes stop with exit
0 and restart. This is read-only verification, not resubmission of equivalent
jobs. It extends the fault study with persistence checks, not power-loss durability.
The three phased-study result files also remain byte-identical after the idle
restart. The fresh Agent reports zero started executions after all read-only
checks, so those checks did not silently dispatch new calculations.

## Provenance and retention

Base revision is `b9bc00cc250b13246f219f9edea05b9901bc6bd2` plus the retained
23-file source/test overlay. Local and remote source digests match. Native
Installer seals and installs label **3.1.3**, activation generation 1, into an
isolated managed runtime root. Rust/SDK manifests and the reused Orchestra Mix
release still report their actual **3.0.0** metadata; no repository version bump,
commit, or system-installation promotion is part of this task.

One remote physical Linux host runs non-root uid 1001 containers with read-only
root/payload, separate writable state, init, no capabilities, no-new-privileges,
four CPUs each and 4 GiB / 2 GiB memory limits. HTTP/TCP listen only on loopback
6440/6441; Orchestra uses SQLite. Agent build Rust is 1.95.0, the existing
Orchestra release uses Elixir 1.19.5 / OTP 28.5.0.2. Existing production services
are not targets of faults or replacement. This is not an HA or SSH rollout test.

Large artifacts remain under managed server state
`research-runs/cooperative-research-20260908/`. `evidence/` retains original
submissions, reports, numerical-stage snapshots, installed live-test observations,
and the full matrix/readback. `evidence/verification/` retains red/green logs,
source overlay, lifecycle records and hashes. No server configuration, credentials,
runtime payload or full research result set is added to Git. Previous studies
and their failed evidence are not overwritten.

Two operator invocation errors were corrected without changing product physics:
the first container used the wrong SQLite environment-variable name and failed
on its read-only fallback path; the first baseline verifier pointed to a
nonexistent reference directory. An immediate post-restart verifier also ran
before HTTP readiness. These failed attempts remain separate from the passing
reruns. They must not be counted as passed research runs or hidden solver fixes.

| Retained artifact | SHA-256 |
| --- | --- |
| Installed Agent | `367b6c5045418213f9b41b8ab1e241ae03b8c5dbaaef7402ee36a4762401f57f` |
| Installed payload manifest | `21674f9195b7a579842fd99144496397c9d39fc24a138e416b117acbfa67b87f` |
| Source/test overlay | `d49ca0c42a2bf74c40a80369467ef2e757e6ac54d1dd713167c49aa9c53ad15b` |
| Fresh 302-case report | `e28661dd609c9e37de4b17272523761ea5302f310f250c04d36f6fac4aa8b8c6` |

Digests identify retained bytes, not signatures or material certificates.

Documentation inventory, the 25-page HTML book, Rust formatting, tensor self-test
and structure, and project organization checks pass. Source files remain within
800 lines and documents within 2000. The global tensor still reports
`daji status=blocked`; scoped evidence does not override the overall release gate.
Both owned candidate containers were stopped and removed after evidence capture.
Original service PIDs 3461/3462/3463 retain their August 31 start times; unrelated
containers remain running. Managed installation, database and research evidence
remain on the server for inspection.

## Reproduction and remaining boundaries

Use the [HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [numerical control contract](../docs/agent-orchestrator-boundary.md#cooperative-numerical-cancellation).
In the Rust workspace, run `cargo test -p kyuubiki-solver --lib solver_control`
and `cargo test -p kyuubiki-cli --test agent_solver_cancel_live`.
`KYUUBIKI_TEST_AGENT_BINARY` selects an Installer-installed Agent;
`KYUUBIKI_TEST_AGENT_EVIDENCE_DIR` retains owned process observations. Faults
must target isolated, explicitly owned processes, never an active user's study.

Still unqualified: arbitrary solver preemption, cancellation inside assembly,
compression or preconditioner construction, a fixed latency bound within a
large matrix/vector operation, every nonlinear outer loop, external libraries,
child-thread propagation, numerical-state checkpoints, exactly-once side effects,
network blackholes, multi-host partitions, power loss, database corruption,
material calibration, independent external-solver agreement, and installed
Windows/macOS live cancellation. The analytical TaskIR `solve.bar_1d` shortcut
has entry/exit control but no new internal numerical safe points in this round.
TCP liveness still observes job-bound heartbeat failures, not every connection.

The new tensor evidence is scoped to these kernels, tests and installed research
outcomes. It is not a blanket solver maturity, performance or release-readiness
upgrade. Uncooperative work retains its capacity until it actually returns.
