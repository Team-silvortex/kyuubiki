# Native Agent LAN Control Gate

Date: 2026-10-28. Source version: daji 3.7.1.

## Boundary And Change

The source-IP allowlist previously admitted all native RPC operations except
drain/resume. It did not authenticate a caller, and public execution, store,
cluster or controller labels were not credentials. The user explicitly approved
a stricter default with a separately acknowledged trusted-LAN compatibility mode.

Non-loopback peers must still pass `KYUUBIKI_AGENT_ALLOWED_PEER_IPS` before a
connection receives capacity or a handler. Passing that check now grants only
`ping`, `describe_agent` and `describe_agent_lifecycle` by default. All remaining
operations fail before dispatch, store access or execution admission. This covers
solver and TaskIR execution, durable run/submit, result retrieval, execution index,
cancellation and package-job release. The same check runs on recovery-reserve
connections and every request on a persistent connection.

`KYUUBIKI_AGENT_TRUSTED_LAN_CONTROL=true` restores these operations for already
allowed source IPs on an isolated trusted LAN. Unset or exactly `false` keeps
discovery only; malformed or non-Unicode settings fail startup without echoing
their values. The switch never expands the IP allowlist. Drain/resume remain
loopback-only. Existing local callers retain their previous behavior.

Denied operations return `agent_lan_control_requires_explicit_trust` with
`dispatched=false`, `automatic_retry=false` and `automatic_replay_authorized=false`.
This is evidence about the denied request, not a terminal result or absence proof
for the execution it names. Original execution IDs and receipts must be retained.
Configuration and saturating denial counters are visible under
`describe_agent.request_ingress.network_access.lan_control`.

The shared protocol policy has no environment, filesystem or network effects.
Agent startup, Installer environment checks and managed-runtime preflight use it.
Invalid managed restart configuration is rejected before stopping existing
services. Runtime status reports configured intent, not an observed running
policy. Applying or revoking the switch requires an explicit process restart;
old sockets close and existing durable records are not removed or replayed.
No engine or operator numerical code changed.

## Verification

| Suite | Passed | Not run |
| --- | ---: | --- |
| Agent binary unit suite | 331 | 0 |
| Agent ingress / local Mesh / network policy / recovery integration targets | 25 | 1 interface case run separately below |
| Explicit local non-loopback interface case | 1 | 0 |
| Protocol / Installer / desktop-runtime library suites | 394 | 1 real installed-control-stack opt-in test |
| Managed runtime lifecycle / network preflight integration targets | 16 | 0 |

Total: 767 passing test executions, zero failures in the final runs. The normally
ignored local-interface case was explicitly run; the real installed-control-stack
test remains ignored to avoid starting existing host services.

- The normally ignored interface test was explicitly run against a local,
  non-loopback IPv4 interface after first verifying that address could be bound
  locally. It did not probe another machine. It passed default socket denial,
  allowlisted discovery, denial of every advertised non-discovery/non-lifecycle
  method, recovery-reserve denial, trusted solve/admission/readback, lifecycle
  rejection, restart revocation, old-socket closure and unchanged local receipts.
- The first interface-test invocation was denied by execution permissions before
  binding. The approved local-interface run then passed; the denied attempt is
  not treated as product behavior or a successful test.
- Test-owned processes and temporary stores are cleaned by the existing fixture.
  No installed application, real host service configuration or retained user
  study was changed.

Workspace `cargo fmt --all --check`, Agent binary `cargo clippy -- -D warnings`,
`git diff --check` and the coverage-tensor checker passed. The tensor still has
23 evidence-grade gaps and 18 P0 coordinate gaps: this verified containment claim
does not close the authenticated LAN qualification requirement.

Commands use the existing Rust target directory and offline locked dependencies:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-cli --test agent_network_access_live --test agent_lan_mesh_live --test agent_ingress_live --test agent_recovery_ingress_live --locked --offline
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-protocol -p kyuubiki-installer -p kyuubiki-desktop-runtime --lib --locked --offline
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-desktop-runtime --test runtime_network_preflight --test runtime_lifecycle --locked --offline
```

For the explicitly selected interface case, set `KYUUBIKI_TEST_AGENT_LAN_IP` to
the test host's own IPv4 address, then run the `agent_network_access_live` target
with filter `real_non_loopback_socket_is_denied_then_explicitly_admitted_and_revoked`
and test flags `--ignored --exact`. No machine-specific address is retained here.

## Remaining Limits

Explicit LAN trust is not authentication. It grants every allowed source control
without differentiating users/controllers on that host or behind NAT. There is no
new TLS, signature, credential-revocation or replay-protection protocol. Certificate
metadata and task/store IDs still cannot authenticate the caller. Discovery still
exposes Agent metadata; it is not a confidential-information boundary. Local
processes and local proxies remain trusted and must enforce their own permissions.

This is local source-build containment evidence, not installed-app, multi-host
authenticated Mesh, host-power-loss or operational load qualification. In the
coverage tensor, retain it as a separate verified claim; keep
`native-agent-lan-ingress-authorization` open rather than using this compatibility
switch as authentication evidence.
