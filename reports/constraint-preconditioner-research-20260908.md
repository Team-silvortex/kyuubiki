# Constraint and Preconditioner Cancellation: 2026-09-08

## Verdict and scope

**Installed constraint cancellation: PASS, 3/3 expected research outcomes.**
Qualification: `mixed-thermal-constraint-preconditioner-recovery`. This extends
the [preparation study](preparation-cancellation-research-20260908.md) into shared
constraint elimination and preconditioner application. It is not a new material
model, performance benchmark, or universal solver-cancellation guarantee.

Research uses the public Rust Headless SDK against an isolated Linux runtime
sealed and installed by the native Installer. Orchestra dispatch is distributed,
transitional worker adapters are disabled, and no mock/local solver fallback is
requested. Qualification remains `synthetic_reference_not_material_certification`.

| Gate | Executed result |
| --- | --- |
| Initial regression fixture against the old numerical paths | 0 pass / 8 expected failures for ignored cancellation |
| Complete solver unit and integration test run | 685 pass across 153 test targets, 0 fail, 0 ignored |
| Solver unit tests within that total | 215 pass, including 12 added in this round |
| New 2D/3D cohesive cancellation regressions within that total | 2 pass; ordinary healthy results remain unchanged |
| CLI unit tests in separate 16-thread and 32-thread runs | 151 pass in each run |
| Cargo-built and Installer-installed cancellation / orphan / lifecycle / shutdown suites | 20 / 7 / 5 / 8 pass in each run |
| Cargo-built TaskIR solve, tamper rejection and healthy subsequent TCP execution | 1 pass |
| Installed baseline / permitted replay / checkpoint-required rejection | 3/3 expected outcomes |
| Fresh distributed thermal and thermal-stress reference matrix | 302/302 pass, 0 failures |
| Exact original-case readback before and after idle process restart | 302/302 at both stages |
| macOS ARM64 CLI and solver test compilation, Rust 1.88 | Pass |

The four live suites include the same startup-ownership regression once per
executable; 40 passes are not 40 distinct physical models. Installed testing
selects the installed Agent binary; CLI unit tests still run in a Cargo test
executable. The matrix contains 14 layered cases and 288 triangle/quad patch
variations, not 302 independent material certifications. Counts are not a global
reliability percentage, GUI acceptance, or a scaling result.

## Defects and changes

1. Shared homogeneous/prescribed sparse constraint elimination could not return
   cancellation. Constraint indexing, free-DOF mapping and reduced-row assembly
   are now fallible, with errors propagated through every shared caller. A
   partial reduced matrix is never returned as usable state. The free map is
   built in original DOF order in one pass; row arithmetic is unchanged.
2. Jacobi, SGS and IC(0) application could continue after cancellation. Their
   applications now return errors, including both forward and reverse sweeps.
   PCG propagates errors from its initial and later applications. Partial output
   and workspace vectors are scratch, not valid results. Factors remain immutable;
   subsequent complete application overwrites the same scratch correctly.
3. Cohesive Newton and co-rotational failure paths could conflate a cancelled
   tangent solve with ordinary nonconvergence. They now propagate cancellation
   instead of exporting degraded output or selecting another solve. The shared
   symmetric tangent fallback checks cancellation before dense expansion.

New diagnostics are `constraint_index`, `constraint_map`, `constraint_reduce`,
`preconditioner_jacobi`, `sgs_forward`, `sgs_backward`, `ic0_forward` and
`ic0_backward`. Polling occurs at entry, each 64 completed entries/rows, and the
final short batch. Constraint stages count constraint entries, original DOFs
and free rows respectively. Reverse sweeps count completed rows, not descending
row indices. Counts restart per invocation and are not whole-job progress.

Eight new tests first compiled against the old numerical implementation and
all failed because the selected sweep ignored cancellation. Added green tests
also cover independent dense-reference reduction/RHS values, all-constrained
systems, 49-row final batches, reused IC(0)/SGS scratch and the second PCG
preconditioner application. Public Q4 heat profiles exercise SGS and IC(0)
application on 1,681-node meshes and then recover the analytical temperature.
Both cohesive regressions assert the raw solver returns an error inside the
observer scope, rather than accepting a nonconverged result that only the outer
scope rejects. Subsequent complete 2D/3D results equal uninterrupted baselines.

Installed heat RPC cases add constraint indexing, free mapping, reduction,
Jacobi application, and transport loss during reduction. They observe exactly
64 completed steps before cancellation and exercise same-job reuse. SGS/IC(0)
application is qualified through native solver tests, not installed RPC fault
injection or a new public RPC preconditioner option.

Constraint reduction moved to `linear_sparse_reduction.rs`; `linear_algebra.rs`
decreased from 672 to 607 lines. Source files remain below the 800-line limit.

## Installed research evidence

| Submission | Job id | Expected outcome |
| --- | --- | --- |
| `baseline` | `b897ced9417eafe7` | Generation 1 completes and matches the previous qualified physical baseline |
| `orchestra-replay` | `08f6d8cc1485d62f` | Original job completes at generation/attempt 2 with one terminal commit |
| `orchestra-blocked` | `d3e2f9ffd36872a6` | Generation/attempt 1 stays recovery-blocked without successful structure output |

The seven-node mixed graph uses the existing 153-node, 128-Q4-cell high-contrast
fixture. Both owned Orchestra SIGKILLs occur during
`solve_thermal_plane_quad_2d` at `constraint_reduce`, after exactly 64 free rows.
Both exit 137 without OOM. The exact-job marker is not released to provoke
cancellation, and the Agent is not restarted between original and replayed work.

