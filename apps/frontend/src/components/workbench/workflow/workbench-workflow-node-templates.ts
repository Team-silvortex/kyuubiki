"use client";

import type {
  WorkflowOperatorDescriptor,
  WorkflowDatasetValueInfo,
  WorkflowGraphNode,
  WorkflowGraphPort,
} from "@/lib/api";
import {
  createBridgeConfigForOperator,
  normalizeBridgeConfigForOperator,
} from "@/lib/workbench/workflow-bridge-contract";
import { createDefaultWorkflowConditionConfig } from "@/components/workbench/workflow/workbench-workflow-condition";
import { DATASET_VALUE_PRESETS } from "@/components/workbench/workflow/workbench-workflow-node-template-dataset-presets";
import { CONTROL_NODE_TEMPLATE_PRESETS } from "@/components/workbench/workflow/workbench-workflow-node-template-control-presets";
import { normalizeBridgeConfigWithSupport } from "@/lib/workbench/workflow-bridge-contract-support";

export type WorkflowNodeTemplatePreset = {
  id: string;
  kind: string;
  label: string;
  operatorId?: string;
  config?: Record<string, unknown>;
  inputs: WorkflowGraphPort[];
  outputs: WorkflowGraphPort[];
};

export type WorkflowNodeTemplateSelection = {
  kind?: string;
  operatorId?: string;
  config?: Record<string, unknown>;
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
    id: "solve.buckling_beam_1d",
    kind: "solve",
    label: "Solve beam buckling",
    operatorId: "solve.buckling_beam_1d",
    inputs: [{ id: "model", artifact_type: "study_model/buckling_beam_1d", description: "Beam-column buckling model", dataset_value: "buckling_beam_model" }],
    outputs: [{ id: "result", artifact_type: "result/buckling_beam_1d", description: "Buckling load factors and mode shapes", dataset_value: "buckling_beam_result" }],
  },
  {
    id: "solve.buckling_frame_2d",
    kind: "solve",
    label: "Solve frame buckling",
    operatorId: "solve.buckling_frame_2d",
    inputs: [{ id: "model", artifact_type: "study_model/buckling_frame_2d", description: "Statically preloaded 2D frame buckling model", dataset_value: "buckling_frame_model" }],
    outputs: [{ id: "result", artifact_type: "result/buckling_frame_2d", description: "Frame preloads, critical factors, and mode shapes", dataset_value: "buckling_frame_result" }],
  },
  {
    id: "solve.frame_2d_p_delta",
    kind: "solve",
    label: "Solve frame P-Delta",
    operatorId: "solve.frame_2d_p_delta",
    inputs: [{ id: "model", artifact_type: "study_model/frame_2d_p_delta", description: "Imperfect precritical 2D frame stability model", dataset_value: "frame_p_delta_model" }],
    outputs: [{ id: "result", artifact_type: "result/frame_2d_p_delta", description: "Load-step displacements and imperfection amplification", dataset_value: "frame_p_delta_result" }],
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
    id: "solve.electrostatic_bar_1d",
    kind: "solve",
    label: "Solve electrostatic 1D bar",
    operatorId: "solve.electrostatic_bar_1d",
    inputs: [{ id: "model", artifact_type: "study_model/electrostatic_bar_1d", description: "Electrostatic 1D bar model", dataset_value: "electrostatic_bar_model" }],
    outputs: [{ id: "result", artifact_type: "result/electrostatic_bar_1d", description: "Electrostatic 1D bar result", dataset_value: "electrostatic_bar_result" }],
  },
  {
    id: "solve.electrostatic_plane_quad_2d",
    kind: "solve",
    label: "Solve electrostatic plane quad",
    operatorId: "solve.electrostatic_plane_quad_2d",
    inputs: [{ id: "model", artifact_type: "study_model/electrostatic_plane_quad_2d", description: "Electrostatic plane quad model", dataset_value: "electrostatic_plane_quad_model" }],
    outputs: [{ id: "result", artifact_type: "result/electrostatic_plane_quad_2d", description: "Electrostatic plane quad result", dataset_value: "electrostatic_plane_quad_result" }],
  },
  {
    id: "solve.electrostatic_plane_triangle_2d",
    kind: "solve",
    label: "Solve electrostatic plane triangle",
    operatorId: "solve.electrostatic_plane_triangle_2d",
    inputs: [{ id: "model", artifact_type: "study_model/electrostatic_plane_triangle_2d", description: "Electrostatic plane triangle model", dataset_value: "electrostatic_plane_triangle_model" }],
    outputs: [{ id: "result", artifact_type: "result/electrostatic_plane_triangle_2d", description: "Electrostatic plane triangle result", dataset_value: "electrostatic_plane_triangle_result" }],
  },
  {
    id: "solve.spring_1d",
    kind: "solve",
    label: "Solve spring 1D",
    operatorId: "solve.spring_1d",
    inputs: [{ id: "model", artifact_type: "study_model/spring_1d", description: "Spring 1D model", dataset_value: "spring_model" }],
    outputs: [{ id: "result", artifact_type: "result/spring_1d", description: "Spring 1D result", dataset_value: "spring_result" }],
  },
  {
    id: "solve.truss_2d",
    kind: "solve",
    label: "Solve truss 2D",
    operatorId: "solve.truss_2d",
    inputs: [{ id: "model", artifact_type: "study_model/truss_2d", description: "Truss 2D model", dataset_value: "truss_model" }],
    outputs: [{ id: "result", artifact_type: "result/truss_2d", description: "Truss 2D result", dataset_value: "truss_result" }],
  },
  {
    id: "solve.truss_3d",
    kind: "solve",
    label: "Solve truss 3D",
    operatorId: "solve.truss_3d",
    inputs: [{ id: "model", artifact_type: "study_model/truss_3d", description: "Truss 3D model", dataset_value: "truss_3d_model" }],
    outputs: [{ id: "result", artifact_type: "result/truss_3d", description: "Truss 3D result", dataset_value: "truss_3d_result" }],
  },
  {
    id: "solve.frame_2d",
    kind: "solve",
    label: "Solve frame 2D",
    operatorId: "solve.frame_2d",
    inputs: [{ id: "model", artifact_type: "study_model/frame_2d", description: "Frame 2D model", dataset_value: "frame_2d_model" }],
    outputs: [{ id: "result", artifact_type: "result/frame_2d", description: "Frame 2D result", dataset_value: "frame_2d_result" }],
  },
  {
    id: "solve.beam_1d",
    kind: "solve",
    label: "Solve beam 1D",
    operatorId: "solve.beam_1d",
    inputs: [{ id: "model", artifact_type: "study_model/beam_1d", description: "Beam 1D model", dataset_value: "beam_model" }],
    outputs: [{ id: "result", artifact_type: "result/beam_1d", description: "Beam 1D result", dataset_value: "beam_result" }],
  },
  {
    id: "solve.spring_2d",
    kind: "solve",
    label: "Solve spring 2D",
    operatorId: "solve.spring_2d",
    inputs: [{ id: "model", artifact_type: "study_model/spring_2d", description: "Spring 2D model", dataset_value: "spring_2d_model" }],
    outputs: [{ id: "result", artifact_type: "result/spring_2d", description: "Spring 2D result", dataset_value: "spring_2d_result" }],
  },
  {
    id: "solve.spring_3d",
    kind: "solve",
    label: "Solve spring 3D",
    operatorId: "solve.spring_3d",
    inputs: [{ id: "model", artifact_type: "study_model/spring_3d", description: "Spring 3D model", dataset_value: "spring_3d_model" }],
    outputs: [{ id: "result", artifact_type: "result/spring_3d", description: "Spring 3D result", dataset_value: "spring_3d_result" }],
  },
  {
    id: "solve.thermal_beam_1d",
    kind: "solve",
    label: "Solve thermal beam 1D",
    operatorId: "solve.thermal_beam_1d",
    inputs: [{ id: "model", artifact_type: "study_model/thermal_beam_1d", description: "Thermal beam 1D model", dataset_value: "thermal_beam_model" }],
    outputs: [{ id: "result", artifact_type: "result/thermal_beam_1d", description: "Thermal beam 1D result", dataset_value: "thermal_beam_result" }],
  },
  {
    id: "solve.thermal_frame_2d",
    kind: "solve",
    label: "Solve thermal frame 2D",
    operatorId: "solve.thermal_frame_2d",
    inputs: [{ id: "model", artifact_type: "study_model/thermal_frame_2d", description: "Thermal frame 2D model", dataset_value: "thermal_frame_2d_model" }],
    outputs: [{ id: "result", artifact_type: "result/thermal_frame_2d", description: "Thermal frame 2D result", dataset_value: "thermal_frame_2d_result" }],
  },
  {
    id: "bridge.temperature_field_to_thermo_quad_2d",
    kind: "transform",
    label: "Bridge heat result to thermo model",
    operatorId: "bridge.temperature_field_to_thermo_quad_2d",
    config: createBridgeConfigForOperator("bridge.temperature_field_to_thermo_quad_2d") ?? undefined,
    inputs: [{ id: "heat_result", artifact_type: "result/heat_plane_quad_2d", description: "Heat quad result", dataset_value: "heat_result" }],
    outputs: [{ id: "thermo_model", artifact_type: "study_model/thermal_plane_quad_2d", description: "Thermo-mechanical quad model", dataset_value: "thermo_model" }],
  },
  {
    id: "bridge.electrostatic_field_to_heat_quad_2d",
    kind: "transform",
    label: "Bridge electrostatic field to heat model",
    operatorId: "bridge.electrostatic_field_to_heat_quad_2d",
    config: createBridgeConfigForOperator("bridge.electrostatic_field_to_heat_quad_2d") ?? undefined,
    inputs: [{
      id: "electrostatic_result",
      artifact_type: "result/electrostatic_plane_quad_2d",
      description: "Electrostatic quad result",
      dataset_value: "electrostatic_plane_quad_result",
    }],
    outputs: [{
      id: "heat_model",
      artifact_type: "study_model/heat_plane_quad_2d",
      description: "Heat quad model with bridged nodal loads",
      dataset_value: "heat_model",
    }],
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

function buildPortsFromOperatorDescriptor(descriptor: WorkflowOperatorDescriptor): WorkflowNodeTemplatePreset {
  const kind = workflowKindFromOperatorKind(descriptor.kind);
  return {
    id: descriptor.id,
    kind,
    label: descriptor.summary,
    operatorId: descriptor.id,
    config:
      descriptor.id === "extract.result_summary"
        ? { fields: ["max_displacement", "max_stress"] }
        : descriptor.id === "extract.field_statistics"
          ? { source: "elements", field: "von_mises_stress", output_prefix: "stress", percentiles: [50, 90, 99] }
        : descriptor.id === "extract.field_hotspots"
          ? { source: "elements", field: "von_mises_stress", output_prefix: "stress", percentile: 90, sample_limit: 4, sample_sort: "value_desc" }
        : descriptor.id === "transform.merge_summary_pair"
          ? { left_prefix: "left", right_prefix: "right", include_source_count: false }
        : descriptor.id === "transform.compare_summary_pair"
          ? { left_prefix: "baseline", right_prefix: "candidate", delta_prefix: "delta", ratio_prefix: "ratio", percent_prefix: "percent_change", include_originals: true, include_delta: true, include_ratio: true, include_percent_change: true, include_shared_field_count: true }
        : descriptor.id === "transform.normalize_summary_fields"
          ? { copy_unmapped: false, rules: [{ source: "max_temperature", target: "temperature_peak" }, { source: "max_heat_flux", target: "heat_flux_peak" }] }
        : descriptor.id === "transform.select_best_summary"
          ? { criteria: [{ field: "max_temperature", goal: "min", weight: 1 }, { field: "max_heat_flux", goal: "max", weight: 0.5 }], include_breakdown: true, include_all_scores: true }
        : descriptor.id.startsWith("bridge.")
          ? normalizeBridgeConfigForOperator(
              descriptor.id,
              createBridgeConfigForOperator(descriptor.id),
            ) ?? undefined
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

function listDescriptorBackedPresets(operatorDescriptors?: WorkflowOperatorDescriptor[]) {
  return (operatorDescriptors ?? []).map(buildPortsFromOperatorDescriptor);
}

export function listWorkflowNodeTemplatePresets(
  kind?: string,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const descriptorPresets = listDescriptorBackedPresets(operatorDescriptors);
  const merged = [...PRESETS, ...CONTROL_NODE_TEMPLATE_PRESETS];
  for (const preset of descriptorPresets) {
    if (!merged.some((entry) => entry.id === preset.id || entry.operatorId === preset.operatorId)) {
      merged.push(preset);
    }
  }
  return merged.filter((preset) => !kind || preset.kind === kind);
}

export function resolveWorkflowNodeTemplate(
  template?: WorkflowNodeTemplateSelection,
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
  if (kind === "condition") return PRESETS.find((preset) => preset.id === "condition.if_else") ?? null;
  if (kind === "output") return PRESETS.find((preset) => preset.id === "output.blank") ?? null;
  return null;
}

export function listWorkflowTemplateDatasetValues(
  template?: WorkflowNodeTemplateSelection,
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
  template?: WorkflowNodeTemplateSelection,
  operatorDescriptors?: WorkflowOperatorDescriptor[],
) {
  const operatorId = template?.operatorId?.trim();
  const operatorDescriptor = operatorId
    ? (operatorDescriptors ?? []).find((entry) => entry.id === operatorId)
    : undefined;
  const preset = resolveWorkflowNodeTemplate(template, operatorDescriptors);
  if (preset) {
    const mergedConfig = {
      ...(preset.config ?? {}),
      ...(template?.config ?? {}),
    };

    return {
      kind: preset.kind,
      operatorId: preset.operatorId,
      config:
        preset.operatorId?.startsWith("bridge.")
          ? normalizeBridgeConfigWithSupport(
              preset.operatorId,
              mergedConfig,
              operatorDescriptor ?? (preset.operatorId ? (operatorDescriptors ?? []).find((entry) => entry.id === preset.operatorId) : undefined),
            ) ?? undefined
          : mergedConfig,
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
  if (kind === "condition") {
    return {
      kind,
      operatorId: undefined,
      config: createDefaultWorkflowConditionConfig(),
      inputs: [{ id: "value", artifact_type: "artifact/json", description: "" }],
      outputs: [
        { id: "if_true", artifact_type: "artifact/json", description: "" },
        { id: "if_false", artifact_type: "artifact/json", description: "" },
      ],
    };
  }

  return {
    kind,
    operatorId,
    config:
      operatorId?.startsWith("bridge.")
        ? normalizeBridgeConfigWithSupport(
            operatorId,
            template?.config ? { ...template.config } : undefined,
            operatorDescriptor,
          ) ?? undefined
        : template?.config
          ? { ...template.config }
          : undefined,
    inputs: [{ id: "in_1", artifact_type: "artifact/json", description: "" }],
    outputs: [{ id: "out_1", artifact_type: "artifact/json", description: "" }],
  };
}
