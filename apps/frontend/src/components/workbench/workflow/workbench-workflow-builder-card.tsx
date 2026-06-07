"use client";
import { useEffect, useMemo, useRef, useState, type ChangeEvent } from "react";
import type {
  WorkflowCatalogEntryArtifact,
  WorkflowDatasetAxis,
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
import { WorkbenchWorkflowArtifactCard } from "@/components/workbench/workflow/workbench-workflow-artifact-card";
import { validateWorkflowGraphDefinition } from "@/components/workbench/workflow/workbench-workflow-builder-validation";
import { WorkbenchWorkflowDatasetCard } from "@/components/workbench/workflow/workbench-workflow-dataset-card";

type WorkbenchWorkflowBuilderCardProps = {
  labels: WorkflowSidebarLabels;
  selectedWorkflow: WorkflowCatalogEntry | null;
  onRunWorkflowCatalog: (workflowId: string) => void;
};

function cloneWorkflowGraph(graph: WorkflowGraphDefinition | null): WorkflowGraphDefinition | null {
  if (!graph) return null;
  return JSON.parse(JSON.stringify(graph)) as WorkflowGraphDefinition;
}

function buildDraftArtifact(nextIndex: number): WorkflowCatalogEntryArtifact {
  return {
    node_id: `node_${nextIndex}`,
    artifact_type: "artifact/json",
    description: "",
  };
}

function ensureDatasetContract(
  graph: WorkflowGraphDefinition | null,
): WorkflowDatasetContract | null {
  if (!graph) return null;
  if (!graph.dataset_contract) {
    graph.dataset_contract = {
      schema_version: "kyuubiki.workflow-dataset/v1",
      id: `${graph.id}.dataset`,
      version: graph.version ?? "1.4.0",
      name: `${graph.name ?? graph.id} dataset contract`,
      values: [],
      metadata: {},
    };
  }
  return graph.dataset_contract;
}

function buildDraftDatasetValue(nextIndex: number): WorkflowDatasetValueInfo {
  return {
    id: `value_${nextIndex}`,
    data_class: "field",
    element_type: "scalar",
    shape: { axes: [] },
    semantic_type: "result/derived",
    encoding: "json",
    unit: "",
  };
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
    if (nextDraft) {
      nextDraft.entry_inputs = selectedWorkflow?.entry_inputs
        ? [...selectedWorkflow.entry_inputs]
        : nextDraft.entry_inputs ?? [];
      nextDraft.output_artifacts = selectedWorkflow?.output_artifacts
        ? [...selectedWorkflow.output_artifacts]
        : nextDraft.output_artifacts ?? [];
    }
    setDraftGraph(nextDraft);
    setSelectedDatasetValueId(nextDraft?.dataset_contract?.values?.[0]?.id ?? null);
    setImportMessage(null);
  }, [selectedWorkflow]);

  const selectedGraph = draftGraph;
  const selectedNodes = selectedGraph?.nodes ?? [];
  const selectedEdges = selectedGraph?.edges ?? [];
  const selectedEntryInputs = selectedGraph?.entry_inputs ?? [];
  const selectedOutputArtifacts = selectedGraph?.output_artifacts ?? [];
  const selectedDatasetContract = selectedGraph?.dataset_contract ?? null;
  const selectedDatasetValues = selectedDatasetContract?.values ?? [];
  const validationIssues = useMemo(
    () =>
      validateWorkflowGraphDefinition(
        selectedGraph,
        selectedEntryInputs,
        selectedOutputArtifacts,
      ),
    [selectedGraph, selectedEntryInputs, selectedOutputArtifacts],
  );
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

  function updateArtifacts(
    field: "entry_inputs" | "output_artifacts",
    updater: (artifacts: WorkflowCatalogEntryArtifact[]) => WorkflowCatalogEntryArtifact[],
  ) {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      next[field] = updater([...(next[field] ?? [])]);
      return next;
    });
  }

  function addArtifact(field: "entry_inputs" | "output_artifacts") {
    const nextIndex = ((selectedGraph?.[field] ?? []).length || 0) + 1;
    updateArtifacts(field, (artifacts) => [...artifacts, buildDraftArtifact(nextIndex)]);
  }

  function removeArtifact(field: "entry_inputs" | "output_artifacts", index: number) {
    updateArtifacts(field, (artifacts) => artifacts.filter((_, artifactIndex) => artifactIndex !== index));
  }

  function updateArtifact(
    field: "entry_inputs" | "output_artifacts",
    index: number,
    updater: (artifact: WorkflowCatalogEntryArtifact) => WorkflowCatalogEntryArtifact,
  ) {
    updateArtifacts(field, (artifacts) =>
      artifacts.map((artifact, artifactIndex) =>
        artifactIndex === index ? updater(artifact) : artifact,
      ),
    );
  }

  function applyValidationFix(issueId: string) {
    const issue = validationIssues.find((entry) => entry.id === issueId);
    if (!issue?.fix) return;
    const fix = issue.fix;
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      switch (fix.kind) {
        case "set_edge_artifact_type_from_source":
        case "set_edge_artifact_type_from_target": {
          const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
          if (edge) edge.artifact_type = fix.artifactType;
          break;
        }
        case "set_catalog_artifact_type": {
          const artifacts = next[fix.mode === "entry" ? "entry_inputs" : "output_artifacts"] ?? [];
          const artifact = artifacts.find(
            (entry) =>
              entry.node_id === fix.nodeId &&
              entry.artifact_type === fix.currentArtifactType,
          );
          if (artifact) artifact.artifact_type = fix.artifactType;
          break;
        }
        case "clear_port_dataset_value": {
          const node = next.nodes.find((entry) => entry.id === fix.nodeId);
          const port = node?.[fix.direction]?.find((entry) => entry.id === fix.portId);
          if (port) port.dataset_value = undefined;
          break;
        }
        case "clear_edge_dataset_value": {
          const edge = next.edges?.find((entry) => entry.id === fix.edgeId);
          if (edge) edge.dataset_value = undefined;
          break;
        }
      }
      return next;
    });
  }

  function addDatasetValue() {
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      const contract = ensureDatasetContract(next);
      if (!contract) return current;
      const nextValue = buildDraftDatasetValue(contract.values.length + 1);
      contract.values = [...contract.values, nextValue];
      setSelectedDatasetValueId(nextValue.id);
      return next;
    });
  }

  function removeSelectedDatasetValue() {
    if (!selectedDatasetValue) return;
    setDraftGraph((current) => {
      if (!current) return current;
      const next = cloneWorkflowGraph(current);
      if (!next) return current;
      const contract = ensureDatasetContract(next);
      if (!contract) return current;
      contract.values = contract.values.filter((value) => value.id !== selectedDatasetValue.id);
      for (const node of next.nodes) {
        for (const port of [...(node.inputs ?? []), ...(node.outputs ?? [])]) {
          if (port.dataset_value === selectedDatasetValue.id) port.dataset_value = undefined;
        }
      }
      for (const edge of next.edges ?? []) {
        if (edge.dataset_value === selectedDatasetValue.id) edge.dataset_value = undefined;
      }
      setSelectedDatasetValueId(contract.values[0]?.id ?? null);
      return next;
    });
  }

  function updateDatasetAxes(
    valueId: string,
    updater: (axes: WorkflowDatasetAxis[]) => WorkflowDatasetAxis[],
  ) {
    updateDatasetValue(valueId, (value) => ({
      ...value,
      shape: {
        ...(value.shape ?? {}),
        axes: updater(value.shape?.axes ?? []),
      },
    }));
  }

  function addDatasetAxis() {
    if (!selectedDatasetValue) return;
    updateDatasetAxes(selectedDatasetValue.id, (axes) => [
      ...axes,
      { id: `axis_${axes.length + 1}` },
    ]);
  }

  function removeDatasetAxis(axisId: string) {
    if (!selectedDatasetValue) return;
    updateDatasetAxes(selectedDatasetValue.id, (axes) => axes.filter((axis) => axis.id !== axisId));
  }

  function updateDatasetAxis(
    axisId: string,
    updater: (axis: WorkflowDatasetAxis) => WorkflowDatasetAxis,
  ) {
    if (!selectedDatasetValue) return;
    updateDatasetAxes(selectedDatasetValue.id, (axes) =>
      axes.map((axis) => (axis.id === axisId ? updater(axis) : axis)),
    );
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
      if (nextGraph) {
        nextGraph.entry_inputs = nextGraph.entry_inputs ?? selectedEntryInputs;
        nextGraph.output_artifacts = nextGraph.output_artifacts ?? selectedOutputArtifacts;
      }
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
      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{labels.validationTitle}</h2>
          <span
            className={`status-pill status-pill--${validationIssues.length > 0 ? "watch" : "good"}`}
          >
            {validationIssues.length}
          </span>
        </div>
        {validationIssues.length > 0 ? (
          <div className="sidebar-list">
            {validationIssues.map((issue) => (
              <div className="sidebar-list__row" key={issue.id}>
                <span>{issue.level}</span>
                <strong>{issue.message}</strong>
                {issue.fix ? (
                  <button onClick={() => applyValidationFix(issue.id)} type="button">
                    {labels.validationFixLabel}
                  </button>
                ) : null}
              </div>
            ))}
          </div>
        ) : (
          <p className="card-copy">{labels.validationOkLabel}</p>
        )}
      </section>
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
          <strong>{selectedEntryInputs.length}</strong>
        </div>
        <div className="sidebar-list__row">
          <span>{labels.outputArtifactsTitle}</span>
          <strong>{selectedOutputArtifacts.length}</strong>
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

      <WorkbenchWorkflowDatasetCard
        addDatasetAxis={addDatasetAxis}
        addDatasetValue={addDatasetValue}
        labels={labels}
        removeDatasetAxis={removeDatasetAxis}
        removeSelectedDatasetValue={removeSelectedDatasetValue}
        selectedDatasetContract={selectedDatasetContract}
        selectedDatasetValue={selectedDatasetValue}
        selectedDatasetValueId={selectedDatasetValueId}
        selectedDatasetValues={selectedDatasetValues}
        selectedEdges={selectedEdges}
        selectedNodes={selectedNodes}
        setSelectedDatasetValueId={setSelectedDatasetValueId}
        updateDatasetAxis={updateDatasetAxis}
        updateDatasetValue={updateDatasetValue}
        updateEdgeDatasetValue={updateEdgeDatasetValue}
        updateNodePortDatasetValue={updateNodePortDatasetValue}
      />

      <WorkbenchWorkflowArtifactCard
        addLabel={labels.artifactAddEntryLabel}
        artifacts={selectedEntryInputs}
        labels={labels}
        mode="entry"
        onAddArtifact={() => addArtifact("entry_inputs")}
        onRemoveArtifact={(index) => removeArtifact("entry_inputs", index)}
        onUpdateArtifact={(index, updater) => updateArtifact("entry_inputs", index, updater)}
        selectedNodes={selectedNodes}
        title={labels.entryInputsTitle}
      />

      <WorkbenchWorkflowArtifactCard
        addLabel={labels.artifactAddOutputLabel}
        artifacts={selectedOutputArtifacts}
        labels={labels}
        mode="output"
        onAddArtifact={() => addArtifact("output_artifacts")}
        onRemoveArtifact={(index) => removeArtifact("output_artifacts", index)}
        onUpdateArtifact={(index, updater) => updateArtifact("output_artifacts", index, updater)}
        selectedNodes={selectedNodes}
        title={labels.outputArtifactsTitle}
      />
    </section>
  );
}
