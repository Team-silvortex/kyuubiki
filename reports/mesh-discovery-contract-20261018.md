# Mesh Discovery Contract And Recovery Isolation

Date: 2026-10-18. Working tree based on `3a5ef43f` (`daji 3.6.5` commit
subject); source package metadata remains 3.6.0. Existing network-access,
deployment-preflight, architecture and submission-identity changes are preserved.
No version bump, installation, remote deployment, commit or push is included.

## Finding

Native peer discovery checked the cluster and offline authority before recording
success, but did not check the advertised role, protocol name, transport contract
or presence of the discovery method. A correctly framed reply could claim an
incompatible protocol and still mark its endpoint healthy and introduce hints.

Two failing-before-fix regressions establish the gap:

- The compatibility unit regression accepted eight inconsistent descriptions:
  role, protocol name, RPC version, transport kind, encoding, wrong framing,
  absent framing and missing `describe_agent`. The wire reader already rejected
  an unsupported RPC version; that case is defense at the compatibility boundary,
  not a newly demonstrated wire-version bypass.
- A real native Agent connected to controlled loopback descriptor peers. When
  its previously healthy seed advertised newline framing and a new peer hint,
  the old code retained 100% health and learned the hint. The test failed while
  waiting for the required degraded state. This was a deliberate inconsistent
  reply fixture, not an observed compromised host.

## Repair

The Agent control-layer compatibility gate now requires `solver_agent`, the
current solver-RPC protocol/version, TCP/JSON with `length_prefixed_u32` framing,
and advertised `describe_agent` support before recording success or merging
hints. A mismatch takes the existing bounded probe-failure path. No solver,
operator package, execution engine, persistent ledger or replay logic is changed.

Busy, draining and quiescent nodes remain discoverable. Engine/program branding
and identical solver sets are deliberately not membership requirements. Discovery
does not claim execution readiness; dispatch still has its separate method,
capacity, lifecycle and ownership checks. The read-only discovery policy exposes
`descriptor_contract` and `discovery_is_execution_readiness=false`.

The process regression now verifies the complete sequence:

1. Discover a compatible seed and commit a real native bar-solver result.
2. Publish an incompatible seed descriptor with an uncontacted peer hint.
3. Observe degradation without adopting or probing that hint; retain the receipt.
4. Force-kill and restart the test Agent using the same store and configured seed.
5. Observe an unreachable seed and continued hint isolation after restart.
6. Repair the descriptor and rediscover both seed and independently probed hint.
7. Read the exact original receipt and repeat the fenced submission without
   replaying the completed work or replacing its original execution identity.

## Verification

- Native Agent binary unit suite: 281 passed, including two new contract tests.
- Four process suites: 18 passed executions, consisting of ten scenario tests
  and two shared fixture checks repeated in each suite. The new scenario uses one
  real Agent plus two descriptor fixtures; the existing suites separately retain
  real multi-Agent discovery, owner-replacement and lost-cancellation recovery.
- Desktop runtime direct/Mesh selection: 50 passed, including the preceding
  submission-identity fix and existing transport/recovery guards.
- Strict CLI Clippy (`--all-targets`, `-D warnings`), formatting, HTML book and
  inventory checks and the project organization audit passed. Source/document
  limits remain 800/2000 lines with no tracked organization debt.
- Tensor structure/command validation passed: zero structural, two maturity,
  22 evidence-grade and 17 P0 gaps; release readiness remains blocked.
- All sockets are loopback, listener waits are stoppable, I/O waits are bounded,
  and fixture processes/scratch stores are cleaned up. No user service is restarted.

Reproduce from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-cli --locked --offline --quiet
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test agent_mesh_descriptor_live --test agent_lan_mesh_live --test agent_mesh_owner_live --test agent_mesh_cancel_recovery_live --locked --offline -- --test-threads=2
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-desktop-runtime --lib direct_mesh --locked --offline --quiet
cargo clippy --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --all-targets --locked --offline -- -D warnings
```

Compatibility checks are not authentication. A dishonest peer can still advertise
the expected fields; source-IP containment is not cryptographic membership.
The run does not qualify installed GUI recovery, multi-host authentication/load,
host power loss or distributed workflow continuation. Tensor evidence is appended
to the existing local-isolation claim without promoting its grade or release gate.
