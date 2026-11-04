# Agent connection-scoped Mesh selection regression

Date: 2026-10-28. Source line: daji 3.7.1.

## Reproduced gap

After default-deny LAN control was introduced, direct-Mesh legacy solver selection
still ranked a healthy, accepting Agent whose current caller had discovery-only
access. Before the fix, the regression expected
`connection_control_not_permitted` but received `Ok(100)`. Such an Agent could be
selected ahead of a usable peer, only to reject the actual solver request.

## Change and boundaries

- Native `describe_agent` adds `connection_access` with schema
  `kyuubiki.agent-connection-access/v1`, scoped to `native_socket_connection`.
- The native handler derives `host_loopback`, `discovery_only` or
  `trusted_lan_control` from the socket peer and host-owned network policy.
  Caller parameters cannot supply this result. Global descriptors, registration
  and heartbeat do not retain the connection-specific observation.
- The typed protocol validates the exact current schema, modes and consistent
  boolean flags. All three modes report no authentication and no encryption.
- Both `first_reachable` and `healthiest` exclude discovery-only, missing and
  malformed access before dispatch. No eligible peer means zero solver requests.
  A health score is never permission to execute.
- New tasks rediscover permissions; no old permission is cached. Discovery is not
  a lease: the Agent independently enforces the policy on the execution socket.
  Lost replies, solver errors and post-discovery permission denial never cause
  automatic cross-Agent replay.
- Pinned durable recovery remains separate from new-work selection. Busy/draining
  execution capacity does not change the saved owner, and source-IP/LAN-control
  enforcement and owner fencing still apply to recovery requests.
- Agent and gateway should be upgraded together. An old descriptor without
  `connection_access` fails selection with `invalid_connection_access`, rather
  than being silently accepted. This is an intentional fail-closed change.

## Verification

The added regression first failed against the old selection logic. After the fix:

- Desktop-runtime library: 123 passed; two opt-in environment tests skipped in the
  default suite. The new native selection test was then run explicitly below.
- Protocol library: 144 passed, including missing/unknown/malformed fields and
  inconsistent claims of authentication, encryption and lifecycle permission.
- Native CLI unit tests: 332 passed, including spoofed request parameters and
  local/LAN descriptor isolation.
- Native LAN-Mesh integration: 8 passed.
- Native network-access integration: 6 passed in the default suite; its opt-in
  real-interface test was then run explicitly below.
- Native recovery-ingress integration: 5 passed.
- Two opt-in real-interface tests passed, bringing these runs to 620 passing
  test executions. This is a test count, not a percentage of product coverage.
- Workspace formatting and `git diff --check` passed. Strict Clippy for all
  targets of protocol, desktop-runtime and CLI passed with warnings denied.
- The module-function tensor checker passed structural and command checks:
  zero structural gaps, two maturity gaps, 23 evidence-grade gaps and 18 P0
  gaps remain. The release qualification status is still blocked.

`native_selection_skips_discovery_only_lan_and_observes_restart_policy_changes`
starts two matching source-built native Agent processes on temporary ports. One
is reached through this host's actual non-loopback interface, the other through
loopback. Both selection modes skip the discovery-only Agent without sending it
any execution request; the local Agent computes the expected bar stress. A
LAN-only candidate set sends no work. Restarting the LAN Agent with explicit
trust permits a solve; restarting it with trust revoked sends the next new task
to the local Agent. Native execution counters and denial counters corroborate
the chosen paths. Test children are terminated and reaped on exit.

`real_non_loopback_socket_is_denied_then_explicitly_admitted_and_revoked` also
checks that one Agent reports different connection access through loopback and
its LAN interface, while existing access/restart/retained-receipt checks pass.

For opt-in reproduction, set `KYUUBIKI_TEST_AGENT_BINARY` to the matching built
CLI and `KYUUBIKI_TEST_AGENT_LAN_IP` to an explicit address of the test host. The
native selection fixture verifies the address is locally bindable and clears
inherited child environments. No server addresses, credentials or machine-local
configuration belong in this report or the repository.

## Remaining qualification

This is local source-process and actual non-loopback socket evidence. It is not
an installed WebView journey, a two-machine LAN test, cryptographic peer/controller
identity, transport encryption, distributed workflow recovery or host power-loss
qualification. A peer can lie about readiness; this metadata is not a credential.
The installed real-control-stack opt-in test was not run. No app reinstall, remote
deployment, version bump, commit or push is part of this change.

The tensor's native Agent authentication and authenticated multi-host recovery
requirements remain open. Explicit LAN trust is not authentication.
