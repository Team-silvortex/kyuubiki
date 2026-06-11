"use client";

export const ELECTROSTATIC_TO_HEAT_BRIDGE_CONTRACT_SCHEMA = {
  schema: "kyuubiki.bridge-contract.electrostatic_to_heat.v1",
  version: "1",
} as const;

export const HEAT_TO_THERMO_BRIDGE_CONTRACT_SCHEMA = {
  schema: "kyuubiki.bridge-contract.heat_to_thermo.v1",
  version: "1",
} as const;

export const WORKFLOW_BRIDGE_CONTRACT_DOCS_HREF = "/docs/workflow-bridge-contracts";

export type WorkflowBridgeContractSchemaRef = {
  schema: string;
  version: string;
};

export type WorkflowBridgeContract = {
  version: string;
  source: {
    field: string;
    distribution: string;
    node_index_fields: string[];
  };
  transform: {
    scale: number;
    reduction: string;
    default_value: number;
  };
  target: {
    field: string;
  };
};

export function isWorkflowBridgeContractOperator(operatorId?: string | null) {
  return (
    operatorId === "bridge.electrostatic_field_to_heat_quad_2d" ||
    operatorId === "bridge.temperature_field_to_thermo_quad_2d"
  );
}

export function createElectrostaticToHeatBridgeContract(
  scale = 50,
): WorkflowBridgeContract {
  return {
    version: "kyuubiki.bridge-contract/v1",
    source: {
      field: "electric_field_magnitude",
      distribution: "element_to_nodes",
      node_index_fields: ["node_i", "node_j", "node_k", "node_l"],
    },
    transform: {
      scale,
      reduction: "mean",
      default_value: 0,
    },
    target: {
      field: "heat_load",
    },
  };
}

export function createHeatToThermoBridgeContract(
  scale = 1,
): WorkflowBridgeContract {
  return {
    version: "kyuubiki.bridge-contract/v1",
    source: {
      field: "temperature",
      distribution: "node_to_node",
      node_index_fields: [],
    },
    transform: {
      scale,
      reduction: "copy",
      default_value: 0,
    },
    target: {
      field: "temperature_delta",
    },
  };
}

export function createBridgeContractForOperator(
  operatorId?: string | null,
): WorkflowBridgeContract | null {
  if (operatorId === "bridge.electrostatic_field_to_heat_quad_2d") {
    return createElectrostaticToHeatBridgeContract();
  }
  if (operatorId === "bridge.temperature_field_to_thermo_quad_2d") {
    return createHeatToThermoBridgeContract();
  }
  return null;
}

export function resolveBridgeContractForOperator(
  operatorId?: string | null,
  config?: Record<string, unknown> | null,
): WorkflowBridgeContract | null {
  const contract = config?.contract;
  if (
    contract &&
    typeof contract === "object" &&
    typeof (contract as { version?: unknown }).version === "string"
  ) {
    return contract as WorkflowBridgeContract;
  }
  return createBridgeContractForOperator(operatorId);
}
