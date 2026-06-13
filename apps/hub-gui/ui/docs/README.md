# Hub Docs Shelf

This directory is the desktop-facing HTML docs shelf used by the Hub shell.

## Ownership

- `docs/`
  remains the repository-level engineering and product documentation source of
  truth.
- `apps/hub-gui/ui/docs/`
  is the desktop-facing HTML shelf for operators and end users inside Hub.

## How to treat these files

- keep Hub docs short, scan-friendly, and release-facing
- when a topic also exists under `docs/`, treat the Markdown or root HTML doc
  there as the deeper source narrative
- avoid copying local machine paths or repo-private workflow notes into this
  shelf
- generated contract pages may be refreshed by tooling, so do not assume every
  HTML file here is hand-maintained forever

## Current mirrored topics

- `current-line.html`
- `installation-integrity.html`
- `operations.html`
- `troubleshooting.html`
- `update-catalog.html`

If a topic needs long-form engineering detail, put that detail under `docs/`
and keep the Hub page as the short desktop entrypoint.
