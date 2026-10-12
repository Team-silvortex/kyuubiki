import { captureWorkbenchApiAuthority } from "@/lib/api/auth-context";
import { workbenchProjectContextChangedError, type WorkbenchProjectContext } from "@/lib/workbench/project-context";

export type WorkbenchLibraryMutationTarget =
  | { kind: "createProject" }
  | { kind: "importProject" }
  | { kind: "project"; projectId: string }
  | { kind: "model"; projectId: string; modelId: string };

export class WorkbenchLibraryWritePendingError extends Error {
  constructor(label: string, readonly isCurrent: () => boolean) {
    super(`WORKBENCH_LIBRARY_WRITE_PENDING: ${label}`);
  }
}

function conflicts(left: WorkbenchLibraryMutationTarget, right: WorkbenchLibraryMutationTarget) {
  if (left.kind === "importProject" || right.kind === "importProject") return left.kind === right.kind;
  if (left.kind === "createProject" || right.kind === "createProject") return left.kind === right.kind;
  return left.projectId === right.projectId && (left.kind === "project" || right.kind === "project" || left.modelId === right.modelId);
}

export function createWorkbenchLibraryMutations(context: WorkbenchProjectContext, capture = () =>
  captureWorkbenchApiAuthority("/api/v1/projects", "/api/v1/models", "/api/v1/model-versions")) {
  const buckets: Array<{ isCurrent: () => boolean; targets: Set<WorkbenchLibraryMutationTarget> }> = [];
  const safe = (check: () => boolean) => { try { return check(); } catch { return false; } };
  return {
    run<T>(target: WorkbenchLibraryMutationTarget, action: () => Promise<T>, pendingLabel: string): Promise<T> {
      if (!context.capture()()) return Promise.reject(workbenchProjectContextChangedError());
      let bucket = buckets.find((entry) => safe(entry.isCurrent));
      if (!bucket) {
        try { bucket = { isCurrent: capture(), targets: new Set() }; }
        catch (error) { return Promise.reject(error); }
        if (!safe(bucket.isCurrent)) return Promise.reject(workbenchProjectContextChangedError());
      }
      const current = () => context.capture()() && safe(bucket.isCurrent);
      if ([...bucket.targets].some((entry) => conflicts(entry, target))) {
        return Promise.reject(new WorkbenchLibraryWritePendingError(pendingLabel, current));
      }
      if (!buckets.includes(bucket)) buckets.push(bucket);
      // Reserve synchronously before accepted work can change context or dispatch a request.
      const reservation = { ...target };
      bucket.targets.add(reservation);
      let operation: Promise<T>;
      try { operation = Promise.resolve(action()); } catch (error) { operation = Promise.reject(error); }
      return operation.finally(() => {
        bucket.targets.delete(reservation);
        if (!bucket.targets.size) buckets.splice(buckets.indexOf(bucket), 1);
      });
    },
  };
}

const owners = new WeakMap<WorkbenchProjectContext, ReturnType<typeof createWorkbenchLibraryMutations>>();
export function getWorkbenchLibraryMutations(context: WorkbenchProjectContext) {
  let owner = owners.get(context);
  if (!owner) { owner = createWorkbenchLibraryMutations(context); owners.set(context, owner); }
  return owner;
}

export function resolveWorkbenchLibraryMutationTarget(action: string, projectId: string | null, modelId: string | null): WorkbenchLibraryMutationTarget | null {
  if (action === "project/create") return { kind: "createProject" };
  if (!projectId) return null;
  if (action === "project/updateSelected" || action === "project/deleteSelected"
    || action === "model/saveAs" || (action === "model/save" && !modelId)) return { kind: "project", projectId };
  if (modelId && ["model/save", "model/deleteSelected", "model/renameSelectedVersion", "model/deleteSelectedVersion"].includes(action)) {
    return { kind: "model", projectId, modelId };
  }
  return null;
}

export function runWorkbenchLibraryTransition<T>(startTransition: (callback: () => void) => void, action: () => Promise<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    startTransition(async () => {
      try { resolve(await action()); } catch (error) { reject(error); }
    });
  });
}
