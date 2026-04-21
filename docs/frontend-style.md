# Frontend Style Guide

This guide keeps the workbench visually coherent and implementation-friendly as
the UI grows.

## Product Feel

The workbench should feel like a compact engineering studio:

- calm, technical, and legible
- dense without feeling cramped
- expressive through structure, not decoration
- immersive when editing, efficient when reviewing

## Visual Rules

### Color

- default to soft neutrals and cool technical tones
- use warm accent color for active operations, selection emphasis, and warnings
- use cool accent color for navigation, mode, and structural emphasis
- avoid large saturated fills unless they encode an important state

### Typography

- headers should be strong and fast to scan
- body text should stay quiet and readable
- labels and metadata should use muted contrast, not tiny unreadable text
- monospace is for data, logs, coordinates, IDs, and scripted content

### Surfaces

- prefer layered panels over flat blocks
- keep radius, spacing, and shadow language consistent
- panels inside editing workspaces should feel lighter than app-shell chrome
- if a panel competes with the viewport, move it into a drawer or tab

## Layout Rules

### Workbench

- viewport is the primary surface
- inspectors, object trees, libraries, and reports are secondary surfaces
- secondary surfaces should be docked, tabbed, or collapsible
- avoid whole-page scrolling when panel scrolling will do

### Immersive Modes

- immersive mode should remove non-essential chrome
- tools/help should live outside the scene when possible
- scene overlays should only remain when they directly support editing

### Large Data

- use virtualization for long lists and histories
- use chunked browsing for large result sets
- keep rendering scoped to what the user can currently see or meaningfully use

## Interaction Rules

### Editing

- direct manipulation should be preferred for geometry edits
- parameter editing should remain available alongside dragging
- selection should be visible, reversible, and multi-select friendly

### Controls

- frequent actions should have both click and shortcut paths when practical
- advanced controls should group by task, not implementation detail
- buttons near the viewport should justify their presence; otherwise dock them

### Feedback

- loading, error, and success states should be explicit
- failures should explain what the user can do next
- long-running actions should surface progress or heartbeat information

## Implementation Rules

### React/Next

- keep domain/state transforms in `src/lib` or focused hooks/helpers, not buried
  in render blocks
- components should prefer composition over giant condition-heavy files
- split workbench surfaces by responsibility so one interaction does not force
  a full-page rerender

### Styling

- use the shared tokens in `globals.css` first
- introduce new tokens when a pattern repeats, not for one-off tweaks
- avoid local color drift; new colors should map back to the established theme

### Performance

- viewport and large lists get first-class performance attention
- prefer incremental rendering, memoized structure boundaries, and coarse-grain
  rerender isolation where it materially helps
- do not pay complexity costs before profiling suggests a real bottleneck

## Avoid

- marketing-style hero layouts inside tooling surfaces
- oversized controls that waste viewport area
- deeply nested modal flows for common editing actions
- mixing transport/API logic directly into presentational components
- introducing new visual motifs without aligning them to existing tokens
