import { directMeshMethodForStudyKind, normalizeDirectMeshStudyInput, type DirectMeshStudyKind } from "./direct-mesh-study-contract.ts";
export type DurableTaskKind = DirectMeshStudyKind | "operator_task";
export function durableTaskMethod(kind: DurableTaskKind) {
  return kind === "operator_task" ? "run_operator_task_ir" : directMeshMethodForStudyKind(kind);
}
export function normalizeDurableTaskInput(kind: DurableTaskKind, input: Record<string, unknown>) {
  if (kind !== "operator_task") return normalizeDirectMeshStudyInput(kind, input);
  const task = input?.task_ir as Record<string, unknown> | undefined;
  if (!input || Object.keys(input).length !== 2 || input.mode !== "execute" || !task || Array.isArray(task)
    || task.schema_version !== "kyuubiki.operator-task-ir/v1" || JSON.stringify(input).length > 131072) {
    throw new Error("Invalid prepared operator task. Prepare it from the enabled project package first.");
  }
  return input;
}
