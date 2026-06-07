"use client";

import type {
  WorkflowDatasetAxis,
  WorkflowDatasetContract,
  WorkflowDatasetValueInfo,
  WorkflowGraphEdge,
  WorkflowGraphNode,
} from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";

type WorkbenchWorkflowDatasetCardProps = {
  labels: WorkflowSidebarLabels;
  selectedDatasetContract: WorkflowDatasetContract | null;
  selectedDatasetValues: WorkflowDatasetValueInfo[];
  selectedDatasetValue: WorkflowDatasetValueInfo | null;
  selectedNodes: WorkflowGraphNode[];
  selectedEdges: WorkflowGraphEdge[];
  selectedDatasetValueId: string | null;
  focusedDatasetValueId?: string | null;
  highlightDatasetEditor?: boolean;
  setSelectedDatasetValueId: (value: string | null) => void;
  updateDatasetValue: (
    valueId: string,
    updater: (value: WorkflowDatasetValueInfo) => WorkflowDatasetValueInfo,
  ) => void;
  updateNodePortDatasetValue: (
    nodeId: string,
    portId: string,
    direction: "inputs" | "outputs",
    datasetValue: string,
  ) => void;
  updateEdgeDatasetValue: (edgeId: string, datasetValue: string) => void;
  addDatasetValue: () => void;
  removeSelectedDatasetValue: () => void;
  addDatasetAxis: () => void;
  removeDatasetAxis: (axisId: string) => void;
  updateDatasetAxis: (
    axisId: string,
    updater: (axis: WorkflowDatasetAxis) => WorkflowDatasetAxis,
  ) => void;
};

