"use client";

import { useState, type RefObject } from "react";
import type { BatchAxis, BatchQuery } from "@/lib/workbench/model-batch-selection";
import type { ModelBatchOperation } from "@/lib/workbench/model-batch-commands";
import type { BatchPivot } from "@/lib/workbench/model-batch-geometry";

export function createModelBatchDraft(spatial: boolean) {
  return {
    queryKind: "current" as BatchQuery["kind"], indices: "", queryAxis: "x" as BatchAxis,
    min: "0", max: "0", invert: false, kind: "translate" as ModelBatchOperation["kind"],
    pivotKind: "selection" as BatchPivot["kind"], point: { x: "0", y: "0", z: "0" },
    angle: "90", factors: { x: "1", y: "1", z: "1" }, createCopy: false,
    spacing: "1", snapAxes: spatial ? ["x", "y", "z"] as BatchAxis[] : ["x", "y"] as BatchAxis[],
    vector: { x: "0", y: "0", z: "0" }, axis: "x" as BatchAxis, coordinate: "0",
    copies: "1", copyConditions: false, distribution: "total" as "per_node" | "total",
    supportAxes: ["x", "y"] as Array<BatchAxis | "rz">, fixed: true,
    scope: "internal" as "internal" | "touching", area: "", materialId: "",
  };
}
type Draft = ReturnType<typeof createModelBatchDraft>;
export type ModelBatchDraftCache = { studyKind: string; draft: Draft } | null;

export function readModelBatchDraft(cache: ModelBatchDraftCache, studyKind: string, spatial: boolean): Draft {
  return cache?.studyKind === studyKind ? cache.draft : createModelBatchDraft(spatial);
}

export function useModelBatchDraft(cache: RefObject<ModelBatchDraftCache>, studyKind: string, spatial: boolean) {
  const [draft, setDraft] = useState(() => readModelBatchDraft(cache.current, studyKind, spatial));
  const update = (patch: Partial<Draft>) => {
    const next = { ...readModelBatchDraft(cache.current, studyKind, spatial), ...patch };
    // Only small form parameters survive remounts; model data and deletion consent never enter this cache.
    cache.current = { studyKind, draft: next };
    setDraft(next);
  };
  const field = <K extends keyof Draft>(key: K): [Draft[K], (value: Draft[K]) => void] =>
    [draft[key], (value) => update({ [key]: value })];
  return { field, update };
}
