"use client";

import type {
  WorkflowOperatorDescriptor,
  WorkflowDatasetValueInfo,
  WorkflowGraphNode,
  WorkflowGraphPort,
} from "@/lib/api";

type WorkflowNodeTemplatePreset = {
  id: string;
  kind: string;
  label: string;
  operatorId?: string;
  config?: Record<string, unknown>;
  inputs: WorkflowGraphPort[];
  outputs: WorkflowGraphPort[];
};

const DATASET_VALUE_PRESETS: Record<string, WorkflowDatasetValueInfo> = {
  heat_model: {
    id: "heat_model",
    data_class: "study_model",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "study_model/heat_plane_quad_2d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.heat_plane_quad_2d.input", version: "1" },
  },
  heat_result: {
    id: "heat_result",
    data_class: "result",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "result/heat_plane_quad_2d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.heat_plane_quad_2d.output", version: "1" },
  },
  thermo_model: {
    id: "thermo_model",
    data_class: "study_model",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "study_model/thermal_plane_quad_2d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.thermal_plane_quad_2d.input", version: "1" },
  },
  thermo_result: {
    id: "thermo_result",
    data_class: "result",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "result/thermal_plane_quad_2d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.thermal_plane_quad_2d.output", version: "1" },
  },
  frame_model: {
    id: "frame_model",
    data_class: "study_model",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "study_model/frame_3d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.frame_3d.input", version: "1" },
  },
  frame_result: {
    id: "frame_result",
    data_class: "result",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "result/frame_3d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.frame_3d.output", version: "1" },
  },
  thermal_frame_model: {
    id: "thermal_frame_model",
    data_class: "study_model",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "study_model/thermal_frame_3d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.thermal_frame_3d.input", version: "1" },
  },
  thermal_frame_result: {
    id: "thermal_frame_result",
    data_class: "result",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "result/thermal_frame_3d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.thermal_frame_3d.output", version: "1" },
  },
  thermal_truss_model: {
    id: "thermal_truss_model",
    data_class: "study_model",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "study_model/thermal_truss_3d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.thermal_truss_3d.input", version: "1" },
  },
  thermal_truss_result: {
    id: "thermal_truss_result",
    data_class: "result",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "result/thermal_truss_3d",
    encoding: "json",
    schema_ref: { schema: "kyuubiki.operator.solve.thermal_truss_3d.output", version: "1" },
  },
  result_summary: {
    id: "result_summary",
    data_class: "artifact",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "artifact/result_summary",
    encoding: "json",
  },
  json_export: {
    id: "json_export",
    data_class: "artifact",
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: "artifact/json",
    encoding: "json",
  },
  csv_export: {
    id: "csv_export",
    data_class: "artifact",
    element_type: "table",
    shape: { axes: [] },
    semantic_type: "artifact/csv",
    encoding: "text/csv",
  },
};

