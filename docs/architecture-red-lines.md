# Architecture Red Lines

Use this document as the practical boundary checklist for Kyuubiki.

The goal is not to make evolution harder. The goal is to stop the system from
quietly collapsing its product layers into one another.

If a proposed change crosses one of these lines, it needs a deliberate
architecture review rather than an opportunistic implementation.

## Core Rule

These four roles stay distinct:

- `Hub`
  system entrypoint and global workload shell
- `Workbench`
  concrete engineering workflow surface
- `Installer`
  deployment, lifecycle, integrity, and repair surface
- `Runtime / Agent`
  execution and protocol surface

The red lines below exist to preserve that split.

## Red Lines

### 1. `Hub` must not become the workflow editor

Do not let `Hub` absorb:

- node-graph editing
- operator parameter editing
- study modeling panels
- result-inspection flows that belong to active engineering work

`Hub` may launch, summarize, inspect, and route into a workflow context.
It must not become a second `Workbench`.

### 2. `Hub` must not become the deployment authoring tool

Do not let `Hub` absorb:

- full install/uninstall workflows
- path-policy editing
- disk cleanup policy authoring
- deep repair operations
- detailed runtime package layout management

`Hub` may show status, warnings, and entrypoints into those actions.
The heavy deployment logic belongs to `Installer`.

### 3. `Workbench` must not own runtime topology

Do not let `Workbench` become responsible for:

- agent installation
- runtime package deployment
- orchestration fleet configuration
- storage layout policy
- platform-specific repair logic

`Workbench` may choose a runtime target or execution mode for a task.
It should not become the authority for system deployment shape.

### 4. `Workbench` must not encode agent internals

Do not make frontend workflow behavior depend on:

- agent-local file layout assumptions
- private runtime module names
- process tree details
- internal orchestration state not exposed through contracts
- UI-specific exceptions embedded in runtime code

If `Workbench` needs something from the execution layer, add or refine a
contract.

### 5. `Installer` must not become an engineering workspace

Do not add to `Installer`:

- project editing
- workflow authoring
- result visualization as a primary surface
- domain-specific study panels

`Installer` can validate a project package or runtime dependency surface, but
it is not where engineering work should live.

### 6. Runtime code must not depend on frontend implementation details

Do not let runtime or agent behavior rely on:

- React component structure
- DOM layout
- Hub navigation state
- installer panel hierarchy
- frontend-only naming conventions with no protocol meaning

Runtime code should speak in:

- schemas
- manifests
- RPC methods
- capability descriptors
- dataset contracts

### 7. Frontends must not bypass contracts to reach internals

Do not let frontends reach directly into:

- Elixir internal modules as hidden product APIs
- Rust internal runtime details as UI behavior dependencies
- private file layouts as if they were public contracts
- ad hoc local process state with no documented interface

If a UI needs the behavior repeatedly, expose it through a stable boundary.

### 8. `orchestra` must not become an unbounded god object

Do not keep stacking unrelated authority into one hidden center without review.

Watch for uncontrolled growth in:

- scheduling
- package authority
- update authority
- install authority
- audit authority
- workflow state authority
- user-facing product logic

`orchestra` can be a central control plane. It must not become the place where
every missing abstraction goes to hide.

### 9. Agent identity and control authority must remain explicit

Do not allow:

- one agent bound to several orchestras at the same time
- silent control-mode flips
- invisible authority changes
- hidden package authority reassignment

Agent control state must stay visible and contract-driven.

See [agent-control-authority.md](agent-control-authority.md)
and [operator-library-centralization.md](operator-library-centralization.md).

### 10. Overlapping visibility is allowed; overlapping ownership is not

Several surfaces may display the same runtime facts:

- `Hub` may show runtime health
- `Workbench` may show execution context
- `Installer` may show install state

That is fine.

What is not fine is letting shared visibility blur ownership of:

- deployment actions
- workflow authoring
- runtime architecture
- protocol design

## Contract Escalation Rule

When a feature seems to need two layers at once, do not immediately merge the
layers.

Escalate in this order:

1. ask whether the need is really shared or only looks shared
2. prefer a contract addition over a layering collapse
3. prefer shared visibility over shared ownership
4. only merge responsibilities if the old split is clearly artificial

## Smell List

Pause for review if you hear phrases like:

- "it was easier to just do it in Hub too"
- "Workbench needs to know the runtime internals here"
- "Installer already has a panel, so we can edit workflows there"
- "the runtime can just special-case this UI surface"
- "we can call the internal module directly for now"
- "orchestra can hold that too"

Those are usually signs that a boundary is being traded away for convenience.

## Review Questions

Before shipping a cross-layer feature, ask:

1. Which layer owns this behavior?
2. Which other layers only need visibility into it?
3. Is there a stable contract at the boundary?
4. Would this still make sense if the frontend were replaced?
5. Would this still make sense if the runtime moved to another machine?
6. Are we adding a true capability, or just leaking one layer into another?

If those questions do not have clean answers, the design probably needs one
more pass before implementation.
