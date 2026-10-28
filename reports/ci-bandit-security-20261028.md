# CI Bandit Security Dependency Repair

Reviewed October 28, 2026. Scope: the Orchestra HTTP transport dependency and
its regression tests, not workflow recovery or numerical solver changes.

## Failure Evidence

[CI run 40650766467](https://github.com/Team-silvortex/kyuubiki/actions/runs/40650766467)
tested commit `59b95efe648d9e8bd0feb12839f227328504e611`. Its dependency audit job
`122690043260` rejected the locked Bandit 1.13.1 version. Both `mix hex.audit`
and the live OSV Hex query failed; OSV reported 11 unmitigated advisories.
All four npm and five Rust audit lanes passed. This is separate from the
previous browser-preparation timeout repair.

The [upstream 1.14.0 changelog](https://bandit.hexdocs.pm/changelog.html)
and [published package](https://hex.pm/packages/bandit/1.14.0) identify the
October 26 security release. It addresses HTTP/1 parsing and chunk handling,
HTTP/2 flow control, header handling, stream work and timeouts, and WebSocket
close/fragment handling. For example, the
[chunk-extension advisory](https://github.com/mtrudel/bandit/security/advisories/GHSA-w5wc-g2hc-g7m2)
and [HTTP/2 flow-control advisory](https://github.com/mtrudel/bandit/security/advisories/GHSA-c3g7-2pg5-qvwh)
both identify 1.14.0 as patched.

## Scoped Repair

- Raise the Bandit requirement to `~> 1.14.0` and lock 1.14.0. No transitive
  dependency version changes, audit exceptions or CI gate relaxations.
- Guard the minimum safe version, the manifest requirement and the actual
  loaded application against the lockfile in the transport security tests.
- Add actual loopback TCP probes using the production transport options.
  Bare CR and LF in chunk extensions must produce HTTP 400 without body
  acceptance. Ordinary chunks, token/quoted extensions and permitted optional
  whitespace must still produce HTTP 200 with the expected body.
- Bound probes to small bodies, a five-second response deadline and an
  eight-KiB response check. Each test owns its ephemeral listener and closes
  its client sockets; installed services are not touched.

## Validation

Before upgrading, the eight transport tests reported four failures: the
version-floor guard, both malformed-extension probes (incorrect HTTP 200),
and the valid optional-whitespace control (incorrect HTTP 400). After the
upgrade, all eight passed, with no skipped tests.

`make audit-dependencies` passed its self-test and all 11 audit lanes. The
live OSV response covered 18 locked Hex packages, reporting zero known
vulnerability records and zero explicit mitigations. No cached response or
advisory suppression was used.

On the current macOS working tree, `mix test --warnings-as-errors` completed
in 192.0 seconds: 3,856 tests, zero failures, 77 existing environment/backend
conditional skips. This includes the uncommitted workflow-recovery tests;
those changes remain outside the CI dependency repair. No new skip was added.
Full Web formatting, project organization (800 source / 2,000 documentation
lines), documentation inventory and whitespace checks passed.

These bounded protocol tests do not reproduce every resource-exhaustion
advisory or prove safety under arbitrary hostile traffic. The source upgrade
also does not update previously installed Orchestra release binaries; those
need rebuilding and deployment to receive the fixed dependency.
