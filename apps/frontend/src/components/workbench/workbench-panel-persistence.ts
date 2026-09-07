import { WORKBENCH_PANEL_LAYOUT_KEY, parseWorkbenchPanelPreferences, type WorkbenchPanelPreferences } from "./workbench-panel-layout.ts";
import type { PanelLayoutRecord, WorkbenchPanelLayoutClient } from "../../../../desktop-shared/src/workbench-panel-layout-bridge.ts";

type Options = {
  storage: () => Pick<Storage, "getItem" | "setItem" | "removeItem">;
  client: WorkbenchPanelLayoutClient | null;
  restore: (sizes: WorkbenchPanelPreferences) => void;
  status: (status: string) => void;
};

export function createWorkbenchPanelPersistence({ storage, client, restore, status }: Options) {
  let initial: WorkbenchPanelPreferences = {};
  let revision = 0;
  let disposed = false;
  let reading = false;
  let ready = false;
  let writing = false;
  let queued: PanelLayoutRecord | null = null;
  try { initial = parseWorkbenchPanelPreferences(storage().getItem(WORKBENCH_PANEL_LAYOUT_KEY)); }
  catch { status("memory"); }
  const report = (value: string) => { if (!disposed) status(value); };
  function cache(sizes: WorkbenchPanelPreferences) {
    try {
      if (Object.keys(sizes).length) storage().setItem(WORKBENCH_PANEL_LAYOUT_KEY, JSON.stringify({ version: 1, sizes }));
      else storage().removeItem(WORKBENCH_PANEL_LAYOUT_KEY);
      return true;
    } catch { return false; }
  }
  function flush() {
    if (!client || !ready || writing || !queued || disposed) return;
    const next = queued;
    queued = null;
    writing = true;
    report("native-saving");
    void client.write(next).then(() => {
      if (!queued) report("native");
    }).catch(() => { report("memory"); }).finally(() => { writing = false; flush(); });
  }
  function read() {
    if (!client || reading || disposed) return;
    reading = true;
    report("native-loading");
    void client.read().then(layout => {
      if (disposed) return;
      ready = true;
      // A late read must not undo a drag (including an in-flight one), keyboard edit or reset.
      if (revision === 0 && layout) { restore(layout.sizes); cache(layout.sizes); }
      report("native");
      flush();
    }).catch(() => { report("memory"); }).finally(() => { reading = false; });
  }
  read();
  return {
    initial,
    changed: () => { revision++; },
    save: (sizes: WorkbenchPanelPreferences) => {
      revision++;
      const saved = cache(sizes);
      if (!client) { report(saved ? "saved" : "memory"); return; }
      // Coalesce while IPC is busy, keeping only the latest committed preference.
      queued = { version: 1, sizes: { ...sizes } };
      if (!ready) read();
      else flush();
    },
    dispose: () => { disposed = true; client?.dispose(); },
  };
}
