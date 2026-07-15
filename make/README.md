# Make Modules

The root `Makefile` is intentionally only a small entrypoint.

Keep target implementation in this directory so the repository-level Make
surface can grow without turning the root file into another unreviewable
control script.

Current modules:

- `help.mk`
  Human-facing target descriptions. This stays included first so
  plain `make` still defaults to the help target.
- `checks.mk`
  Audit, architecture, reliability, readiness, and verification gates.
- `tests.mk`
  Unit, SDK, GUI smoke, integration, formatting, and focused TDD targets.
- `benchmarks.mk`
  Benchmark profile, standard matrix, remote regression, and report targets.
- `build.mk`
  Build, package, generated-doc, and operator-package preflight targets.
- `runtime.mk`
  Local runtime, hot-reload, service, SDK entrypoint, and smoke targets.
- `desktop.mk`
  Tauri desktop shell build, dev, release, and remote Linux desktop targets.
- `misc.mk`
  Small repository utility targets that do not justify a dedicated module yet.

Next split points:

- split `build.mk` further if packaging and generated docs start to grow
- split `runtime.mk` if service lifecycle and headless command aliases diverge

Rules:

- Put shared variables in the root `Makefile` unless they are module-local.
- Keep each `.mk` file below the shared 800-line source limit.
- Add new public targets to the narrowest matching module instead of creating
  catch-all target files.
- Prefer forwarding to `./scripts/kyuubiki` or native Rust utilities instead of
  adding shell-heavy recipes.
- When moving a target, keep its public target name stable unless the old name
  is deliberately deprecated.
