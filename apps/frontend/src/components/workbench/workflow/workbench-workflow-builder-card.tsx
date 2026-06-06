"use client";
import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type {
  WorkflowCatalogEntry,
  WorkflowDatasetContract,
  WorkflowDatasetValueInfo,
  WorkflowGraphDefinition,
} from "@/lib/api";
import type { WorkflowSidebarLabels } from "@/components/workbench/workflow/workbench-workflow-types";
import {
  asWorkflowDatasetContract,
  asWorkflowGraphDefinition,
  mergeDatasetContractIntoGraph,
  readJsonFile,
} from "@/components/workbench/workflow/workbench-workflow-builder-import";

type WorkbenchWorkflowBuilderCardProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry | null;
  onRunWorkflowCatalog: (workflowId: string) => void;
};

function cloneWorkflowGraph(graph: WorkflowGraphDefinition | null): WorkflowGraphDefinition | null {
  if (!graph) return null;
  return JSON.parse(JSON.stringify(graph)) as WorkflowGraphDefinition;
}

function ensureDatasetContract(
  graph: WorkflowGraphDefinition | null,
): WorkflowDatasetContract | null {
  if (!graph) return null;
  if (!graph.dataset_contract) {
    graph.dataset_contract = {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: `${graph.id}.dataset`,
      version: graph.version ?? "1.0.0",
      name: `${graph.name ?? graph.id} dataset contract`,
      values: [],
      metadata: {},
    };
  }
  return graph.dataset_contract;
}

function slugifyWorkflowAssetName(value: string): string {
  return (
    value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "") || "workflow"
  );
}

function downloadJsonArtifact(filename: string, payload: unknown) {
  const blob = new Blob([JSON.stringify(payload, null, 2)], {
    type: "application/json",
  });
  const href = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = href;
  anchor.download = filename;
  anchor.click();
  URL.revokeObjectURL(href);
}

