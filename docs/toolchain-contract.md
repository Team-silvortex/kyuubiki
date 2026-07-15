# Kyuubiki Toolchain Contract

`config/toolchains.json` is the single source of truth for development,
server testing, Docker test images, installer bootstrap, and future self-host
runtime provisioning.

The contract separates compatibility from bootstrap defaults:

- Rust uses a pinned channel because the workspace should compile the same way
  on local machines, lab nodes, containers, and installer-managed runtimes.
- Elixir uses a Mix constraint plus lab defaults because patch/minor releases
  can move while the application contract remains compatible with `~> 1.19`.
  Self-host and container baselines must still satisfy the explicit minimum in
  `config/toolchains.json`; for `moxi 2.0.0` that means Elixir/Mix
  `2.0.0+` and OTP `28.0+`.
- Node uses a compatibility engine range plus a preferred installer version so
  UI shells and docs tooling can run on current local Node releases while
  installer-managed runtimes still have a stable default.

Run `make check-toolchains` before changing any Dockerfile, package manifest,
remote lab script, or language runtime version. The installer should read the
same JSON contract when provisioning self-hosted runtimes instead of carrying a
second private version table.

Run `make check-elixir-self-host` before treating a machine as an
installer-managed control-plane host. That check verifies the actual `elixir`,
`mix`, and OTP versions against `config/toolchains.json`, then confirms the
orchestrator self-host environment keys are referenced by `apps/web/config`.
If it reports a host-installed Elixir below the minimum, use the
installer-managed Elixir/OTP runtime declared by the same contract or prepend
that runtime's `bin` paths before launch.
For image-building stages where Elixir is not installed yet, use
`./scripts/kyuubiki check-elixir-self-host --static-only --json` to validate
the static contract without probing the runtime.

Portable releases also generate
`dist/<platform>/manifests/embedded-runtimes.json` from this same contract. That
manifest is the installer-facing promise that Kyuubiki can carry Elixir/OTP and
Node as managed runtime payloads, similar to products that bundle their
language runtime dependencies instead of asking users to install them manually.
Runtime launchers prepend manifest-declared runtime `bin` paths before starting
services and expose host fallback in `status`; set `KYUUBIKI_RUNTIME_STRICT=1`
to make missing required embedded runtimes a deployment failure.
