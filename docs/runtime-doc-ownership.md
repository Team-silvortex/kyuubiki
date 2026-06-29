# Runtime Documentation Ownership

Use this note when several runtime-facing docs seem to say similar things.

The goal is not to merge everything into one file.
The goal is to keep one primary owner per topic and make the other files point
back to it instead of re-explaining the same boundary in full.

## Primary ownership map

### Product-layer split

- Primary:
  `app-runtime-boundaries.md`
- Owns:
  Hub vs Workbench vs Installer vs runtime role separation, including mobile
  WebView as a remote-control GUI host rather than a runtime host
- Should not absorb:
  agent registration policy, protocol details, or deployment command recipes

### Mobile GUI host boundary

- Primary:
  `mobile-gui-runtime-boundary.md`
- Owns:
  mobile WebView capability limits, remote-backend-only posture, and forbidden
  local runtime assumptions
- Should not absorb:
  general Hub/Workbench/Installer role definitions or agent protocol details

### System shape

- Primary:
  `system-overview.md`
- Owns:
  top-level map of GUI, control plane, solver data plane, and deployment
  shapes
- Should not absorb:
  fine-grained authority rules or step-by-step operator procedures

### Runtime authority and control binding

- Primary:
  `agent-control-authority.md`
- Owns:
  one-orchestrator-or-offline-mesh rule, visible binding fields, and legal
  transitions
- Should not absorb:
  full protocol catalog or generic product-layer explanation

### Agent versus orchestrator architecture boundary

- Primary:
  `agent-orchestrator-boundary.md`
- Owns:
  runtime-side separation between Rust agents, Elixir orchestration, frontend
  shells, and transitional bridge inventory
- Should not absorb:
  deployment commands or installer workflow details

### Headless runtime contract

- Primary:
  `headless-agent-contract.md`
- Owns:
  stable headless runtime contract, solver RPC layering, and the rule that
  frontend direct-mesh routes are gateways rather than the runtime source of
  truth
- Should not absorb:
  desktop product-role descriptions, binding-state transition rules, or
  release-lane guidance

### Operator/runtime procedures

- Primary:
  `operations.md`
- Owns:
  deployment modes, discovery modes, watchdog controls, security entrypoints,
  and everyday operator runtime commands
- Should not absorb:
  architectural theory already anchored in boundary docs or the full authority
  state machine

### Installer-owned remote node control

- Primary:
  `installer-remote-control.md`
- Owns:
  remote bootstrap, certificate alignment, mesh preparation, workflow snapshot
  inspection, and Installer-side authority over that surface
- Should not absorb:
  generic solver RPC semantics or broad system-overview narrative

### Public protocol surface

- Primary:
  `protocols.md`
- Owns:
  public HTTP and TCP contract families
- Should not absorb:
  high-level product narrative or control-authority policy explanation

## Pointer rules

When touching one of these docs, prefer pointers instead of restating the whole
story:

- from `system-overview.md`
  point to `app-runtime-boundaries.md`, `agent-orchestrator-boundary.md`, and
  `installer-remote-control.md`
- from `app-runtime-boundaries.md`
  point to `agent-control-authority.md` and `headless-agent-contract.md`
- from `operations.md`
  point to `installer-remote-control.md`, `security.md`, and `protocols.md`
- from `headless-agent-contract.md`
  point to `protocols.md` and `agent-control-authority.md`
- from `agent-orchestrator-boundary.md`
  point to `agent-control-authority.md` for binding rules and to
  `operations.md` for runtime procedures

## Current duplication risks

Watch these pairs first:

- `system-overview.md` vs `app-runtime-boundaries.md`
  Keep overview in the former and role split in the latter.
- `agent-orchestrator-boundary.md` vs `headless-agent-contract.md`
  Keep architecture separation in the former and headless runtime contract in
  the latter.
- `agent-control-authority.md` vs `headless-agent-contract.md`
  Keep binding-state rules in the former and transport/layering rules in the
  latter.
- `operations.md` vs `installer-remote-control.md`
  Keep general runtime operation in the former and Installer-owned remote-node
  control in the latter.
- `operations.md` vs `headless-agent-contract.md` vs `agent-control-authority.md`
  Keep procedures in `operations.md`, headless runtime contract layering in
  `headless-agent-contract.md`, and authority modes/transitions in
  `agent-control-authority.md`.

## Archive and reduction posture

No immediate archive is required today, but future cleanup should prefer:

- reducing repeated boundary prose in `operations.md`
- reducing repeated authority prose in `headless-agent-contract.md`
- keeping Hub shelf pages as short mirrors, never as the primary source

If a future change needs new runtime-facing docs, add the ownership line here in
the same patch.
