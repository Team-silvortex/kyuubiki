export type WorkbenchAuthorityDimension =
  | "mode" | "backend" | "controlPlaneApiToken" | "clusterApiToken" | "directMeshApiToken";

// Fixed-size, process-local identities: no credentials, history, listeners or disk writes.
const revisions: Record<WorkbenchAuthorityDimension, symbol> = {
  mode: Symbol(), backend: Symbol(), controlPlaneApiToken: Symbol(),
  clusterApiToken: Symbol(), directMeshApiToken: Symbol(),
};

export function readWorkbenchAuthorityRevision(dimension: WorkbenchAuthorityDimension): symbol {
  return revisions[dimension];
}

export function advanceWorkbenchAuthorityRevision(dimension: WorkbenchAuthorityDimension): void {
  revisions[dimension] = Symbol();
}
