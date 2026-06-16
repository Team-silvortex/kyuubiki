import type {
  WorkflowDatasetContract,
  WorkflowGraphDefinition,
  WorkflowGraphNode,
  WorkflowOperatorDescriptor,
} from "@/lib/api";
import {
  hasBridgeSeedModelConfig,
  resolveBridgeContractForOperator,
  resolveBridgeSeedModelForOperator,
  type WorkflowBridgeContract,
} from "@/lib/workbench/workflow-bridge-contract";
import {
  buildNodeMap,
  findPort,
} from "@/components/workbench/workflow/workbench-workflow-validation-graph";

export type WorkflowBridgeValidationIssue = {
  id: string;
  level: "warning";
  message: string;
  locate: { kind: "node"; nodeId: string };
};

function isAllowedValue(value: string, options: string[]) {
  return options.length === 0 || options.includes(value);
}

function validateBridgeContractSupport(
  node: WorkflowGraphNode,
  contract: WorkflowBridgeContract,
  descriptor: WorkflowOperatorDescriptor,
): WorkflowBridgeValidationIssue[] {
  const issues: WorkflowBridgeValidationIssue[] = [];
  const support = descriptor.contract_support;
  if (!support) return issues;

  const distributionOptions = Object.keys(support.source.distributions ?? {});
  if (!isAllowedValue(contract.source.distribution, distributionOptions)) {
    issues.push({
      id: `bridge:contract:distribution:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" uses unsupported source distribution "${contract.source.distribution}" for operator "${descriptor.id}".`,
      locate: { kind: "node", nodeId: node.id },
    });
    return issues;
  }

  const sourceFieldOptions =
    support.source.distributions?.[contract.source.distribution] ?? support.source.fields ?? [];
  if (!isAllowedValue(contract.source.field, sourceFieldOptions)) {
    issues.push({
      id: `bridge:contract:source-field:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" maps unsupported source field "${contract.source.field}" for distribution "${contract.source.distribution}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  if (!isAllowedValue(contract.transform.reduction, support.transform.reductions ?? [])) {
    issues.push({
      id: `bridge:contract:reduction:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" uses unsupported reduction "${contract.transform.reduction}" for operator "${descriptor.id}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  if (!isAllowedValue(contract.target.field, support.target.fields ?? [])) {
    issues.push({
      id: `bridge:contract:target-field:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" maps unsupported target field "${contract.target.field}" for operator "${descriptor.id}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  if (contract.source.distribution === "element_to_nodes") {
    const requiredNodeIndexFields = support.source.node_index_fields ?? [];
    const missingNodeIndexFields = requiredNodeIndexFields.filter(
      (field) => !contract.source.node_index_fields.includes(field),
    );
    if (missingNodeIndexFields.length > 0) {
      issues.push({
        id: `bridge:contract:node-index-fields:${node.id}`,
        level: "warning",
        message: `Bridge node "${node.id}" is missing node index fields ${missingNodeIndexFields.join(", ")} for distribution "${contract.source.distribution}".`,
        locate: { kind: "node", nodeId: node.id },
      });
    }
  } else if (contract.source.node_index_fields.length > 0) {
    issues.push({
      id: `bridge:contract:node-index-fields-extra:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" should not define node index fields for distribution "${contract.source.distribution}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  return issues;
}

type WorkflowBridgeSemanticExpectation = {
  inputPortId: string;
  inputSemanticType: string;
  outputPortId: string;
  outputSemanticType: string;
  minimumNodeCount: number;
  requiredElementFields: string[];
  allowedSourceFields: string[];
  writableTargetFields: string[];
};

const BRIDGE_SEMANTIC_EXPECTATIONS: Record<
  string,
  WorkflowBridgeSemanticExpectation
> = {
  "bridge.electrostatic_field_to_heat_quad_2d": {
    inputPortId: "electrostatic_result",
    inputSemanticType: "result/electrostatic_plane_quad_2d",
    outputPortId: "heat_model",
    outputSemanticType: "study_model/heat_plane_quad_2d",
    minimumNodeCount: 4,
    requiredElementFields: ["node_i", "node_j", "node_k", "node_l"],
    allowedSourceFields: [
      "electric_field_magnitude",
      "electric_potential",
    ],
    writableTargetFields: ["heat_load"],
  },
  "bridge.electrostatic_field_to_heat_triangle_2d": {
    inputPortId: "electrostatic_result",
    inputSemanticType: "result/electrostatic_plane_triangle_2d",
    outputPortId: "heat_model",
    outputSemanticType: "study_model/heat_plane_triangle_2d",
    minimumNodeCount: 3,
    requiredElementFields: ["node_i", "node_j", "node_k"],
    allowedSourceFields: [
      "electric_field_magnitude",
      "electric_potential",
    ],
    writableTargetFields: ["heat_load"],
  },
  "bridge.temperature_field_to_thermo_quad_2d": {
    inputPortId: "heat_result",
    inputSemanticType: "result/heat_plane_quad_2d",
    outputPortId: "thermo_model",
    outputSemanticType: "study_model/thermal_plane_quad_2d",
    minimumNodeCount: 4,
    requiredElementFields: ["node_i", "node_j", "node_k", "node_l"],
    allowedSourceFields: [
      "temperature",
      "average_temperature",
      "heat_flux_magnitude",
      "temperature_gradient_x",
      "temperature_gradient_y",
    ],
    writableTargetFields: ["temperature_delta"],
  },
  "bridge.temperature_field_to_thermo_triangle_2d": {
    inputPortId: "heat_result",
    inputSemanticType: "result/heat_plane_triangle_2d",
    outputPortId: "thermo_model",
    outputSemanticType: "study_model/thermal_plane_triangle_2d",
    minimumNodeCount: 3,
    requiredElementFields: ["node_i", "node_j", "node_k"],
    allowedSourceFields: [
      "temperature",
      "average_temperature",
      "heat_flux_magnitude",
      "temperature_gradient_x",
      "temperature_gradient_y",
    ],
    writableTargetFields: ["temperature_delta"],
  },
};

function buildDatasetSemanticMap(contract?: WorkflowDatasetContract | null) {
  return new Map(
    (contract?.values ?? []).map((value) => [value.id, value.semantic_type] as const),
  );
}

function resolveDatasetSemanticType(
  datasetSemanticMap: Map<string, string | undefined>,
  datasetValueId?: string,
) {
  if (!datasetValueId) return undefined;
  return datasetSemanticMap.get(datasetValueId);
}

function validateBridgePortSemantics(
  node: WorkflowGraphNode,
  expectation: WorkflowBridgeSemanticExpectation,
  datasetSemanticMap: Map<string, string | undefined>,
): WorkflowBridgeValidationIssue[] {
  const issues: WorkflowBridgeValidationIssue[] = [];
  const inputPort = findPort(node, expectation.inputPortId, "inputs");
  const outputPort = findPort(node, expectation.outputPortId, "outputs");
  const inputSemanticType = resolveDatasetSemanticType(
    datasetSemanticMap,
    inputPort?.dataset_value,
  );
  const outputSemanticType = resolveDatasetSemanticType(
    datasetSemanticMap,
    outputPort?.dataset_value,
  );

  if (inputSemanticType && inputSemanticType !== expectation.inputSemanticType) {
    issues.push({
      id: `bridge:semantic:input:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" expects input semantic "${expectation.inputSemanticType}" but its input port is bound to "${inputSemanticType}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  if (outputSemanticType && outputSemanticType !== expectation.outputSemanticType) {
    issues.push({
      id: `bridge:semantic:output:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" expects output semantic "${expectation.outputSemanticType}" but its output port is bound to "${outputSemanticType}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  return issues;
}

function validateBridgeEdgeSemantics(
  graph: WorkflowGraphDefinition,
  node: WorkflowGraphNode,
  expectation: WorkflowBridgeSemanticExpectation,
  datasetSemanticMap: Map<string, string | undefined>,
): WorkflowBridgeValidationIssue[] {
  const issues: WorkflowBridgeValidationIssue[] = [];

  for (const edge of graph.edges ?? []) {
    if (edge.to.node === node.id && edge.to.port === expectation.inputPortId) {
      const edgeSemanticType = resolveDatasetSemanticType(
        datasetSemanticMap,
        edge.dataset_value,
      );
      if (edgeSemanticType && edgeSemanticType !== expectation.inputSemanticType) {
        issues.push({
          id: `bridge:edge-semantic:input:${node.id}:${edge.id}`,
          level: "warning",
          message: `Bridge node "${node.id}" receives semantic "${edgeSemanticType}" on edge "${edge.id}" but expects "${expectation.inputSemanticType}".`,
          locate: { kind: "node", nodeId: node.id },
        });
      }
    }

    if (edge.from.node === node.id && edge.from.port === expectation.outputPortId) {
      const edgeSemanticType = resolveDatasetSemanticType(
        datasetSemanticMap,
        edge.dataset_value,
      );
      if (edgeSemanticType && edgeSemanticType !== expectation.outputSemanticType) {
        issues.push({
          id: `bridge:edge-semantic:output:${node.id}:${edge.id}`,
          level: "warning",
          message: `Bridge node "${node.id}" emits semantic "${edgeSemanticType}" on edge "${edge.id}" but should emit "${expectation.outputSemanticType}".`,
          locate: { kind: "node", nodeId: node.id },
        });
      }
    }
  }

  return issues;
}

function validateBridgeSeedModelShape(
  node: WorkflowGraphNode,
  expectation: WorkflowBridgeSemanticExpectation,
): WorkflowBridgeValidationIssue[] {
  const issues: WorkflowBridgeValidationIssue[] = [];
  const seedModel = resolveBridgeSeedModelForOperator(
    node.operator_id,
    node.config as Record<string, unknown> | null | undefined,
  ) as
    | {
        nodes?: unknown;
        elements?: unknown;
      }
    | null;
  if (!seedModel) return issues;

  const nodes = Array.isArray(seedModel.nodes) ? seedModel.nodes : [];
  const elements = Array.isArray(seedModel.elements) ? seedModel.elements : [];
  if (nodes.length < expectation.minimumNodeCount) {
    issues.push({
      id: `bridge:seed-shape:nodes:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" seed model should expose at least ${expectation.minimumNodeCount} nodes for "${node.operator_id}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  const hasRequiredElementShape = elements.some((element) => {
    if (typeof element !== "object" || element === null) return false;
    return expectation.requiredElementFields.every(
      (field) => typeof (element as Record<string, unknown>)[field] === "number",
    );
  });
  if (elements.length > 0 && !hasRequiredElementShape) {
    issues.push({
      id: `bridge:seed-shape:elements:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" seed model elements do not match the expected ${expectation.requiredElementFields.length}-node topology for "${node.operator_id}".`,
      locate: { kind: "node", nodeId: node.id },
    });
  }

  return issues;
}

function validateBridgeSourceFieldCompatibility(
  node: WorkflowGraphNode,
  contract: WorkflowBridgeContract,
  expectation: WorkflowBridgeSemanticExpectation,
): WorkflowBridgeValidationIssue[] {
  if (expectation.allowedSourceFields.includes(contract.source.field)) return [];
  return [
    {
      id: `bridge:source-field-semantic:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" uses source field "${contract.source.field}" which is not recognized for upstream semantic "${expectation.inputSemanticType}".`,
      locate: { kind: "node", nodeId: node.id },
    },
  ];
}

function validateBridgeTargetFieldWritable(
  node: WorkflowGraphNode,
  contract: WorkflowBridgeContract,
  expectation: WorkflowBridgeSemanticExpectation,
): WorkflowBridgeValidationIssue[] {
  const seedModel = resolveBridgeSeedModelForOperator(
    node.operator_id,
    node.config as Record<string, unknown> | null | undefined,
  ) as
    | {
        nodes?: unknown;
        elements?: unknown;
      }
    | null;
  if (!seedModel) return [];

  const nodes = Array.isArray(seedModel.nodes) ? seedModel.nodes : [];
  const elements = Array.isArray(seedModel.elements) ? seedModel.elements : [];
  const targetField = contract.target.field;
  const writableCollections = [
    ...nodes.filter(
      (entry): entry is Record<string, unknown> =>
        typeof entry === "object" && entry !== null,
    ),
    ...elements.filter(
      (entry): entry is Record<string, unknown> =>
        typeof entry === "object" && entry !== null,
    ),
  ];

  const hasWritableField = writableCollections.some((entry) => targetField in entry);
  if (hasWritableField) return [];

  return [
    {
      id: `bridge:target-field-seed:${node.id}`,
      level: "warning",
      message: `Bridge node "${node.id}" targets field "${targetField}" but the downstream seed model does not expose a writable field for it.`,
      locate: { kind: "node", nodeId: node.id },
    },
  ];
}

export function validateBridgeNodes(
  graph: WorkflowGraphDefinition,
  operatorDescriptors: WorkflowOperatorDescriptor[],
): WorkflowBridgeValidationIssue[] {
  const issues: WorkflowBridgeValidationIssue[] = [];
  const descriptorMap = new Map(operatorDescriptors.map((descriptor) => [descriptor.id, descriptor] as const));
  const datasetSemanticMap = buildDatasetSemanticMap(graph.dataset_contract);
  const nodeMap = buildNodeMap(graph);

  for (const node of graph.nodes) {
    if (!node.operator_id?.startsWith("bridge.")) continue;
    if (
      !hasBridgeSeedModelConfig(
        node.operator_id,
        node.config as Record<string, unknown> | null | undefined,
      )
    ) {
      issues.push({
        id: `bridge:seed-model:${node.id}`,
        level: "warning",
        message:
          node.operator_id === "bridge.electrostatic_field_to_heat_quad_2d" ||
          node.operator_id === "bridge.electrostatic_field_to_heat_triangle_2d"
            ? `Bridge node "${node.id}" is missing config.seed_model for the downstream heat quad seed model.`
            : `Bridge node "${node.id}" is missing downstream thermo seed-model fields in config.`,
        locate: { kind: "node", nodeId: node.id },
      });
    }

    const descriptor = descriptorMap.get(node.operator_id);
    if (!descriptor?.contract_support) continue;
    const contract = resolveBridgeContractForOperator(
      node.operator_id,
      node.config as Record<string, unknown> | null | undefined,
    );
    if (!contract) continue;
    issues.push(...validateBridgeContractSupport(node, contract, descriptor));

    const expectation = BRIDGE_SEMANTIC_EXPECTATIONS[node.operator_id];
    if (!expectation) continue;
    issues.push(
      ...validateBridgePortSemantics(node, expectation, datasetSemanticMap),
    );
    issues.push(
      ...validateBridgeEdgeSemantics(graph, node, expectation, datasetSemanticMap),
    );
    if (nodeMap.has(node.id)) {
      issues.push(...validateBridgeSeedModelShape(node, expectation));
      issues.push(
        ...validateBridgeSourceFieldCompatibility(node, contract, expectation),
      );
      issues.push(
        ...validateBridgeTargetFieldWritable(node, contract, expectation),
      );
    }
  }

  return issues;
}
