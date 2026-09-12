"use client";

import { useMemo, useRef, useState, type RefObject } from "react";
import type { WorkbenchCopy } from "../workbench-copy";
import type { WorkbenchModelBatchController } from "../workbench-model-batch-controller";
import { MODEL_BATCH_LIMITS, type ModelBatchOperation } from "@/lib/workbench/model-batch-commands";
import { inspectBatchSelection, ModelBatchError, parseBatchIndices, type BatchAxis, type BatchQuery } from "@/lib/workbench/model-batch-selection";
import { getModelBatchCopy } from "./workbench-model-batch-copy";
import { getModelTransformCopy } from "./workbench-model-transform-copy";
import { getModelSelectionCopy } from "./workbench-model-selection-copy";
import type { BatchPivot } from "@/lib/workbench/model-batch-geometry";
import { useModelBatchDraft, type ModelBatchDraftCache } from "./workbench-model-batch-draft";

type Props = { controller: WorkbenchModelBatchController; language: string; t: WorkbenchCopy;
  draftCache?: RefObject<ModelBatchDraftCache>; embedded?: boolean };

export function WorkbenchModelBatchCard(props: Props) {
  const [open, setOpen] = useState(false);
  const ownDraft = useRef<ModelBatchDraftCache>(null);
  const draftCache = props.draftCache ?? ownDraft;
  const c = getModelBatchCopy(props.language);
  if (props.embedded) return <BatchEditor {...props} draftCache={draftCache} />;
  return <section className="sidebar-card" data-model-batch="panel">
    <details open={open} onToggle={(event) => setOpen(event.currentTarget.open)}>
      <summary data-model-batch="toggle">{c.title}</summary>
      {open ? <BatchEditor {...props} draftCache={draftCache} /> : null}
    </details>
  </section>;
}

