# Hub Docs Shelf

This directory is the desktop-facing HTML docs shelf used by the Hub shell.

## Ownership

- `docs/`
  remains the repository-level engineering and product documentation source of
  truth.
- `apps/hub-gui/ui/docs/`
  is the desktop-facing HTML shelf for operators and end users inside Hub.
- `index.html`
  is the Hub-facing entry to the centralized Kyuubiki Book and should stay
  aligned with `docs/book.html`.

## How to treat these files

- keep Hub docs short, scan-friendly, and release-facing
- when a topic also exists under `docs/`, treat the Markdown or root HTML doc
  there as the deeper source narrative
- avoid copying local machine paths or repo-private workflow notes into this
  shelf
- generated contract pages may be refreshed by tooling, so do not assume every
  HTML file here is hand-maintained forever

## Current mirrored topics

- `index.html`
  Desktop dispatch page that links both the short Hub mirrors and the
  repository-level chapter pages of the centralized book.
- `current-line.html`
- `installation-integrity.html`
- `operations.html`
- `troubleshooting.html`
- `update-catalog.html`
  Hub mirror for the centralized book's update-visibility material and release
  channel posture.
- `solver-matrix-optimization-pack.html`
  Hub mirror for the retained Rust solver matrix optimization set and its
  benchmark-backed validation note.

If a topic needs long-form engineering detail, put that detail under `docs/`
and keep the Hub page as the short desktop entrypoint.
