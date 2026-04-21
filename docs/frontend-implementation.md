# Frontend Implementation Guide

This guide turns the frontend philosophy into concrete implementation rules.

It is mainly for `apps/frontend`, but the same ideas should influence desktop
shell surfaces that embed or wrap the workbench.

## Goals

Frontend code should be:

- easy to evolve without accidental full-page rerenders
- explicit about domain boundaries
- safe for large-model and large-result workflows
- readable enough that UI changes do not require archaeology

## Responsibility Split

### `src/app`

Use `src/app` for route entry points, top-level composition, and global shell
concerns.

- route wiring
- page-level bootstrapping
- global CSS tokens and theme plumbing
- API route handlers for frontend-owned runtime surfaces

Do not bury reusable domain logic here.

### `src/components/workbench`

Use this directory for domain-specific UI surfaces.

Examples:

- viewport
- inspector
- console/report panel
- object tree
- assistant panel
- script panel

Rules:

- each component should own one visible surface or one tightly related cluster
  of interactions
- components may be stateful, but should not absorb unrelated business logic
- if a component grows multiple independent modes, split those modes into child
  surfaces before adding more condition-heavy JSX

### `src/components/ui`

Use this directory only for reusable primitives that are not specific to FEM or
to the workbench domain.

Good fits:

- virtualized lists
- generic tabs
- lightweight drawers
- buttons, pills, segmented controls, shared form shells

Bad fits:

- model-specific editors
- result-specific tables
- viewport tools with FEM semantics

### `src/lib`

`src/lib` owns domain and transport logic that should be usable outside render
code.

Good fits:

- API clients
- model import/export
- parametric model generation
- material parsing and library transforms
- project bundle handling
- assistant/runtime bridges
- selection helpers, geometry transforms, and chunk-window logic

Bad fits:

- JSX fragments
- browser-only UI glue that only exists to satisfy one component layout

## State Placement Rules

### Keep source-of-truth state near the smallest shared owner

Ask:

1. Which surfaces need to read this state?
2. Which surface is allowed to mutate it?
3. Is it domain data, UI state, or transient interaction state?

Use that answer to place state.

### Put domain state above presentation

Examples:

- loaded model
- selected materials
- active job/result payloads
- assistant transaction history
- persisted settings

These should live in a coordinating surface or dedicated helper layer, not deep
inside a leaf component that only renders them.

### Keep transient interaction state local when possible

Examples:

- hover state
- drawer open/closed state for one panel
- pointer-drag bookkeeping
- local scroll sync

Do not lift this state unless multiple siblings truly need it.

### Separate persisted state from ephemeral state

- persisted state affects future sessions or exported work
- ephemeral state only affects the current interaction

Do not mix them in the same update path unless there is a strong reason.

## Component Splitting Rules

Split a component when one or more of these become true:

- it owns multiple visually independent panels
- a change in one section rerenders unrelated heavy sections
- the file starts mixing transport, transformation, and rendering concerns
- one mode could be developed or tested independently from the others

For large surfaces, prefer this pattern:

1. coordinator component
2. focused surface components
3. `src/lib` helpers for transforms and side-effectful logic

## `workbench.tsx` Rule

`workbench.tsx` may coordinate the workbench, but it should not become the only
place where every rule lives.

It should gradually move toward:

- app-shell orchestration
- cross-panel state ownership
- mode switching
- high-level command wiring

It should gradually move away from:

- geometry algorithms
- result window transforms
- material normalization details
- giant inline render helpers
- unrelated action-specific update logic

## Naming Rules

### Components

- use full domain names, not vague UI names
- prefer `WorkbenchViewport`, `WorkbenchInspector`, `MaterialLegend`
  over `Panel`, `Box`, `Thing`

### Helpers

- use verb-led names for transforms and commands:
  `buildStudyModelPayload`, `exportProjectBundle`, `parseMaterialLibrary`
- use noun-led names for stable types and domain concepts:
  `ResultWindowState`, `AssistantTransactionEntry`

### Files

- route files describe route ownership
- component files describe rendered surface ownership
- helper files describe domain ownership

Avoid names that only describe implementation accidents.

## Performance Rules

### Protect the expensive surfaces

First-class performance-sensitive areas:

- viewport rendering
- virtualized histories/libraries/object trees
- large result browsing
- large selection/multi-edit flows

### Rerender intentionally

- avoid passing unstable freshly built objects through large trees without need
- defer derived views when the user benefits from responsiveness
- isolate heavyweight surfaces behind clear prop boundaries
- do not centralize every tiny update at the page root

### Scale by visibility

- render what is visible or actionable first
- prefer chunking, virtualization, and clipping before adding exotic complexity
- make large-model behavior explicit in the code, not accidental

## Styling Rules

- use shared tokens from `globals.css`
- keep spacing, radius, and shadow changes token-driven when repeated
- prefer semantic class naming tied to the surface role
- avoid scattered one-off color decisions that drift from the theme

## Testing Rules

- test pure transforms in `src/lib` as logic, not through full UI render paths
- test user-critical flows through higher-level integration or smoke coverage
- when a UI change affects a contract or workflow boundary, add coverage at the
  boundary that would actually fail for a user

## Review Checklist

Before merging a frontend change, ask:

1. Did this strengthen or weaken the current component boundaries?
2. Did any heavy surface start rerendering more than before?
3. Did domain logic move toward `src/lib`, or back into JSX?
4. Did the visual result get calmer and denser, or noisier and more scattered?
5. Will this still make sense when the workbench grows another mode or panel?
