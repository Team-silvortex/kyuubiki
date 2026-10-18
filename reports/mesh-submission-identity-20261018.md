# Direct/Mesh Submission Identity Binding

Date: 2026-10-18. Working tree based on `3a5ef43f` (`daji 3.6.5` commit
subject); source package metadata remains 3.6.0. Prior uncommitted network-access,
deployment-preflight and architecture work is preserved. No version bump,
installation, remote deployment, commit or push is part of this change.

## Reproduced Gap

The native direct/Mesh gateway validated durable execution IDs and report shapes,
but did not compare submitted method/input identity with the returned receipt.
An admission rejection checked digest syntax but not its value. Consequently a
reply could have the right execution ID and still describe a different task,
or return a lookup-only `not_found` decision as a successful submission response.

Before the fix, four new TCP/gateway regression tests failed with HTTP 200 where
HTTP 502 was required: foreign method/input, adjacent binary64 inputs, lookup
absence in place of admission, and a valid-hex foreign rejection digest. Two
positive controls passed: matching historical receipts and independent lookup.
These are controlled reply substitutions, not observed corruption from a live
Agent or evidence that a production calculation was wrong.

## Repair

- Compute the expected submission identity before opening the connection; bind
  every successful submit reply, including non-admission, to execution ID,
  method and payload digest. Reject lookup absence even if identity fields are
  present. Existing shape, store-owner and RPC-envelope checks remain in force.
- Reuse the protocol's typed binary64 digest through a borrowed payload API.
  No duplicate hashing algorithm or large input clone is introduced for this
  check. Adjacent representable floats must remain distinct.
- On a mismatch, keep the original endpoint/ID and return an ambiguous failure
  without exposing an accepted execution, automatic retry, cross-Agent replay,
  or permission to clear the saved handle.
- Preserve stored original process/request IDs for a duplicate submission of
  the same payload. A new transport request ID is not a new execution owner.
  Lookup remains independent of the user's currently edited input.
- Publish the identity-binding fields and encoding in the existing read-only
  transport policy. This is content consistency, not authentication.

The stricter pre-connect check exposed two older transport tests that supplied
empty task parameters and then waited forever in their mock listener's accept.
That incomplete test run was terminated. The shared fixture now supplies valid
submission parameters and bounds accept to five seconds. The final full suite
passes; an interrupted run is not counted as a pass.

## Verification

On macOS, using the existing workspace target and locked offline dependencies:

- Desktop runtime library: 118 passed, one intentionally ignored real local-stack
  test. This includes seven new submission tests and the existing transport,
  dispatch, store-fencing, journal and recovery-route regressions.
- Protocol library: 142 passed. Borrowed payload hashing agrees with the task's
  validated digest and rejects unsupported method/non-object payloads.
- Native Agent binary unit suite: 279 passed, including discovery, admission,
  ownership and persistent-execution validation.
- Native Mesh process suites: 15 passed executions, comprising nine scenario
  tests and two shared fixture checks repeated in each of three suites. They
  cover three-peer discovery, loss/return, learned-candidate retirement,
  endpoint replacement, foreign cancellation and forced-restart ambiguity.
- Strict Clippy for protocol, desktop runtime and CLI (`--all-targets`,
  `-D warnings`), HTML book/inventory, project organization and whitespace checks
  passed. Source files remain within 800 lines and documents within 2000.
- Tensor validation passed with zero structural, two maturity, 22 evidence-grade
  and 17 P0 gaps. Release readiness remains blocked; these counts are unchanged.

Reproduce from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-desktop-runtime -p kyuubiki-protocol --lib --locked --offline --quiet
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-cli --locked --offline --quiet
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test agent_lan_mesh_live --test agent_mesh_owner_live --test agent_mesh_cancel_recovery_live --locked --offline -- --test-threads=2
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-desktop-runtime -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

The new gateway cases use bounded loopback TCP reply fixtures. The separate Mesh
suites run real native Agent processes and real bar solves; they are not an
installed GUI-to-Agent test. No user service or remote host is restarted. Native
authentication, sustained multi-host Mesh load, installed recovery and host power
loss remain unqualified. The existing local-isolation tensor claim gains scoped
evidence only; its grade and the release gate are not promoted.
