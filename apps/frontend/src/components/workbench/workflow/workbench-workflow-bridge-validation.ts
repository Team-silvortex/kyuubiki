import type { WorkflowGraphDefinition, WorkflowGraphNode, WorkflowOperatorDescriptor } from "@/lib/api";
import {
  hasBridgeSeedModelConfig,
  resolveBridgeContractForOperator,
  type WorkflowBridgeContract,
} from "@/lib/workbench/workflow-bridge-contract";

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

export function validateBridgeNodes(
  graph: WorkflowGraphDefinition,
  operatorDescriptors: WorkflowOperatorDescriptor[],
): WorkflowBridgeValidationIssue[] {
  const issues: WorkflowBridgeValidationIssue[] = [];
  const descriptorMap = new Map(operatorDescriptors.map((descriptor) => [descriptor.id, descriptor] as const));

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
  }

  return issues;
}
