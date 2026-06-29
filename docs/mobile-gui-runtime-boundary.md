# Mobile GUI Runtime Boundary

Mobile operating systems are valid GUI targets, but they are not valid Kyuubiki
runtime hosts.

This is a product boundary, not a temporary implementation gap. iOS and Android
permission models do not provide the process, filesystem, socket, installer, or
long-running service control that Kyuubiki needs for local agents and orchestra.
The mobile surface should therefore be treated as a WebView control client.

## Allowed mobile role

A mobile GUI may:

- open Hub or Workbench as a WebView surface
- select a remote backend target
- authenticate to an orchestrator, gateway, or compatible service
- author and inspect workflows
- submit jobs to a remote runtime
- observe agents, mesh state, logs, progress, and results through public APIs
- export or share client-side reports when the platform allows it

## Forbidden mobile role

A mobile GUI must not:

- install or repair local Kyuubiki runtimes
- host orchestra
- host a Rust agent
- assume `localhost` is the execution backend
- write hidden runtime state outside visible app storage
- bypass backend contracts by importing runtime internals

## Architectural consequence

The same GUI code should run in desktop WebView, browser, and mobile WebView
hosts, but the capabilities are different:

- desktop WebView can manage local workstation runtimes through Installer
- browser can control same-origin or configured remote backends
- mobile WebView can only control remote backends

This is why GUI and runtime must stay decoupled. Mobile support is only viable
if Workbench is a stable client of backend contracts rather than a wrapper around
local agent processes.

## Implementation anchor

The frontend capability contract lives in:

- `apps/frontend/src/lib/api/gui-runtime-capabilities.ts`

That contract should be used by future mobile shells to hide or disable local
runtime installation, local agent launch, and local orchestra controls while
preserving workflow authoring and remote execution.

It also exposes a backend-target decision helper so mobile shells can reject
`localhost`, `127.0.0.0/8`, and `[::1]` backend targets before users accidentally
point the WebView at a runtime that can only exist on a desktop host.