For permitted replay, original request `0c4b452affb9bb1f` reports
`solver cancelled at constraint_reduce after 64 steps`. Replacement request
`7fb5ed614ac3256f` reaches the same stage with one active execution. Agent instance
`agent-instance-7-1788865658253-1` is unchanged. Its execution generations are
4 and 6; these are not the workflow attempt numbers. Only after observing the
replacement request is the marker released to let the original job complete.

For checkpoint-required work, request `8bb5ca8bcea55b9b` reports the same
cancellation. With its marker still present, active count is zero, total-started
stays 8 and total-completed stays 6. No replacement calculation is hidden behind
the blocked label. Marker removal does not change the blocked result. Safe
rejection is the expected outcome, not numerical acceptance or saved-state resume.

Positive outcomes compare full physical arrays and bridged models with the
uninterrupted reference. Maximum errors remain 6.310e-12 K, 6.806e-17 m and
7.687e-9 W/m2, below unchanged gates of 1e-7 K, 1e-9 m and 7.921e-5 W/m2.
The verifier checks 600 ms of terminal stability, not unbounded exactly-once behavior.

The fresh 302-case matrix and both exact readbacks have zero failures. Idle
Orchestra and Agent stop with exit 0, restart, and return the same original
results. All three phased result files are byte-identical after restart. The
fresh Agent reports zero started executions after readback, so validation did
not silently rerun computations. This is persistence readback, not power-loss
durability, material certification or numerical checkpoint resumption.

## Provenance and boundaries

Base revision is `3aab326bbaa9b162f111bd8fe9ceb4f5150f15b1` plus the retained
51-file Rust source/test overlay, including the preceding uncommitted preparation
changes. All 51 local/remote digests match. Candidate label **3.1.4** is sealed and
installed into a new isolated managed root. Cargo/reused Orchestra metadata remain
their actual **3.0.0** values. No version bump, commit, system installation
promotion, production process replacement or GUI repackaging occurs here.

The Agent is built with Rust 1.95.0. The reused release runs in Elixir 1.19.5 /
OTP 28.5.0.2. Non-root uid 1001 containers have read-only root/payload, isolated
writable state, init, no capabilities, no-new-privileges, four CPUs each and
4 GiB / 2 GiB limits. HTTP/TCP are loopback-only 6460/6461 with SQLite storage.
This is one physical Linux host, not multi-host/HA qualification.

The prior source-tree Mix-server failure has a confirmed environment boundary:
host Mix/Elixir is 1.14.0, while `apps/web/mix.exs` requires Elixir `~> 1.19`.
A no-start diagnostic rejects that mismatch before the server starts. The host's
global toolchain is not changed, and the source-tree `operator_task_live` path
remains unqualified; passing installed-service tests do not turn it into a pass.

Artifacts remain under managed server state at
`research-runs/sweep-research-20260908/`: source overlays, initial red failures,
complete solver/CLI logs, installed live observations, installation records,
process exits, submissions, matrix and readback results. Credentials, host
configuration, payloads and full result sets are not added to Git. Earlier
studies are preserved. Both owned containers are stopped and removed; original
service PIDs 3461/3462/3463 retain their August 31 start times, and unrelated
containers remain running. The database, installation and evidence stay remote.

| Retained artifact | SHA-256 |
| --- | --- |
| Installed Agent | `f03be3dff12de8f60e5ca95ad576ad1af7bc2a8c9c55984ab5d0745f5d9a343d` |
| Installed payload manifest | `23605be47c0519a89fcbb711e57f57b847ecefa25dafc46e6aeac13c963c1460` |
| Combined green source/test overlay | `eecfdb3a0a779d9a2318c118a14e6ba10a0621429fd5db796a53de8e9233a936` |
| Fresh 302-case report | `03f2c8a2da242075c41c38c3767f2babbb8e9a58d7667f0935ea189404f34a10` |

Digests identify bytes, not signatures. Reproduce with the
[HTML research tutorial](../docs/research-layered-thermal.html#interrupted-research)
and [numerical control contract](../docs/agent-orchestrator-boundary.md#cooperative-numerical-cancellation).
Run `cargo test -p kyuubiki-solver --tests` and the four named CLI live suites in
the Rust workspace. `KYUUBIKI_TEST_AGENT_BINARY` selects an installed Agent;
`KYUUBIKI_TEST_AGENT_EVIDENCE_DIR` retains test-owned evidence. Faults must target
explicitly owned isolated processes, never another user's active work.

Remaining gaps: input validation, allocation, operator-specific constraints,
postprocessing, other assembly/preconditioner implementations, matrix/vector
interiors, every nonlinear outer loop, analytical shortcuts, external workers
and child threads. There is no fixed cancellation latency or measured overhead
bound. Network blackholes, multi-host partitions, power loss, database corruption,
external-solver cross-validation, material calibration and installed Windows/macOS
live cancellation remain separate work. Uncooperative work owns capacity until
it returns. This scoped tensor claim does not override global release maturity.

The 25-page HTML book and documentation inventory, Rust workspace formatting,
tensor self-test/structure, and project-organization audit/self-test pass.
Source/document limits remain 800/2000 with zero tracked line-limit debt.
The global tensor still reports `daji status=blocked`; this result does not
turn an overall release gate green.
