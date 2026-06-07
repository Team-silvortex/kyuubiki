"use client";

import type {
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

export function listWorkflowNodeTemplatePresets(kind?: string) {
  return PRESETS.filter((preset) => !kind || preset.kind === kind);
}

export function resolveWorkflowNodeTemplate(template?: {
  kind?: string;
  operatorId?: string;
}): WorkflowNodeTemplatePreset | null {
  const operatorId = template?.operatorId?.trim();
  if (operatorId) {
    const byOperator = PRESETS.find((preset) => preset.operatorId === operatorId);
    if (byOperator) return byOperator;
  }

  const kind = template?.kind?.trim();
  if (kind === "input") return PRESETS.find((preset) => preset.id === "input.blank") ?? null;
  if (kind === "output") return PRESETS.find((preset) => preset.id === "output.blank") ?? null;
  return null;
}

export function listWorkflowTemplateDatasetValues(template?: {
  kind?: string;
  operatorId?: string;
}): WorkflowDatasetValueInfo[] {
  const preset = resolveWorkflowNodeTemplate(template);
  if (!preset) return [];
  const datasetIds = [...preset.inputs, ...preset.outputs]
    .map((port) => port.dataset_value)
    .filter((valueId): valueId is string => Boolean(valueId));

  return datasetIds
    .map((valueId) => DATASET_VALUE_PRESETS[valueId])
    .filter((value): value is WorkflowDatasetValueInfo => Boolean(value))
    .map((value) => JSON.parse(JSON.stringify(value)) as WorkflowDatasetValueInfo);
}

export function buildPortsForWorkflowNodeTemplate(template?: {
  kind?: string;
  operatorId?: string;
}) {
  const preset = resolveWorkflowNodeTemplate(template);
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
