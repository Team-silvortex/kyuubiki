# Orchestrator Boundary

`apps/web` is the Elixir orchestration boundary for Kyuubiki.

Current responsibilities:

- accept FEM job submissions over HTTP
- normalize study input and create job records
- dispatch numerical work to the Rust solver agent over TCP
- expose health and orchestration APIs for the Next.js workbench

Planned Phoenix-facing responsibilities:

- migrate this boundary into a Phoenix API application
- add persistence for projects, studies, and jobs
- add PubSub or channels for live progress streaming
- add authentication and multi-user boundaries when needed
