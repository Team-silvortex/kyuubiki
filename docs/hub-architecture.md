# Kyuubiki Hub

`Kyuubiki Hub` is the unified desktop entrypoint for the whole Kyuubiki
workstation.

It sits above the existing workbench and installer/operator surfaces and gives
the system one place to manage projects, runtimes, deployment modes, logs, and
desktop launch flows.

The most important boundary to keep stable is this:

- `Hub` manages runtimes
- `orchestrator/control plane` is one managed runtime target
- `Workbench` is a launched engineering surface
- `solver agents` are managed compute peers

That means the Hub must not collapse into the control plane. It can start a
local control plane, connect to a remote one, switch across several of them, or
work without one for local bundle and desktop tasks.

## Why a Hub exists

Kyuubiki is no longer one app. It is a multi-load system made of:

- workbench UI
- control plane
- solver agents
- local, cloud, and distributed deployment modes
- project assets and automation presets
- diagnostics, logs, packaging, and benchmark tools

At that scale, the user needs three distinct layers:

- `kyuubiki` CLI
  the automation, packaging, validation, and runtime command layer
- `Kyuubiki Hub`
  the desktop orchestration shell and operator-facing entrypoint
- `Kyuubiki Workbench`
  the focused modeling and analysis surface

## Product split

### `Kyuubiki Hub`

The Hub should own:

- project launcher and recent-work map
- local runtime lifecycle
- remote/distributed runtime registration and health
- deployment mode switching
- environment validation and repair guidance
- logs, watchdog state, and health overview
- benchmark and diagnostics launch
- entry into `Workbench`, `Installer`, and future admin surfaces

### `Kyuubiki Workbench`

The Workbench should own:

- modeling
- materials
- results
- study automation
- immersive editing
- project-level engineering workflows

### `Installer / Operator` workflows

The Hub should absorb more of the day-to-day operator shell role over time.

The current `installer-gui` still remains valuable for:

- bootstrap
- deployment authoring
- cross-platform packaging and release staging

But the long-term direction is:

- `installer-gui`
  setup and heavy deployment tooling
- `hub-gui`
  everyday desktop entrypoint
- `workbench-gui`
  focused engineering surface

## Information architecture

The Hub should feel closer to `Unity Hub` or an engine launcher than to a
generic dashboard.

### Main navigation

- `Projects`
  recent projects, pinned projects, import/export, open in workbench
- `Runtimes`
  local stack, local agents, direct mesh, remote control-plane targets
- `Deploy`
  local/cloud/distributed setup flows, agent manifests, bootstrap actions
- `Observe`
  health, logs, watchdog, security events, cluster topology
- `Tools`
  benchmark, doctor, validate, package, export

### Home layout

- left rail
  main sections
- center workspace
  section content and status boards
- right utility column
  quick actions, log tail, current runtime state, alerts

### Quick actions

- `Open workbench`
- `Start local stack`
- `Restart runtime`
- `Inspect logs`
- `Validate environment`
- `Open project`
- `Import bundle`
- `Run benchmark`

## Runtime model

The Hub should treat runtime targets as first-class entities:

- `local workstation`
- `cloud control plane`
- `distributed control plane`
- `direct mesh`

Within that model, the orchestrator is not the Hub itself. It is one runtime
target family that the Hub can manage:

- `local orchestrator`
  a control-plane process running on the same workstation as the Hub
- `remote orchestrator`
  a control-plane endpoint running elsewhere and reached over protocol/API
- `multi-orchestrator fleet`
  several control-plane targets that the Hub can inspect and switch between

Each target should expose:

- mode
- address / endpoint
- storage backend
- health
- last activity
- linked projects
- linked logs

This split matters because it keeps the Hub useful even when the control plane
is absent, remote, restarting, or one of several managed targets.

## Initial repository shape

The Hub should live in:

- [apps/hub-gui](../apps/hub-gui)

Recommended structure:

- `ui/`
  static shell and interaction prototype
- `src-tauri/`
  native commands and shell integration
- `README.md`
  ownership and rollout notes

## Rollout plan

### Phase 1

- create `hub-gui` shell
- define navigation, section model, and shared branding
- allow launching workbench and installer
- show local runtime health and quick actions

### Phase 2

- project list and recent project launch
- local logs and watchdog cards
- remote target cards
- benchmark and validation entrypoints

### Phase 3

- gradually move operator day-to-day tasks from `installer-gui` into Hub
- keep installer for bootstrap and heavier deployment flows
- reduce duplicated lifecycle controls between shells

## Non-goals for the first cut

- replacing the browser workbench
- embedding every operator workflow immediately
- rewriting installer/runtime logic that already works

The Hub should start as a clear orchestration shell, not as a second giant app.