export function WorkbenchWorkflowBuilderCard({
  labels,
  selectedWorkflow,
  onRunWorkflowCatalog,
}: WorkbenchWorkflowBuilderCardProps) {
  const [draftGraph, setDraftGraph] = useState<WorkflowGraphDefinition | null>(null);
  const [selectedDatasetValueId, setSelectedDatasetValueId] = useState<string | null>(null);
  const [importMessage, setImportMessage] = useState<string | null>(null);
  const graphInputRef = useRef<HTMLInputElement | null>(null);
  const datasetInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    const nextDraft = cloneWorkflowGraph(selectedWorkflow?.graph ?? null);
    setDraftGraph(nextDraft);
    setSelectedDatasetValueId(nextDraft?.dataset_contract?.values?.[0]?.id ?? null);
    setImportMessage(null);
  }, [selectedWorkflow]);

  const selectedGraph = draftGraph;
  const selectedNodes = selectedGraph?.nodes ?? [];
  const selectedEdges = selectedGraph?.edges ?? [];
  const selectedDatasetContract = selectedGraph?.dataset_contract ?? null;
  const selectedDatasetValues = selectedDatasetContract?.values ?? [];
  const selectedDatasetValue = useMemo(
    () =>
      selectedDatasetValues.find((value) => value.id === selectedDatasetValueId) ??
      selectedDatasetValues[0] ??
      null,
    [selectedDatasetValueId, selectedDatasetValues],
  );

  function updateDatasetValue(
    valueId: string,
    updater: (value: WorkflowDatasetValueInfo) => WorkflowDatasetValueInfo,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      const contract = ensureDatasetContract(next);
      if (!contract) return current;
      contract.values = contract.values.map((value) =>
        value.id === valueId ? updater(value) : value,
      );
      return next;
    });
  }

  function updateNodePortDatasetValue(
    nodeId: string,
    portId: string,
    direction: "inputs" | "outputs",
    datasetValue: string,
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      const node = next?.nodes.find((entry) => entry.id === nodeId);
      const ports = node?.[direction];
      const port = ports?.find((entry) => entry.id === portId);
      if (port) {
        port.dataset_value = datasetValue || undefined;
      }
      return next;
    });
  }

  function updateEdgeDatasetValue(edgeId: string, datasetValue: string) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      const edge = next?.edges?.find((entry) => entry.id === edgeId);
      if (edge) {
        edge.dataset_value = datasetValue || undefined;
      }
      return next;
    });
  }

  function exportDraftWorkflowGraph() {
    if (!selectedGraph) return;
    downloadJsonArtifact(
      `${slugifyWorkflowAssetName(selectedGraph.id)}.workflow-graph.json`,
      selectedGraph,
    );
  }

  function exportDraftDatasetContract() {
    if (!selectedDatasetContract) return;
    downloadJsonArtifact(
      `${slugifyWorkflowAssetName(selectedDatasetContract.id)}.workflow-dataset.json`,
      selectedDatasetContract,
    );
  }

  async function importWorkflowGraphFile(file: File) {
    try {
      const json = await readJsonFile(file);
      const graph = asWorkflowGraphDefinition(json);
      if (!graph) {
        setImportMessage(labels.importInvalidGraphLabel);
        return;
      }
      const nextGraph = cloneWorkflowGraph(graph);
      setDraftGraph(nextGraph);
      setSelectedDatasetValueId(nextGraph?.dataset_contract?.values?.[0]?.id ?? null);
      setImportMessage(labels.importSuccessLabel);
    } catch {
      setImportMessage(labels.importInvalidGraphLabel);
    }
  }

  async function importDatasetContractFile(file: File) {
    try {
      const json = await readJsonFile(file);
      const contract = asWorkflowDatasetContract(json);
      if (!contract) {
        setImportMessage(labels.importInvalidDatasetLabel);
        return;
      }
      setDraftGraph((current) => mergeDatasetContractIntoGraph(cloneWorkflowGraph(current), contract));
      setSelectedDatasetValueId(contract.values[0]?.id ?? null);
      setImportMessage(labels.importSuccessLabel);
    } catch {
      setImportMessage(labels.importInvalidDatasetLabel);
    }
  }

  function handleGraphFileChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) void importWorkflowGraphFile(file);
    event.target.value = "";
  }

  function handleDatasetFileChange(event: ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0];
    if (file) void importDatasetContractFile(file);
    event.target.value = "";
  }

  if (!selectedWorkflow) {
    return (
      <section className="sidebar-card sidebar-card--compact">
        <p className="card-copy">{labels.noSelectionLabel}</p>
      </section>
    );
  }

  return (
    <section className="sidebar-card sidebar-card--compact">
      <div className="card-head">
        <h2>{selectedWorkflow.name}</h2>
        <span className="status-pill status-pill--good">{selectedWorkflow.version}</span>
      </div>
      <p className="card-copy">{selectedWorkflow.summary}</p>
      <div className="button-row">
        <button onClick={() => onRunWorkflowCatalog(selectedWorkflow.id)} type="button">
          {labels.runLabel}
        </button>
        <button onClick={() => graphInputRef.current?.click()} type="button">
          {labels.importGraphLabel}
        </button>
        <button onClick={() => datasetInputRef.current?.click()} type="button">
          {labels.importDatasetContractLabel}
        </button>
        <button onClick={exportDraftWorkflowGraph} type="button">
          {labels.exportGraphLabel}
        </button>
        <button
          disabled={!selectedDatasetContract}
          onClick={exportDraftDatasetContract}
          type="button"
        >
          {labels.exportDatasetContractLabel}
        </button>
      </div>
      <input
        accept="application/json,.json"
        hidden
        onChange={handleGraphFileChange}
        ref={graphInputRef}
        type="file"
      />
      <input
        accept="application/json,.json"
        hidden
        onChange={handleDatasetFileChange}
        ref={datasetInputRef}
        type="file"
      />
      {importMessage ? <p className="card-copy">{importMessage}</p> : null}
      <div className="sidebar-list">
        <div className="sidebar-list__row">
          <span>{labels.nodesTitle}</span>
          <strong>{selectedNodes.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.edgesTitle}</span>
          <strong>{selectedEdges.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.entryInputsTitle}</span>
          <strong>{selectedWorkflow.entry_inputs.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.outputArtifactsTitle}</span>
          <strong>{selectedWorkflow.output_artifacts.length}</strong>
        </div>
      </div>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.nodesTitle}</h2>
        </div>
        <div className="sidebar-list">
          {selectedNodes.map((node) => (
            <div className="sidebar-list__row" key={node.id}>
              <span>{node.id}</span>
              <strong>
                {labels.kindLabel}: {node.kind}
                {node.operator_id ? ` · ${labels.operatorLabel}: ${node.operator_id}` : ""}
                {node.outputs?.some((port) => port.dataset_value)
                  ? ` · ${labels.datasetValueLabel}: ${node.outputs
                      .map((port) => port.dataset_value)
                      .filter(Boolean)
                      .join(", ")}`
                  : ""}
              </strong>
            </div>
          ))}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.edgesTitle}</h2>
        </div>
        <div className="sidebar-list">
          {selectedEdges.map((edge) => (
            <div className="sidebar-list__row" key={edge.id}>
              <span>
                {edge.from.node}.{edge.from.port} → {edge.to.node}.{edge.to.port}
              </span>
              <strong>
                {edge.artifact_type}
                {edge.dataset_value ? ` · ${labels.datasetValueLabel}: ${edge.dataset_value}` : ""}
              </strong>
            </div>
          ))}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
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
                    ? axes
                        .map((axis) => (axis.size != null ? `${axis.id}[${axis.size}]` : axis.id))
                        .join(" × ")
                    : "--";
                return (
                  <section className="sidebar-card sidebar-card--compact" key={value.id}>
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

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.datasetEditorTitle}</h2>
          <span className="status-pill status-pill--watch">{labels.datasetDraftLocalLabel}</span>
        </div>
        {selectedDatasetValue ? (
          <div className="form-grid compact">
            <label>
              <span>{labels.datasetValueSelectLabel}</span>
              <select
                onChange={(event) => setSelectedDatasetValueId(event.target.value)}
                value={selectedDatasetValue.id}
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
                      ? {
                          ...value.schema_ref,
                          version: event.target.value || "1",
                        }
                      : {
                          schema: "",
                          version: event.target.value || "1",
                        },
                  }))
                }
                value={selectedDatasetValue.schema_ref?.version ?? ""}
              />
            </label>
          </div>
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
                  const direction = (node.inputs ?? []).some((entry) => entry.id === port.id)
                    ? "inputs"
                    : "outputs";
                  return (
                    <label key={`${node.id}:${direction}:${port.id}`}>
                      <span>
                        {direction === "inputs" ? "in" : "out"} · {port.id} · {port.artifact_type}
                      </span>
                      <select
                        onChange={(event) =>
                          updateNodePortDatasetValue(node.id, port.id, direction, event.target.value)
                        }
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

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.entryInputsTitle}</h2>
        </div>
        <div className="sidebar-list">
          {selectedWorkflow.entry_inputs.map((artifact) => (
            <div className="sidebar-list__row" key={`${artifact.node_id}:${artifact.artifact_type}`}>
              <span>{artifact.node_id}</span>
              <strong>{artifact.artifact_type}</strong>
            </div>
          ))}
        </div>
      </section>

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.outputArtifactsTitle}</h2>
        </div>
        <div className="sidebar-list">
          {selectedWorkflow.output_artifacts.map((artifact) => (
            <div className="sidebar-list__row" key={`${artifact.node_id}:${artifact.artifact_type}`}>
              <span>{artifact.node_id}</span>
              <strong>{artifact.artifact_type}</strong>
            </div>
          ))}
        </div>
      </section>
    </section>
  );
}
