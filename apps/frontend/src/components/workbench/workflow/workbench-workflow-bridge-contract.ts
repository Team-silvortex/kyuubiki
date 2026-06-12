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

export function createHeatPlaneQuadBridgeSeedModel() {
  return {
    nodes: [
      { id: "h0", x: 0, y: 0, fix_temperature: true, temperature: 100, heat_load: 0 },
      { id: "h1", x: 1, y: 0, fix_temperature: false, temperature: 0, heat_load: 0 },
      { id: "h2", x: 1, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
      { id: "h3", x: 0, y: 1, fix_temperature: true, temperature: 20, heat_load: 0 },
    ],
    elements: [
      { id: "hq0", node_i: 0, node_j: 1, node_k: 2, node_l: 3, thickness: 0.02, conductivity: 45 },
    ],
  };
}

export function createThermalPlaneQuadBridgeSeedModel() {
  return {
    material: "aluminum",
    youngs_modulus_gpa: 70,
    materials: [
      {
        id: "mat-1",
        name: "Aluminum 70 GPa",
        youngs_modulus: 70000000000,
        poisson_ratio: 0.33,
      },
    ],
    nodes: [
      { id: "n0", x: 0, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
      { id: "n1", x: 1, y: 0, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
      { id: "n2", x: 1, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
      { id: "n3", x: 0, y: 1, fix_x: true, fix_y: true, load_x: 0, load_y: 0, temperature_delta: 30 },
    ],
    elements: [
      {
        id: "tq0",
        node_i: 0,
        node_j: 1,
        node_k: 2,
        node_l: 3,
        thickness: 0.02,
        youngs_modulus: 70000000000,
        poisson_ratio: 0.33,
        thermal_expansion: 0.000011,
        material_id: "mat-1",
      },
    ],
  };
}

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

export function createBridgeConfigForOperator(operatorId?: string | null) {
  if (operatorId === "bridge.electrostatic_field_to_heat_quad_2d") {
    return {
      seed_model: createHeatPlaneQuadBridgeSeedModel(),
      contract: createElectrostaticToHeatBridgeContract(),
    };
  }
  if (operatorId === "bridge.temperature_field_to_thermo_quad_2d") {
    return createThermalPlaneQuadBridgeSeedModel();
  }
  return null;
}

export function resolveBridgeSeedModelForOperator(
  operatorId?: string | null,
  config?: Record<string, unknown> | null,
) {
  if (operatorId === "bridge.electrostatic_field_to_heat_quad_2d") {
    if (config?.seed_model && typeof config.seed_model === "object") {
      return config.seed_model as Record<string, unknown>;
    }
    return createHeatPlaneQuadBridgeSeedModel();
  }
  if (operatorId === "bridge.temperature_field_to_thermo_quad_2d") {
    if (config && typeof config === "object") {
      return config;
    }
    return createThermalPlaneQuadBridgeSeedModel();
  }
  return null;
}

export function hasBridgeSeedModelConfig(
  operatorId?: string | null,
  config?: Record<string, unknown> | null,
) {
  if (operatorId === "bridge.electrostatic_field_to_heat_quad_2d") {
    return Boolean(config?.seed_model && typeof config.seed_model === "object");
  }
  if (operatorId === "bridge.temperature_field_to_thermo_quad_2d") {
    return Boolean(config && typeof config === "object" && Array.isArray((config as { nodes?: unknown }).nodes));
  }
  return true;
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