const PRESETS: WorkflowNodeTemplatePreset[] = [
  {
    id: "input.blank",
    kind: "input",
    label: "Input",
    inputs: [],
    outputs: [{ id: "value", artifact_type: "artifact/json", description: "Workflow input", dataset_value: "input_value" }],
  },
  {
    id: "solve.heat_plane_quad_2d",
    kind: "solve",
    label: "Solve heat plane quad",
    operatorId: "solve.heat_plane_quad_2d",
    inputs: [{ id: "model", artifact_type: "study_model/heat_plane_quad_2d", description: "Heat quad model", dataset_value: "heat_model" }],
    outputs: [{ id: "result", artifact_type: "result/heat_plane_quad_2d", description: "Heat solve result", dataset_value: "heat_result" }],
  },
  {
    id: "solve.thermal_plane_quad_2d",
    kind: "solve",
    label: "Solve thermo plane quad",
    operatorId: "solve.thermal_plane_quad_2d",
    inputs: [{ id: "model", artifact_type: "study_model/thermal_plane_quad_2d", description: "Thermo-mechanical quad model", dataset_value: "thermo_model" }],
    outputs: [{ id: "result", artifact_type: "result/thermal_plane_quad_2d", description: "Thermo-mechanical solve result", dataset_value: "thermo_result" }],
  },
  {
    id: "solve.frame_3d",
    kind: "solve",
    label: "Solve 3D frame",
    operatorId: "solve.frame_3d",
    inputs: [{ id: "model", artifact_type: "study_model/frame_3d", description: "3D frame model", dataset_value: "frame_model" }],
    outputs: [{ id: "result", artifact_type: "result/frame_3d", description: "3D frame result", dataset_value: "frame_result" }],
  },
  {
    id: "solve.thermal_frame_3d",
    kind: "solve",
    label: "Solve thermal 3D frame",
    operatorId: "solve.thermal_frame_3d",
    inputs: [{ id: "model", artifact_type: "study_model/thermal_frame_3d", description: "Thermal 3D frame model", dataset_value: "thermal_frame_model" }],
    outputs: [{ id: "result", artifact_type: "result/thermal_frame_3d", description: "Thermal 3D frame result", dataset_value: "thermal_frame_result" }],
  },
  {
    id: "solve.thermal_truss_3d",
    kind: "solve",
    label: "Solve thermal 3D truss",
    operatorId: "solve.thermal_truss_3d",
    inputs: [{ id: "model", artifact_type: "study_model/thermal_truss_3d", description: "Thermal 3D truss model", dataset_value: "thermal_truss_model" }],
    outputs: [{ id: "result", artifact_type: "result/thermal_truss_3d", description: "Thermal 3D truss result", dataset_value: "thermal_truss_result" }],
  },
  {
    id: "bridge.temperature_field_to_thermo_quad_2d",
    kind: "transform",
    label: "Bridge heat result to thermo model",
    operatorId: "bridge.temperature_field_to_thermo_quad_2d",
    inputs: [{ id: "heat_result", artifact_type: "result/heat_plane_quad_2d", description: "Heat quad result", dataset_value: "heat_result" }],
    outputs: [{ id: "thermo_model", artifact_type: "study_model/thermal_plane_quad_2d", description: "Thermo-mechanical quad model", dataset_value: "thermo_model" }],
  },
  {
    id: "extract.result_summary",
    kind: "extract",
    label: "Extract result summary",
    operatorId: "extract.result_summary",
    config: { fields: ["max_displacement", "max_stress"] },
    inputs: [{ id: "result", artifact_type: "result/derived", description: "Solver result", dataset_value: "thermo_result" }],
    outputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "Result summary", dataset_value: "result_summary" }],
  },
  {
    id: "export.summary_json",
    kind: "export",
    label: "Export summary JSON",
    operatorId: "export.summary_json",
    inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "Summary artifact", dataset_value: "result_summary" }],
    outputs: [{ id: "json", artifact_type: "artifact/json", description: "JSON export", dataset_value: "json_export" }],
  },
  {
    id: "export.summary_csv",
    kind: "export",
    label: "Export summary CSV",
    operatorId: "export.summary_csv",
    inputs: [{ id: "summary", artifact_type: "artifact/result_summary", description: "Summary artifact", dataset_value: "result_summary" }],
    outputs: [{ id: "csv", artifact_type: "artifact/csv", description: "CSV export", dataset_value: "csv_export" }],
  },
  {
    id: "output.blank",
    kind: "output",
    label: "Output",
    inputs: [{ id: "result", artifact_type: "artifact/json", description: "Workflow output", dataset_value: "output_value" }],
    outputs: [],
  },
];

function clonePorts(ports: WorkflowGraphPort[]): WorkflowGraphPort[] {
  return ports.map((port) => ({ ...port }));
}

function workflowKindFromOperatorKind(
  kind: WorkflowOperatorDescriptor["kind"],
): string {
  if (kind === "solver") return "solve";
  if (kind === "workflow_bridge") return "transform";
  return kind;
}

function fallbackDataClassForArtifactType(artifactType: string) {
  if (artifactType.startsWith("model/") || artifactType.startsWith("study_model/")) return "study_model";
  if (artifactType.startsWith("result/")) return "result";
  if (artifactType.startsWith("extract/") || artifactType.startsWith("report/")) return "artifact";
  if (artifactType.startsWith("export/") || artifactType.startsWith("artifact/")) return "artifact";
  return "field";
}

function buildDatasetValuePresetFromPort(
  port: WorkflowGraphPort,
  schemaRef?: { schema: string; version: string } | null,
): WorkflowDatasetValueInfo | null {
  const datasetId = port.dataset_value?.trim();
  if (!datasetId) return null;
  const preset = DATASET_VALUE_PRESETS[datasetId];
  if (preset) return JSON.parse(JSON.stringify(preset)) as WorkflowDatasetValueInfo;
  return {
    id: datasetId,
    data_class: fallbackDataClassForArtifactType(port.artifact_type),
    element_type: "json_object",
    shape: { axes: [] },
    semantic_type: port.artifact_type,
    encoding: port.artifact_type.startsWith("export/summary_csv") ? "text/csv" : "json",
    schema_ref: schemaRef ? { ...schemaRef } : undefined,
  };
}