export function WorkbenchWorkflowDatasetCard({
  labels,
  selectedDatasetContract,
  selectedDatasetValues,
  selectedDatasetValue,
  selectedNodes,
  selectedEdges,
  selectedDatasetValueId,
  focusedDatasetValueId,
  highlightDatasetEditor,
  setSelectedDatasetValueId,
  updateDatasetValue,
  updateNodePortDatasetValue,
  updateEdgeDatasetValue,
  addDatasetValue,
  removeSelectedDatasetValue,
  addDatasetAxis,
  removeDatasetAxis,
  updateDatasetAxis,
}: WorkbenchWorkflowDatasetCardProps) {
  return (
    <>
      <section
        className="sidebar-card sidebar-card--compact"
        data-workflow-dataset-editor="summary"
        style={
          highlightDatasetEditor
            ? { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" }
            : undefined
        }
      >
        <div className="card-head">
          <h2>{labels.datasetContractTitle}</h2>
          <span className={`status-pill status-pill--${selectedDatasetContract ? "good" : "watch"}`}>
            {selectedDatasetContract?.version ?? "--"}
          </span>
        </div>
        {selectedDatasetContract ? (
          <>
            <p className="card-copy">{selectedDatasetContract.name ?? selectedDatasetContract.id}</p>
            <p className="card-copy">{labels.datasetDraftHint}</p>
            <div className="sidebar-list">
              <div className="sidebar-list__row">
                <span>{labels.datasetValuesTitle}</span>
                <strong>{selectedDatasetValues.length}</strong>
              </div>
              <div className="sidebar-list__row">
                <span>{labels.datasetMetadataLabel}</span>
                <strong>{selectedDatasetContract.id}</strong>
              </div>
            </div>
            <div className="sidebar-stack">
              {selectedDatasetValues.map((value) => {
                const axes = value.shape?.axes ?? [];
                const schemaLabel = value.schema_ref
                  ? `${value.schema_ref.schema}@${value.schema_ref.version}`
                  : "--";
                const shapeLabel =
                  axes.length > 0
                    ? axes.map((axis) => (axis.size != null ? `${axis.id}[${axis.size}]` : axis.id)).join(" × ")
                    : "--";
                return (
                  <section
                    className="sidebar-card sidebar-card--compact"
                    data-workflow-dataset-value-id={value.id}
                    key={value.id}
                    style={
                      focusedDatasetValueId === value.id
                        ? { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" }
                        : undefined
                    }
                  >
                    <div className="card-head">
                      <h2>{value.id}</h2>
                      <span className="status-pill status-pill--watch">{value.data_class}</span>
                    </div>
                    <div className="sidebar-list">
                      <div className="sidebar-list__row">
                        <span>{labels.datasetSemanticTypeLabel}</span>
                        <strong>{value.semantic_type ?? "--"}</strong>
                      </div>
                      <div className="sidebar-list__row">
                        <span>{labels.datasetEncodingLabel}</span>
                        <strong>{value.encoding ?? "--"}</strong>
                      </div>
                      <div className="sidebar-list__row">
                        <span>{labels.datasetClassLabel}</span>
                        <strong>{value.element_type}</strong>
                      </div>
                      <div className="sidebar-list__row">
                        <span>{labels.datasetShapeLabel}</span>
                        <strong>{shapeLabel}</strong>
                      </div>
                      <div className="sidebar-list__row">
                        <span>{labels.datasetSchemaLabel}</span>
                        <strong>{schemaLabel}</strong>
                      </div>
                    </div>
                    {axes.length > 0 ? (
                      <div className="sidebar-list">
                        {axes.map((axis) => (
                          <div className="sidebar-list__row" key={`${value.id}:${axis.id}`}>
                            <span>{labels.datasetAxesLabel}</span>
                            <strong>
                              {axis.id}
                              {axis.semantic ? ` · ${axis.semantic}` : ""}
                              {axis.size != null ? ` · ${axis.size}` : ""}
                            </strong>
                          </div>
                        ))}
                      </div>
                    ) : null}
                  </section>
                );
              })}
            </div>
          </>
        ) : (
          <p className="card-copy">{labels.datasetNoneLabel}</p>
        )}
      </section>

      <section
        className="sidebar-card sidebar-card--compact"
        data-workflow-dataset-editor="editor"
        style={
          highlightDatasetEditor
            ? { outline: "2px solid var(--accent, #4f46e5)", outlineOffset: "2px" }
            : undefined
        }
      >
        <div className="card-head">
          <h2>{labels.datasetEditorTitle}</h2>
          <span className="status-pill status-pill--watch">{labels.datasetDraftLocalLabel}</span>
        </div>
        <div className="button-row">
          <button onClick={addDatasetValue} type="button">
            {labels.datasetAddValueLabel}
          </button>
          <button disabled={!selectedDatasetValue} onClick={removeSelectedDatasetValue} type="button">
            {labels.datasetRemoveValueLabel}
          </button>
        </div>
        {selectedDatasetValue ? (
          <>
            <div className="form-grid compact">
              <label>
                <span>{labels.datasetValueSelectLabel}</span>
                <select
                  onChange={(event) => setSelectedDatasetValueId(event.target.value)}
                  value={selectedDatasetValueId ?? selectedDatasetValue.id}
                >
                  {selectedDatasetValues.map((value) => (
                    <option key={value.id} value={value.id}>
                      {value.id}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                <span>{labels.datasetSemanticTypeLabel}</span>
                <input
                  onChange={(event) =>
                    updateDatasetValue(selectedDatasetValue.id, (value) => ({
                      ...value,
                      semantic_type: event.target.value || undefined,
                    }))
                  }
                  value={selectedDatasetValue.semantic_type ?? ""}
                />
              </label>
              <label>
                <span>{labels.datasetEncodingLabel}</span>
                <input
                  onChange={(event) =>
                    updateDatasetValue(selectedDatasetValue.id, (value) => ({
                      ...value,
                      encoding: event.target.value || undefined,
                    }))
                  }
                  value={selectedDatasetValue.encoding ?? ""}
                />
              </label>
              <label>
                <span>{labels.datasetUnitLabel}</span>
                <input
                  onChange={(event) =>
                    updateDatasetValue(selectedDatasetValue.id, (value) => ({
                      ...value,
                      unit: event.target.value || undefined,
                    }))
                  }
                  value={selectedDatasetValue.unit ?? ""}
                />
              </label>
              <label>
                <span>{labels.datasetSchemaLabel}</span>
                <input
                  onChange={(event) =>
                    updateDatasetValue(selectedDatasetValue.id, (value) => ({
                      ...value,
                      schema_ref: {
                        schema: event.target.value,
                        version: value.schema_ref?.version ?? "1",
                      },
                    }))
                  }
                  value={selectedDatasetValue.schema_ref?.schema ?? ""}
                />
              </label>
              <label>
                <span>{labels.datasetMetadataLabel}</span>
                <input
                  onChange={(event) =>
                    updateDatasetValue(selectedDatasetValue.id, (value) => ({
                      ...value,
                      schema_ref: value.schema_ref
                        ? { ...value.schema_ref, version: event.target.value || "1" }
                        : { schema: "", version: event.target.value || "1" },
                    }))
                  }
                  value={selectedDatasetValue.schema_ref?.version ?? ""}
                />
              </label>
            </div>
            <section className="sidebar-card sidebar-card--compact">
              <div className="card-head">
                <h2>{labels.datasetAxesEditorTitle}</h2>
              </div>
              <div className="button-row">
                <button onClick={addDatasetAxis} type="button">
                  {labels.datasetAddAxisLabel}
                </button>
              </div>
              <div className="sidebar-stack">
                {(selectedDatasetValue.shape?.axes ?? []).map((axis) => (
                  <section className="sidebar-card sidebar-card--compact" key={`${selectedDatasetValue.id}:${axis.id}`}>
                    <div className="button-row">
                      <button onClick={() => removeDatasetAxis(axis.id)} type="button">
                        {labels.datasetRemoveAxisLabel}
                      </button>
                    </div>
                    <div className="form-grid compact">
                      <label>
                        <span>{labels.datasetAxisIdLabel}</span>
                        <input
                          onChange={(event) =>
                            updateDatasetAxis(axis.id, (current) => ({
                              ...current,
                              id: event.target.value || current.id,
                            }))
                          }
                          value={axis.id}
                        />
                      </label>
                      <label>
                        <span>{labels.datasetAxisSizeLabel}</span>
                        <input
                          onChange={(event) =>
                            updateDatasetAxis(axis.id, (current) => ({
                              ...current,
                              size: event.target.value ? Number(event.target.value) : undefined,
                            }))
                          }
                          value={axis.size ?? ""}
                        />
                      </label>
                      <label>
                        <span>{labels.datasetAxisSemanticLabel}</span>
                        <input
                          onChange={(event) =>
                            updateDatasetAxis(axis.id, (current) => ({
                              ...current,
                              semantic: event.target.value || undefined,
                            }))
                          }
                          value={axis.semantic ?? ""}
                        />
                      </label>
                    </div>
                  </section>
                ))}
              </div>
            </section>
          </>
        ) : (
          <p className="card-copy">{labels.datasetNoneLabel}</p>
        )}
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.datasetPortMappingsTitle}</h2>
        </div>
        <div className="sidebar-stack">
          {selectedNodes.map((node) => (
            <section className="sidebar-card sidebar-card--compact" key={`map:${node.id}`}>
              <div className="card-head">
                <h2>{node.id}</h2>
                <span className="status-pill status-pill--watch">{node.kind}</span>
              </div>
              <div className="form-grid compact">
                {[...(node.inputs ?? []), ...(node.outputs ?? [])].map((port) => {
                  const direction = (node.inputs ?? []).some((entry) => entry.id === port.id) ? "inputs" : "outputs";
                  return (
                    <label key={`${node.id}:${direction}:${port.id}`}>
                      <span>
                        {direction === "inputs" ? "in" : "out"} · {port.id} · {port.artifact_type}
                      </span>
                      <select
                        onChange={(event) => updateNodePortDatasetValue(node.id, port.id, direction, event.target.value)}
                        value={port.dataset_value ?? ""}
                      >
                        <option value="">{labels.datasetUnassignedLabel}</option>
                        {selectedDatasetValues.map((value) => (
                          <option key={value.id} value={value.id}>
                            {value.id}
                          </option>
                        ))}
                      </select>
                    </label>
                  );
                })}
              </div>
            </section>
          ))}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.datasetEdgeMappingsTitle}</h2>
        </div>
        <div className="form-grid compact">
          {selectedEdges.map((edge) => (
            <label key={`edge-map:${edge.id}`}>
              <span>
                {edge.from.node}.{edge.from.port} → {edge.to.node}.{edge.to.port}
              </span>
              <select
                onChange={(event) => updateEdgeDatasetValue(edge.id, event.target.value)}
                value={edge.dataset_value ?? ""}
              >
                <option value="">{labels.datasetUnassignedLabel}</option>
                {selectedDatasetValues.map((value) => (
                  <option key={value.id} value={value.id}>
                    {value.id}
                  </option>
                ))}
              </select>
            </label>
          ))}
        </div>
      </section>
    </>
  );
}
