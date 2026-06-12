"use client";

import type {
  WorkflowGraphNode,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import type { WorkflowNodeTemplatePreset } from "@/components/workbench/workflow/workbench-workflow-node-templates";

export type WorkflowOperatorPresetRecommendation = {
  preset: WorkflowNodeTemplatePreset;
  reason: string;
  score: number;
};

function artifactMatches(sourceType: string, targetType: string) {
  if (sourceType === targetType) return true;
  if (targetType === "result/derived" && sourceType.startsWith("result/")) return true;
  if (targetType === "artifact/json" && sourceType.startsWith("artifact/")) return true;
  return false;
}

function scoreArtifactCompatibility(
  sourcePorts: NonNullable<WorkflowGraphNode["outputs"]>,
  targetPorts: WorkflowNodeTemplatePreset["inputs"],
) {
  let score = 0;
  let reason: string | null = null;
  for (const input of targetPorts) {
    for (const output of sourcePorts) {
      if (!artifactMatches(output.artifact_type, input.artifact_type)) continue;
      score += output.artifact_type === input.artifact_type ? 120 : 70;
      if (!reason) reason = `accepts ${output.artifact_type} from the current node`;
      if (input.dataset_value && output.dataset_value && input.dataset_value === output.dataset_value) {
        score += 20;
      }
    }
  }
  return { score, reason };
}

function scoreNamedWorkflowChain(sourceOperatorId: string | undefined, targetOperatorId: string | undefined) {
  if (!sourceOperatorId || !targetOperatorId) return { score: 0, reason: null as string | null };
  if (sourceOperatorId === "solve.electrostatic_plane_quad_2d" && targetOperatorId === "bridge.electrostatic_field_to_heat_quad_2d") {
    return { score: 220, reason: "continues the electrostatic -> heat bridge" };
  }
  if (sourceOperatorId === "solve.electrostatic_plane_triangle_2d" && targetOperatorId === "bridge.electrostatic_field_to_heat_triangle_2d") {
    return { score: 220, reason: "continues the electrostatic triangle -> heat triangle bridge" };
  }
  if (sourceOperatorId === "bridge.electrostatic_field_to_heat_quad_2d" && targetOperatorId === "solve.heat_plane_quad_2d") {
    return { score: 200, reason: "solves the bridged heat model next" };
  }
  if (sourceOperatorId === "bridge.electrostatic_field_to_heat_triangle_2d" && targetOperatorId === "solve.heat_plane_triangle_2d") {
    return { score: 200, reason: "solves the bridged heat triangle model next" };
  }
  if (sourceOperatorId === "solve.heat_plane_quad_2d" && targetOperatorId === "bridge.temperature_field_to_thermo_quad_2d") {
    return { score: 220, reason: "continues the heat -> thermo bridge" };
  }
  if (sourceOperatorId === "solve.heat_plane_triangle_2d" && targetOperatorId === "bridge.temperature_field_to_thermo_triangle_2d") {
    return { score: 220, reason: "continues the heat triangle -> thermo triangle bridge" };
  }
  if (sourceOperatorId === "bridge.temperature_field_to_thermo_quad_2d" && targetOperatorId === "solve.thermal_plane_quad_2d") {
    return { score: 200, reason: "solves the bridged thermo model next" };
  }
  if (sourceOperatorId === "bridge.temperature_field_to_thermo_triangle_2d" && targetOperatorId === "solve.thermal_plane_triangle_2d") {
    return { score: 200, reason: "solves the bridged thermo triangle model next" };
  }
  if (sourceOperatorId === "solve.electrostatic_plane_quad_2d" && targetOperatorId === "extract.result_summary") {
    return { score: 180, reason: "extracts an electrostatic field summary for downstream aggregation" };
  }
  if (sourceOperatorId === "solve.electrostatic_plane_quad_2d" && targetOperatorId === "extract.field_statistics") {
    return { score: 195, reason: "extracts electric-field statistics directly from the electrostatic plane result" };
  }
  if (sourceOperatorId === "solve.electrostatic_plane_quad_2d" && targetOperatorId === "extract.field_hotspots") {
    return { score: 198, reason: "extracts high-field hotspot candidates from the electrostatic plane result" };
  }
  if (sourceOperatorId === "solve.heat_plane_quad_2d" && targetOperatorId === "extract.result_summary") {
    return { score: 180, reason: "extracts a heat summary for downstream aggregation" };
  }
  if (sourceOperatorId === "solve.heat_plane_quad_2d" && targetOperatorId === "extract.field_statistics") {
    return { score: 190, reason: "extracts temperature or heat-flux statistics from the heat plane result" };
  }
  if (sourceOperatorId === "solve.heat_plane_quad_2d" && targetOperatorId === "extract.field_hotspots") {
    return { score: 192, reason: "extracts thermal hotspot candidates from the heat plane result" };
  }
  if (sourceOperatorId === "solve.thermal_plane_quad_2d" && targetOperatorId === "extract.result_summary") {
    return { score: 180, reason: "extracts a summary from the thermo result" };
  }
  if (sourceOperatorId === "solve.thermal_plane_quad_2d" && targetOperatorId === "extract.field_statistics") {
    return { score: 190, reason: "extracts coupled field statistics from the thermo result" };
  }
  if (sourceOperatorId === "solve.thermal_plane_quad_2d" && targetOperatorId === "extract.field_hotspots") {
    return { score: 192, reason: "extracts high-response hotspot candidates from the thermo result" };
  }
  if (sourceOperatorId === "extract.result_summary" && targetOperatorId === "transform.merge_summary_pair") {
    return { score: 170, reason: "merges multiple stage summaries into one workflow report" };
  }
  if (sourceOperatorId === "transform.merge_summary_pair" && targetOperatorId === "transform.merge_summary_pair") {
    return { score: 150, reason: "extends the combined summary with another stage report" };
  }
  if (sourceOperatorId === "extract.result_summary" && targetOperatorId?.startsWith("export.summary_")) {
    return { score: 160, reason: "exports the generated summary artifact" };
  }
  if (sourceOperatorId === "extract.result_summary" && targetOperatorId === "export.alert_markdown") {
    return { score: 164, reason: "renders the extracted summary as a readable alert document" };
  }
  if (sourceOperatorId === "transform.merge_summary_pair" && targetOperatorId?.startsWith("export.summary_")) {
    return { score: 165, reason: "exports the merged multi-stage summary artifact" };
  }
  if (sourceOperatorId === "transform.merge_summary_pair" && targetOperatorId === "export.alert_markdown") {
    return { score: 166, reason: "renders the merged workflow summary as one alert report" };
  }
  if (sourceOperatorId === "extract.field_hotspots" && targetOperatorId?.startsWith("export.summary_")) {
    return { score: 168, reason: "exports the extracted hotspot summary for inspection or downstream automation" };
  }
  if (sourceOperatorId === "extract.field_hotspots" && targetOperatorId === "export.alert_markdown") {
    return { score: 172, reason: "turns hotspot candidates into a readable alert for operators or automation logs" };
  }
  return { score: 0, reason: null };
}

export function buildWorkflowOperatorPresetRecommendations(params: {
  operatorDescriptorMap: Map<string, WorkflowOperatorDescriptor>;
  presets: WorkflowNodeTemplatePreset[];
  sourceNode: WorkflowGraphNode | null;
}) {
  const { operatorDescriptorMap, presets, sourceNode } = params;
  if (!sourceNode?.outputs?.length) return [];
  const sourceOutputs = sourceNode.outputs;
  return presets
    .map((preset) => {
      const artifactScore = scoreArtifactCompatibility(sourceOutputs, preset.inputs ?? []);
      const chainScore = scoreNamedWorkflowChain(sourceNode.operator_id, preset.operatorId);
      const descriptor = preset.operatorId ? operatorDescriptorMap.get(preset.operatorId) : undefined;
      const domainBonus = descriptor?.domain && sourceNode.operator_id?.includes(descriptor.domain) ? 15 : 0;
      const score = artifactScore.score + chainScore.score + domainBonus;
      const reason = chainScore.reason ?? artifactScore.reason;
      if (score <= 0 || !reason) return null;
      return { preset, reason, score };
    })
    .filter((entry): entry is WorkflowOperatorPresetRecommendation => Boolean(entry))
    .sort((left, right) => right.score - left.score || left.preset.label.localeCompare(right.preset.label))
    .slice(0, 6);
}