function buildPortsFromOperatorDescriptor(
  descriptor: WorkflowOperatorDescriptor,
): WorkflowNodeTemplatePreset {
  const kind = workflowKindFromOperatorKind(descriptor.kind);
  return {
    id: descriptor.id,
    kind,
    label: descriptor.summary,
    operatorId: descriptor.id,
    config:
      descriptor.id === "extract.result_summary"
        ? { fields: ["max_displacement", "max_stress"] }
        : undefined,
    inputs: descriptor.inputs.map((port) => ({
      id: port.id,
      artifact_type: port.artifact_type,
      description: port.description,
      dataset_value: port.dataset_value ?? undefined,
    })),
    outputs: descriptor.outputs.map((port) => ({
      id: port.id,
      artifact_type: port.artifact_type,
      description: port.description,
      dataset_value: port.dataset_value ?? undefined,
    })),
  };
}

function listDescriptorBackedPresets(
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  return (operatorDescriptors ?? []).map(buildPortsFromOperatorDescriptor);
}

export function listWorkflowNodeTemplatePresets(
  kind?: string,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const descriptorPresets = listDescriptorBackedPresets(operatorDescriptors);
  const merged = [...descriptorPresets];
  for (const preset of PRESETS) {
    if (!merged.some((entry) => entry.id === preset.id || entry.operatorId === preset.operatorId)) {
      merged.push(preset);
    }
  }
  return merged.filter((preset) => !kind || preset.kind === kind);
}

export function resolveWorkflowNodeTemplate(
  template?: {
  kind?: string;
  operatorId?: string;
  },
  operatorDescriptors?: WorkflowOperatorDescriptor[],
): WorkflowNodeTemplatePreset | null {
  const operatorId = template?.operatorId?.trim();
  if (operatorId) {
    const byOperator = listWorkflowNodeTemplatePresets(undefined, operatorDescriptors).find(
      (preset) => preset.operatorId === operatorId,
    );
    if (byOperator) return byOperator;
  }

  const kind = template?.kind?.trim();
  if (kind === "input") return PRESETS.find((preset) => preset.id === "input.blank") ?? null;
  if (kind === "output") return PRESETS.find((preset) => preset.id === "output.blank") ?? null;
  return null;
}

export function listWorkflowTemplateDatasetValues(
  template?: {
    kind?: string;
    operatorId?: string;
  },
  operatorDescriptors?: WorkflowOperatorDescriptor[],
): WorkflowDatasetValueInfo[] {
  const descriptor = (operatorDescriptors ?? []).find(
    (entry) => entry.id === template?.operatorId?.trim(),
  );
  if (descriptor) {
    return [...descriptor.inputs, ...descriptor.outputs]
      .map((port) =>
        buildDatasetValuePresetFromPort(
          {
            id: port.id,
            artifact_type: port.artifact_type,
            description: port.description,
            dataset_value: port.dataset_value ?? undefined,
          },
          port.schema_ref,
        ),
      )
      .filter((value): value is WorkflowDatasetValueInfo => Boolean(value));
  }

  const preset = resolveWorkflowNodeTemplate(template, operatorDescriptors);
  if (!preset) return [];
  const datasetIds = [...preset.inputs, ...preset.outputs]
    .map((port) => port.dataset_value)
    .filter((valueId): valueId is string => Boolean(valueId));

  return datasetIds
    .map((valueId) => DATASET_VALUE_PRESETS[valueId])
    .filter((value): value is WorkflowDatasetValueInfo => Boolean(value))
    .map((value) => JSON.parse(JSON.stringify(value)) as WorkflowDatasetValueInfo);
}

export function buildPortsForWorkflowNodeTemplate(
  template?: {
    kind?: string;
    operatorId?: string;
  },
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const preset = resolveWorkflowNodeTemplate(template, operatorDescriptors);
  if (preset) {
    return {
      kind: preset.kind,
      operatorId: preset.operatorId,
      config: preset.config ? { ...preset.config } : undefined,
      inputs: clonePorts(preset.inputs),
      outputs: clonePorts(preset.outputs),
    };
  }

  const kind = template?.kind?.trim() || "transform";
  if (kind === "input") {
    return {
      kind,
      operatorId: template?.operatorId?.trim() || undefined,
      config: undefined,
      inputs: [] as WorkflowGraphPort[],
      outputs: [{ id: "value", artifact_type: "artifact/json", description: "" }],
    };
  }
  if (kind === "output") {
    return {
      kind,
      operatorId: template?.operatorId?.trim() || undefined,
      config: undefined,
      inputs: [{ id: "result", artifact_type: "artifact/json", description: "" }],
      outputs: [] as WorkflowGraphPort[],
    };
  }

  return {
    kind,
    operatorId: template?.operatorId?.trim() || undefined,
    config: undefined,
    inputs: [{ id: "in_1", artifact_type: "artifact/json", description: "" }],
    outputs: [{ id: "out_1", artifact_type: "artifact/json", description: "" }],
  };
}
