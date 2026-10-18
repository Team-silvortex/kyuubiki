# Native Agent Source-IP Containment

Date: 2026-10-16. Working tree based on `3a5ef43f` (`daji 3.6.5` commit subject).
Source package metadata remains 3.6.0. No product version bump, installation,
remote deployment, commit or push is part of this change.

## Finding And Change

The raw Agent TCP listener previously admitted any reachable source into its
request/recovery pools. The lifecycle RPC guard only restricted drain/resume;
certificate and cluster metadata did not authenticate the native listener.
Operator package integrity and durable ownership identities did not close this
transport exposure either.

All native control modes now share a loopback-only default. Remote source IPs
must be configured explicitly through `KYUUBIKI_AGENT_ALLOWED_PEER_IPS` in the
Agent process environment. The bound interface, advertised address, Orchestra
URL, cluster ID and discovered peers cannot grant access implicitly.

- Parse up to 64 comma-separated IP literals within 4096 bytes. Unset selects
  loopback-only; empty or malformed values stop startup before durable store,
  package initialization and listener creation. Configuration errors do not echo
  the untrusted value.
- Reject wildcards, subnets, DNS names, ports, unspecified, broadcast and multicast
  addresses. Canonicalize IPv4-mapped IPv6 and deduplicate equivalent entries.
- Read the kernel socket peer address before granting ordinary or recovery
  connection capacity, spawning a handler or reading a request frame. Reject an
  unavailable peer address and unlisted non-loopback addresses without a reply.
- Keep loopback available for local control and inspection. Exact remote IP
  permission does not bypass the existing loopback-only drain/resume guard.
- Expose effective policy and a saturating rejection counter through
  `request_ingress.network_access` in descriptions and registration/heartbeats.
  There is no per-rejection log amplification or unbounded address history.
- Apply changes on restart. Existing exact durable receipts remain inspectable;
  changing a network policy never authorizes replay or owner migration.

**Deployment change:** a wildcard bind no longer permits LAN clients by itself.
Before upgrading remote deployments, set the actual controller/gateway and Mesh
source IPs in the host-owned service environment. Each Agent has its own list.
The existing systemd environment-file hook is documented; no server-specific
configuration or addresses have been added to source control. Do not kill active
work merely to apply this setting; drain/reconcile before a planned restart.

## Verification Scope

The policy parser/counter tests cover defaults, exact match, IPv4-mapped IPv6,
invalid and overbroad values, bounded configuration, unavailable peer identity,
concurrent denial and saturation. An ingress test verifies that rejected peers
cannot consume ordinary or recovery connection capacity, including under load.

Native process regressions cover actual bar execution, durable receipt recovery
across policy changes, invalid startup configuration without store loss, Mesh
discovery/peer failure, and recovery-reserve isolation. Test fixtures explicitly
override inherited host/port policy so a developer environment cannot accidentally
redirect their supposedly local child to a different listener.

A separate opt-in wire test binds only a caller-specified local IPv4 interface
and a temporary wildcard Agent listener. It makes real non-loopback connections
to the same machine, sends no frame on denied connections, enables one exact
source, solves a bar, checks that remote lifecycle control remains rejected,
then removes the permission on restart and verifies local work still succeeds.
It is not a two-host, IPv6 wire, installed-app or authenticated-LAN test.
Test-owned processes and scratch directories are removed by the fixture.

The first non-loopback run was blocked by sandbox socket permissions. An initial
authorized run then reached its two-second close-observation timeout. A diagnostic
rerun passed; transient socket/scheduling behavior was not established as a root
cause. The final code removes temporary connection logging and retains bounded
policy diagnostics on test failure. Do not reinterpret that initial timeout as
proof of a diagnosed product defect or count a retry as sustained-load evidence.

Initial containment checks on the implementation without temporary connection logging:

- Agent binary unit suite: 279/279 passed, including six network-policy tests and
  the ingress capacity-isolation test.
- Four native process suites: 24/24 executions passed (16 scenario tests plus
  two shared fixture checks repeated in each suite). The opt-in interface test
  is excluded from the default suite, not counted as an ordinary pass.
- Explicit non-loopback interface test: three consecutive final-code runs passed
  in 1.08, 0.81 and 0.78 seconds. These are repeated local observations, not three
  independent hosts or campaigns.
- Strict CLI Clippy (`--all-targets`, `-D warnings`), HTML book/inventory checks,
  project organization audit and `git diff --check` passed.
- Tensor validation passed with no structural gaps, two dimension-presence gaps,
  22 evidence-grade/scope gaps and 17 P0 gaps. Release readiness remains blocked;
  this bounded containment claim does not close any authentication requirement.

