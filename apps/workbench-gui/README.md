# Workbench GUI

`workbench-gui/` is the Tauri desktop shell for the Kyuubiki workbench.

It is intentionally thin:

- embeds the local workbench at `http://127.0.0.1:3000`
- exposes native `start / restart / stop / status` controls for the local stack
- exposes quick local runtime log viewing for `frontend`, `orchestrator`, and bundled agents
- keeps the browser-first workbench implementation separate from the desktop shell

Main parts:

- `ui/`
  static desktop shell UI and embedded iframe surface
- `src-tauri/`
  native commands that call the shared `scripts/kyuubiki` runtime entrypoint

Useful commands:

- `make workbench-gui-dev`
- `make workbench-gui-build`

Branding:

- desktop icons: [src-tauri/icons](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui/src-tauri/icons)
- shell visuals: [ui/assets](/Users/Shared/chroot/dev/kyuubiki/apps/workbench-gui/ui/assets)
