# Scripts

This directory contains host-native operational entry points.

- `kyuubiki`
  Unified launcher for local, cloud, and distributed development flows.

Use this directory for operator-facing workflow wrappers, not for source
libraries or generated output.

Typical responsibilities:

- start/stop/restart orchestration
- mode switching (`local`, `cloud`, `distributed`)
- verification/test wrappers
- packaging and installer entry points

Keep these scripts thin. Product logic should live in the application/runtime
code, not in shell branching.