function BatchEditor({ controller, language, t, draftCache, embedded }: Props & { draftCache: RefObject<ModelBatchDraftCache> }) {
  const c = getModelBatchCopy(language);
  const g = getModelTransformCopy(language);
  const selectionCopy = getModelSelectionCopy(language);
  const { field, update } = useModelBatchDraft(draftCache, controller.studyKind, controller.spatial);
  const [queryKind, setQueryKind] = field("queryKind"), [indices, setIndices] = field("indices");
  const [queryAxis, setQueryAxis] = field("queryAxis"), [min, setMin] = field("min"), [max, setMax] = field("max");
  const [invert, setInvert] = field("invert"), [kind] = field("kind"), [pivotKind, setPivotKind] = field("pivotKind");
  const [point, setPoint] = field("point"), [angle, setAngle] = field("angle"), [factors, setFactors] = field("factors");
  const [createCopy, setCreateCopy] = field("createCopy"), [spacing, setSpacing] = field("spacing");
  const [snapAxes, setSnapAxes] = field("snapAxes"), [vector, setVector] = field("vector");
  const [axis, setAxis] = field("axis"), [coordinate, setCoordinate] = field("coordinate");
  const [copies, setCopies] = field("copies"), [copyConditions, setCopyConditions] = field("copyConditions");
  const [distribution, setDistribution] = field("distribution"), [supportAxes, setSupportAxes] = field("supportAxes");
  const [fixed, setFixed] = field("fixed"), [scope, setScope] = field("scope");
  const [area, setArea] = field("area"), [materialId, setMaterialId] = field("materialId");
  const [confirmation, setConfirmation] = useState<{ model: typeof controller.model; selection: number[] } | null>(null);
  const [message, setMessage] = useState("");
  const axes: BatchAxis[] = controller.spatial ? ["x", "y", "z"] : ["x", "y"];
  const isTransform = kind === "rotate" || kind === "scale" || kind === "mirror";
  const number = (value: string) => value.trim() === "" ? NaN : Number(value);
  const query = (): BatchQuery => queryKind === "indices"
    ? { kind: "indices", indices: parseBatchIndices(indices, controller.model?.nodes.length ?? 0), invert }
    : queryKind === "range" ? { kind: "range", axis: queryAxis, min: number(min), max: number(max), invert }
      : { kind: queryKind, invert };
  const currentSelection = queryKind === "current" ? controller.selection : null;
  const selectionDependency = currentSelection && currentSelection.length > 1 ? currentSelection : currentSelection?.[0] ?? null;
  const { preview, queryError } = useMemo(() => {
    try {
      if (!controller.model) throw new ModelBatchError("unsupported_study");
      return { preview: inspectBatchSelection(controller.model, query(), currentSelection ?? []), queryError: "" };
    } catch (error) { return { preview: null, queryError: errorCode(error) }; }
    // Geometry filtering is independent of operation parameters and confirmation changes.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [controller.model, selectionDependency, queryKind, indices, queryAxis, min, max, invert]);
  const confirmed = confirmation?.model === controller.model && (queryKind !== "current" ||
    (confirmation.selection.length === controller.selection.length && confirmation.selection.every((index, offset) => index === controller.selection[offset])));

  const operation = (): ModelBatchOperation => {
    const offset = { x: number(vector.x), y: number(vector.y), ...(controller.spatial ? { z: number(vector.z) } : {}) };
    const pivot: BatchPivot = pivotKind === "point" ? { kind: "point", point: {
      x: number(point.x), y: number(point.y), ...(controller.spatial ? { z: number(point.z) } : {}),
    } } : { kind: pivotKind };
    const transformOptions = { pivot, copy: createCopy, copyBoundaryConditions: createCopy && copyConditions };
    switch (kind) {
      case "rotate": return { kind, axis, angleDegrees: number(angle), ...transformOptions };
      case "scale": return { kind, factors: { x: number(factors.x), y: number(factors.y), ...(controller.spatial ? { z: number(factors.z) } : {}) }, ...transformOptions };
      case "mirror": return { kind, axis, ...transformOptions };
      case "snap": return { kind, axes: snapAxes, spacing: number(spacing) };
      case "translate": return { kind, offset };
      case "array": return { kind, offset, copies: number(copies), copyBoundaryConditions: copyConditions };
      case "align": return { kind, axis, value: number(coordinate) };
      case "loads": return { kind, loads: offset, distribution };
      case "supports": return { kind, axes: supportAxes, fixed };
      case "members": return { kind, scope, ...(area.trim() ? { area: number(area) } : {}), ...(materialId ? { materialId } : {}) };
      case "delete": return { kind, confirmDelete: confirmed };
    }
  };
  const resetFeedback = () => { setConfirmation(null); setMessage(""); };
  const select = (clear: boolean) => {
    try {
      const result = controller.select3d(clear ? { kind: "indices", indices: [] } : query());
      setConfirmation(null);
      update({ queryKind: "current", invert: false });
      setMessage(`${c.current}: ${result.selectedNodes}`);
    } catch (error) { setMessage(`${t.auditStatusOptions.failed}: ${errorCode(error)}`); }
  };
  const apply = () => {
    try {
      const summary = controller.apply({ query: query(), operation: operation() });
      setConfirmation(null);
      setMessage(summary.changed
        ? `${t.auditStatusOptions.completed}: ${t.nodes} ${summary.selectedNodes}; ${t.elements} ${summary.affectedMembers}; +${summary.addedNodes}/+${summary.addedMembers}; -${summary.removedNodes}/-${summary.removedMembers}`
        : c.unchanged);
    } catch (error) { setMessage(`${t.auditStatusOptions.failed}: ${errorCode(error)}`); }
  };
  const axisSelect = (value: BatchAxis, set: (axis: BatchAxis) => void, name: string, options = axes) =>
    <select aria-label={name} data-model-batch={name} value={value} onChange={(event) => set(event.target.value as BatchAxis)}>
      {options.map((key) => <option key={key} value={key}>{name === "transform-axis" && kind === "mirror"
        ? `${({ x: "YZ", y: "XZ", z: "XY" })[key]} (${key.toUpperCase()})` : key.toUpperCase()}</option>)}
    </select>;

  return <div className={`sidebar-stack${embedded ? " batch-editor--dock" : ""}`} data-model-batch="editor">
    <details className="batch-help"><summary>{t.summary} · SI</summary><p className="card-copy">{c.hint}</p></details>
    <div className="form-grid compact" data-model-batch="fields" onChange={resetFeedback}>
      <label><span>{t.nodes}</span>
        <select data-model-batch="query" value={queryKind} onChange={(event) => setQueryKind(event.target.value as BatchQuery["kind"])}>
          <option value="current">{c.current}</option><option value="all">{t.workflowCatalogFilterAllLabel}</option>
          <option value="indices">{c.indices}</option><option value="range">{c.range}</option>
        </select>
      </label>
      {queryKind === "indices" ? <label><span>{c.indices}</span>
        <input data-model-batch="indices" value={indices} placeholder="0-9, 15, 20-30" onChange={(event) => setIndices(event.target.value)} />
      </label> : null}
      {queryKind === "range" ? <>
        <label><span>{c.range} (m)</span>{axisSelect(queryAxis, setQueryAxis, "query-axis")}</label>
        <label className="batch-scalar"><span>{queryAxis.toUpperCase()} ≥ (m)</span><input type="number" step="any" data-model-batch="min" value={min} onChange={(event) => setMin(event.target.value)} /></label>
        <label className="batch-scalar"><span>{queryAxis.toUpperCase()} ≤ (m)</span><input type="number" step="any" data-model-batch="max" value={max} onChange={(event) => setMax(event.target.value)} /></label>
      </> : null}
      <label className="checkbox-row"><input type="checkbox" checked={invert} data-model-batch="invert" onChange={(event) => setInvert(event.target.checked)} />{c.invert}</label>
      <label><span>{t.actions}</span>
        <select data-model-batch="operation" value={kind} onChange={(event) => {
          update({ kind: event.target.value as typeof kind, vector: { x: "0", y: "0", z: "0" },
            axis: event.target.value === "rotate" ? "z" : "x", createCopy: false, copyConditions: false });
        }}>
          <option value="translate">{c.translate}</option><option value="align">{c.align}</option><option value="array">{c.array}</option>
          <option value="rotate">{g.rotate}</option><option value="scale">{g.scale}</option>
          <option value="mirror">{g.mirror}</option><option value="snap">{g.snap}</option>
          <option value="loads">{t.immersiveLoads}</option><option value="supports">{c.supports}</option>
          <option value="members">{t.elements} / {t.properties}</option><option value="delete">{t.deleteNode}</option>
        </select>
      </label>
      {kind === "translate" || kind === "array" || kind === "loads" ? axes.map((key) => <label className="batch-scalar" key={key}>
        <span>{kind === "loads" ? `${key.toUpperCase()} (N)` : `Δ${key.toUpperCase()} (m)`}</span>
        <input type="number" step="any" data-model-batch={`vector-${key}`} value={vector[key]} onChange={(event) => setVector({ ...vector, [key]: event.target.value })} />
      </label>) : null}
      {kind === "align" ? <>
        <label><span>{c.align}</span>{axisSelect(axis, setAxis, "align-axis")}</label>
        <label><span>{axis.toUpperCase()} (m)</span><input type="number" step="any" data-model-batch="coordinate" value={coordinate} onChange={(event) => setCoordinate(event.target.value)} /></label>
      </> : null}
      {isTransform ? <>
        <label><span>{g.pivot}</span><select data-model-batch="pivot" value={pivotKind} onChange={(event) => setPivotKind(event.target.value as typeof pivotKind)}>
          <option value="selection">{g.center}</option><option value="origin">{g.origin}</option><option value="point">{g.point}</option>
        </select></label>
        {pivotKind === "point" ? <div className="batch-vector" style={{ gridTemplateColumns: `repeat(${axes.length}, minmax(0, 1fr))` }}>
          {axes.map((key) => <label key={key}>
            <span>{key.toUpperCase()} (m)</span><input type="number" step="any" data-model-batch={`pivot-${key}`} value={point[key]} onChange={(event) => setPoint({ ...point, [key]: event.target.value })} />
          </label>)}
        </div> : null}
        {kind === "rotate" || kind === "mirror" ? <label className={kind === "rotate" ? "batch-scalar" : undefined}><span>{kind === "rotate" ? g.rotate : g.mirror}</span>
          {axisSelect(axis, setAxis, "transform-axis", kind === "rotate" && !controller.spatial ? ["z"] : axes)}
        </label> : null}
        {kind === "rotate" ? <label className="batch-scalar"><span>{g.angle} (°)</span><input type="number" step="any" data-model-batch="angle" value={angle} onChange={(event) => setAngle(event.target.value)} /></label> : null}
        {kind === "scale" ? <div className="batch-vector" style={{ gridTemplateColumns: `repeat(${axes.length}, minmax(0, 1fr))` }}>
          {axes.map((key) => <label key={key}>
            <span>{key.toUpperCase()} ×</span><input type="number" step="any" min={0} data-model-batch={`factor-${key}`} value={factors[key]} onChange={(event) => setFactors({ ...factors, [key]: event.target.value })} />
          </label>)}
        </div> : null}
        <label className="checkbox-row"><input type="checkbox" data-model-batch="create-copy" checked={createCopy} onChange={(event) => setCreateCopy(event.target.checked)} />{g.copy}</label>
        {createCopy ? <label className="checkbox-row"><input type="checkbox" data-model-batch="copy-conditions" checked={copyConditions} onChange={(event) => setCopyConditions(event.target.checked)} />{c.copyConditions}</label> : null}
      </> : null}
      {kind === "snap" ? <>
        <label><span>{g.spacing} (m)</span><input type="number" step="any" min={0} data-model-batch="spacing" value={spacing} onChange={(event) => setSpacing(event.target.value)} /></label>
        {axes.map((key) => <label className="checkbox-row" key={key}>
          <input type="checkbox" data-model-batch={`snap-${key}`} checked={snapAxes.includes(key)} onChange={(event) => setSnapAxes(event.target.checked ? [...snapAxes, key] : snapAxes.filter((item) => item !== key))} />{key.toUpperCase()}
        </label>)}
      </> : null}
      {isTransform || kind === "snap" ? <p className="card-copy">{g.hint}</p> : null}
      {kind === "array" ? <>
        <label className="batch-scalar"><span title={`1-${MODEL_BATCH_LIMITS.copies}`}>{c.copies}</span><input type="number" min={1} max={MODEL_BATCH_LIMITS.copies} data-model-batch="copies" value={copies} onChange={(event) => setCopies(event.target.value)} /></label>
        <label className="checkbox-row"><input type="checkbox" data-model-batch="copy-conditions" checked={copyConditions} onChange={(event) => setCopyConditions(event.target.checked)} />{c.copyConditions}</label>
        <p className="card-copy">+{t.nodes} ≤ {MODEL_BATCH_LIMITS.addedNodes}; +{t.elements} ≤ {MODEL_BATCH_LIMITS.addedMembers}</p>
      </> : null}
      {kind === "loads" ? <label><span>{t.immersiveLoads}</span><select data-model-batch="distribution" value={distribution} onChange={(event) => setDistribution(event.target.value as typeof distribution)}>
        <option value="total">{c.total}</option><option value="per_node">{c.perNode}</option>
      </select></label> : null}
      {kind === "supports" ? <>
        {[...axes, ...(controller.frame ? ["rz" as const] : [])].map((key) => <label className="checkbox-row" key={key}>
          <input type="checkbox" data-model-batch={`support-${key}`} checked={supportAxes.includes(key)} onChange={(event) => setSupportAxes(event.target.checked ? [...supportAxes, key] : supportAxes.filter((item) => item !== key))} />{key.toUpperCase()}
        </label>)}
        <label><span>{c.supports}</span><select data-model-batch="support-mode" value={fixed ? "fix" : "release"} onChange={(event) => setFixed(event.target.value === "fix")}>
          <option value="fix">{t.yes}</option><option value="release">{t.no}</option>
        </select></label>
      </> : null}
      {kind === "members" ? <>
        <label><span>{t.memberSelection}</span><select data-model-batch="member-scope" value={scope} onChange={(event) => setScope(event.target.value as typeof scope)}>
          <option value="internal">{c.internal}</option><option value="touching">{c.touching}</option>
        </select></label>
        <label><span>{t.area}</span><input type="number" step="any" min={0} placeholder={c.unchanged} data-model-batch="area" value={area} onChange={(event) => setArea(event.target.value)} /></label>
        <label><span>{t.material}</span><select data-model-batch="material" value={materialId} onChange={(event) => setMaterialId(event.target.value)}>
          <option value="">{c.unchanged}</option>{controller.model?.materials?.map((material) => <option key={material.id} value={material.id}>{material.name}</option>)}
        </select></label>
      </> : null}
    </div>
    <div className="batch-editor__footer">
    <output data-model-batch="preview" aria-live="polite" className="card-copy">
      {preview ? `${t.nodes}: ${preview.nodes}; ${c.internal}: ${preview.internalMembers}; ${c.touching}: ${preview.touchingMembers}` : `${t.auditStatusOptions.failed}: ${queryError}`}
      {kind === "array" && preview && Number.isInteger(number(copies)) ? `; +${t.nodes}: ${preview.nodes * number(copies)}; +${t.elements}: ${preview.internalMembers * number(copies)}` : ""}
      {isTransform && createCopy && preview ? `; +${t.nodes}: ${preview.nodes}; +${t.elements}: ${preview.internalMembers}` : ""}
    </output>
    {kind === "delete" ? <label className="checkbox-row"><input type="checkbox" data-model-batch="confirm-delete" checked={confirmed} onChange={(event) => setConfirmation(event.target.checked ? { model: controller.model, selection: [...controller.selection] } : null)} />{c.confirmDelete}</label> : null}
    <div className="batch-editor__actions">
    {controller.spatial ? <>
      <button className="ghost-button" data-model-batch="select" type="button" disabled={!preview} onClick={() => select(false)}>{selectionCopy.select}</button>
      <button className="ghost-button" data-model-batch="clear-selection" type="button" onClick={() => select(true)}>{selectionCopy.clear}</button>
    </> : null}
    <button className="ghost-button" data-model-batch="apply" type="button" disabled={!preview?.nodes || (kind === "delete" && !confirmed)} onClick={apply}>{c.apply}</button>
    </div>
    {message ? <p role="status" data-model-batch="status" className="card-copy">{message}</p> : null}
    </div>
  </div>;
}

function errorCode(error: unknown) { return error instanceof ModelBatchError ? error.code : "invalid_request"; }