Reproduce ordinary checks from the repository root:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --bin kyuubiki-cli --locked --offline
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test agent_network_access_live --test agent_lan_mesh_live --test agent_ingress_live --test agent_recovery_ingress_live --locked --offline
```

For the explicit wire check, set `KYUUBIKI_TEST_AGENT_LAN_IP` to an actual local
non-loopback IPv4 address, then run:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-cli --test agent_network_access_live real_non_loopback_socket_is_denied_then_explicitly_admitted_and_revoked --locked --offline -- --ignored --exact
```

The test refuses an address that cannot be locally bound. It never selects or
contacts another machine, changes firewall settings, or installs a service.

## Managed Deployment Preflight Follow-Up

The next audit reproduced a separate lifecycle defect: managed restart stopped
healthy services before the newly configured Agent policy was validated. An
invalid source-IP list could therefore take down the existing runtime and only
then fail startup. Installer environment validation also did not inspect this
setting.

Two new regression tests failed before the fix: restart removed existing process
records, and fresh start accepted an invalid policy and launched fixture services.
After the fix, both reject before stopping services or publishing new processes.

- The pure parser now lives in `kyuubiki-protocol` and is shared by native Agent
  admission, Installer environment validation and managed-runtime preflight. It
  performs no DNS, environment, filesystem or network operations. The Agent keeps
  transport admission, diagnostics and its bounded denial counter separately.
- Managed restart prepares one startup environment snapshot before stopping any
  owned service. It retains the old deployment mode separately, so a missing mode
  record cannot make a local-to-distributed restart skip existing local Agents.
- Invalid network configuration leaves running process incarnations, process
  records and retained execution data unchanged. Status and explicit stop remain
  available, without reflecting an untrusted invalid value in error messages.
- Runtime summaries expose configured intent with `running-policy=not-observed`;
  repeated start does not silently apply a new policy to reused processes. The
  Agent description remains the source of effective-policy diagnostics.
- Runtime modes that launch no local Agent skip irrelevant Agent policy
  validation. Installer environment-file validation still rejects malformed
  settings in its input; it does not attest to a remote process environment.

These integration fixtures use the real native runtime lifecycle executable with
temporary installed-layout manifests and native test-listener children, not live
Orchestra/database services or the installed user App. The retained-data sentinel
checks non-mutation on preflight failure; real Agent durable-receipt behavior is
covered separately by the native suites above. No running user services, remote
hosts or installed packages were changed.

Follow-up checks after sharing the parser:

- Agent unit suite: 279 passed.
- Four native Agent process suites: 24 passed executions, including repeated
  fixture checks as counted above; one opt-in non-loopback test ignored. The
  earlier three non-loopback runs were not repeated after this refactor.
- Runtime lifecycle integration: 9 passed; network-preflight integration: 6
  passed, covering invalid start/restart, readable status and explicit stop,
  configured-versus-running policy, orchestrator-only startup and local Agent
  cleanup during a distributed restart with no mode record.
- Runtime-related unit selection: 25 passed; one real local-stack test ignored.
- Installer unit suite: 132 passed, including two network-preflight tests that
  verify agreement with native parsing for malformed policies.
- Strict Clippy across protocol, CLI, desktop runtime and Installer, HTML book
  and inventory checks, project organization audit and whitespace checks passed.
- Tensor validation remains at zero structural gaps, two maturity gaps, 22
  evidence-grade gaps and 17 P0 gaps. The existing verified containment evidence
  now links deployment preflight; no installed or authentication grade is raised.

Additional reproduction commands:

```text
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-desktop-runtime --test runtime_network_preflight --test runtime_lifecycle --locked --offline
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-desktop-runtime --lib runtime_ --locked --offline
cargo test --manifest-path workers/rust/Cargo.toml -p kyuubiki-installer --lib --locked --offline
```

Preflight is not a transactional restart guarantee for arbitrary artifact,
readiness, storage or process-launch failures. It also does not qualify every
possible change to ports, endpoint membership or service ownership flags.

## Remaining Boundaries

Source-IP containment is not authentication. It does not prove a user, process,
controller or peer identity; encrypt requests; prevent a permitted host from
misusing execution/recovery; or enforce single-Orchestra cryptographic ownership.
Local processes share loopback trust. NAT, proxies and tunnels are authorized as
the observed socket source, not the original caller. DHCP address reassignment
can both deny an intended peer and later grant another host its old address.
Keep an independently isolated trusted network boundary and review address lists
when topology changes. There is no hot revocation or whole-host DoS guarantee.

The tensor's `native-agent-lan-ingress-authorization` scope remains open, as do
authenticated multi-host recovery, sustained recovery budgets, host-power-loss
durability and matched installed journeys. The new evidence is a bounded local
verified containment claim, not a qualification of any of those scenarios.

See the [runtime book](../docs/book-ch04-runtime-modes.html#native-agent-ingress)
and [October 16 tensor calibration](tensor-recalibration-20261016.md).
