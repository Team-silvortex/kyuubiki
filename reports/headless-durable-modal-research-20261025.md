# Official Rust SDK Durable Modal Research

Date: 2026-10-25. Platform: local macOS ARM64. Package baseline: daji 3.7.1.
No version change, installed application update, commit or remote deployment.

This is bounded source-built SDK/Agent verification, not installed or general modal qualification.

This report records the first run. A later same-day
[lost-admission follow-up](headless-durable-admission-recovery-20261025.md) adds
typed input intents, replaces the missing-acknowledgement export restriction
below with validated reconciliation, and records its own test counts.

## Actual Execution Path

The standalone official package under `sdks/rust` calls the native Agent over
loopback TCP through `SolverRpcClient`. It does not import Solver or Engine,
run a mock instead of the Agent, or dispatch through a new research-only CLI.
The existing Agent durable execution wrapper dispatches the ordinary production
`solve_modal_frame_2d` route. No engine/solver algorithm or tolerance changes.

New reusable SDK calls submit once, inspect by original store/execution ID, and
fetch against a retained `DurableExecutionHandle`. The handle binds store,
execution, request digest, original request/process and method. It validates
receipt status and inner response identity before allowing `completed_result`.
A successful outer RPC alone is not computation success. The handle is an
observed consistency fence, not authentication or a numerical certificate.

The example saves input and a checksum-bearing submission intent before its
first mutation, then saves admission separately. A new process can inspect the
same execution and export into a new directory. It does not replace an existing
export. Missing admission acknowledgement leaves read-only inspection evidence
but does not automatically qualify/export a successful result. Transport loss
does not create a new execution ID or authorize automatic resubmission.

## Numerical Reference

- Two horizontal beam segments of length 1 and 2 use different Young's moduli,
  areas, densities and bending inertias. Total mass is 21 in consistent units.
- Axial displacements are restrained and the root is clamped. Four free bending
  DOFs request four modes, not a partial single-mode sample.
- Independent example-side Euler-Bernoulli assembly reconstructs stiffness and
  original diagonal translational/rotary masses from the input. No production
  numerical helper supplies the expected operator, residual or roots.
- A bounded four-DOF LDL inertia/bisection reference checks ordered roots to
  relative `1e-8`. Each published physical shape must retain unit norm within
  `1e-10`, zero restrained DOFs, mass-weighted residual at most `1e-8` and mutual
  mass orthogonality within `1e-8`. Frequency/period identities use `1e-12`.
- Reversed node/member numbering rebuilds the actual request and independent
  reference; it is not simultaneous permutation of a cached assembled matrix.
- Exported JSON is read from disk before verification. Corrupted roots, zero
  shapes, duplicate modes, absent modes, frequency/period and mass are rejected.

The checker deliberately accepts only these two explicit synthetic input
orderings. It is not a general-purpose SDK eigensolver or material certificate.

## Recovery And Failure Cases

1. After terminal receipt commit, forcibly terminate the actual Agent process
   and restart it using the same disk store. The recovered receipt and full
   physical modes equal the originals; independent readback checks pass.
2. Hold a real solve at `modal_shape_norm`, observe that live numerical stage,
   then forcibly terminate the process before completion. Restart reports
   `outcome_unknown`, no response and no automatic replay. Explicit duplicate
   submission observes the same unknown outcome instead of recomputing.
3. A separately authorized fresh ID completes a healthy solve without changing
   the original unknown execution. This is a new attempt, not resumed iteration.
4. Replace the temporary store at the same endpoint. The old handle is rejected
   and the replacement store remains empty. Restoring the original store allows
   the original result to be read again. Changed input under the old ID is rejected.
5. The compiled SDK example is run in separate submit/inspect processes around
   Agent termination/restart. It writes a verified result export, refuses export
   overwrite and rejects a changed retained input before producing new evidence.

The harness uses `Child::kill` (SIGKILL on this macOS run), not graceful shutdown.
It only starts/kills its own loopback test processes and removes successful
temporary test directories. No installed service, user data or host was stopped.
Process-kill evidence is not full host-power-loss or filesystem crash evidence.

## Reproduction And Results

```text
make test-sdk-modal-recovery
cargo test --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets
cargo clippy --manifest-path sdks/rust/Cargo.toml --locked --offline --all-targets -- -D warnings
```

The focused target builds the Agent and SDK example, runs 6 SDK receipt/ownership
tests and explicitly executes 4 real-Agent integration tests. All passed.
The complete official SDK all-target suite passed 123 tests; its 4 opt-in live
tests are ignored by that default command and were executed separately above.
Do not add the 6 focused SDK tests again to the full-suite count.

The `modal-sdk-durable-research` validation profile also ran its command with
`executed=true`, producing `tmp/headless-durable-modal-validation-20261025.json`.
SDK all-target Clippy with warnings denied, changed-file formatting, documentation
book/inventory, project organization and diff whitespace checks passed. Source
files remain below 800 lines and documents below 2,000. The tensor still reports
0 structural, 2 maturity, 23 evidence-grade and 18 P0 gaps; status stays blocked.

Loopback process tests require local socket/process permissions. The initial
restricted invocation failed with `Operation not permitted`; the authorized
rerun executed all live cases. No test was weakened or silently skipped to pass.

## Remaining Acceptance

The tensor claim is `verified` only. Existing installed Python/Elixir research,
heterogeneous long-bending modal physical publication, six-mode recovery,
independent broad reassembly and production cost obligations remain open.
This run does not exercise TaskIR package acquisition, Orchestra or Mesh routing,
authenticated multi-host transport, source-detached installed artifacts,
Windows/Linux execution, arbitrary inputs, a full power cut or kcore export.
The example exports checked JSON research artifacts, not a new exchange format.
