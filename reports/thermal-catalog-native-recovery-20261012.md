# Native thermal catalog decisions and explicit rerun

## Findings

The previous project-receipt sweep exposed three catalog integration failures.
Their fake Agent queues supplied only solver responses, although diagnostics now
execute through native TaskIR. A diagnostic request either consumed the next
solver response as a receipt or reached a closed listener. Production task receipt
validation was correct and is unchanged.

Correcting the fixtures was not sufficient. Forwarding the same diagnostic tasks
to a freshly built Rust Agent exposed a real contract gap: the thermal guard
reported `warn` rather than `block`, and the benchmark omitted criteria. Native
diagnostics emitted `thermal_flux_peak_magnitude` but the stored catalog used
`thermal_peak_flux_magnitude`. Native output also lacked the legacy nonzero loaded
and heated node counts. Downstream thermal guards and coupled comparisons silently
skipped missing evidence, allowing incomplete conclusions to look successful.

## Corrections

- Native vector diagnostics preserve both metric naming forms, components and peak
  identities. Aliases share the validated reduction rather than rescanning inputs.
- Scalar distributions count nonzero samples in the existing pass. Thermal and
  thermo diagnostics publish loaded/heated node counts, with zero and negative
  values distinguished correctly. Missing groups do not become invented zeros.
- Thermal output retains the total heat-load alias; thermo displacement accepts
  both `ux`/`uy` and solver `displacement_x`/`displacement_y`. Explicit mappings and
  invalid selected values still fail rather than falling through to healthy aliases.
- Orchestra thermal guards now require every configured metric and reject invalid
  rules. Coupled comparisons require every criterion on both candidates, rather
  than quietly shrinking the denominator. Missing evidence is an error, not a
  pass, a warning downgrade or a partly evaluated winner.
- A supervised, bounded test Agent checks RPC method, TaskIR identity, digest,
  input/config binding and session order. Default mode uses an explicitly marked
  reference adapter; optional live mode forwards diagnostic requests unchanged to
  a local Rust Agent. Neither mode pretends fixture solver output is a fresh solve.

## Verification

- Rust Engine workflow unit selection: **591 passed**, zero failed or ignored.
  This includes five new catalog metric controls; do not count them twice.
- Rust CLI native diagnostic TaskIR unit selection: **3 passed**, zero failures.
- Catalog API suite with an actual local Rust Agent: **9 passed**, zero failures.
  Thermal guard, thermal/thermo benchmark and coupled electrostatic/heat/thermo
  markdown export complete with their original expected conclusions.
- Fault controls reject four stale receipt identities and pending execution
  acknowledgements. A valid executed receipt with a removed required flux metric
  also blocks the report. Each case then explicitly submits a new job and verifies
  a correct report without inheriting the old task identity or failed output.
- Expanded Library/storage/API/native-recovery suite: **597 tests**, **550 passed**,
  **47 conditional skips**, zero failures, seed `599985`, 15.5 seconds. It includes
  four new direct missing-metric and malformed-rule/criterion controls.

The expanded sweep initially reproduced a separate lock failure in the newly
added project-receipt concurrency fixture. It used DELETE journal mode for runtime
concurrent writers. It now uses runtime WAL mode and the existing 5,000 ms busy
budget; new files enter WAL before pool startup, avoiding a journal-mode race.
Two connections and eight concurrent project requests remain. The final same-seed
sweep has neither test failures nor connection-lock warnings. This is not a claim
that arbitrary concurrent DELETE-mode writes or prolonged lock contention are
qualified; offline copy-upgrade controls remain separate.

## Reproduction and limits

From `workers/rust`, build the Agent with `cargo build --locked --offline -p
kyuubiki-cli`. Run it with a clean environment on an unused loopback port, without
an Orchestra binding, credentials or operator-package store. From `apps/web`, set
`KYUUBIKI_WORKFLOW_DIAGNOSTIC_AGENT_PORT` to that port and run:

```text
mix test test/kyuubiki_web/api/workflow_catalog_thermal_job_api_test.exs test/kyuubiki_web/api/workflow_coupled_diagnostics_template_api_test.exs
```

Without that variable, the same suite uses computed Elixir reference diagnostics
as a transport fixture. Live mode has no fallback if the Agent is unavailable.
Only diagnostics execute in Rust in this catalog fixture round; solver frames
remain synthetic. Existing numerical suites are not replaced by these controls.

This is contract, execution and explicit-rerun evidence, not physical accuracy,
fresh end-to-end numerical studies, automatic resume of the same interrupted job,
power-loss recovery, PostgreSQL, installed GUI or remote qualification. The owned
temporary Agent was stopped cleanly. No installed runtime/database was changed,
no application was repackaged, and no version bump, commit or push was performed.
