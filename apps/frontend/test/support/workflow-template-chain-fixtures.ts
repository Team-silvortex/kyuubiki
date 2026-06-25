export function buildDiagnosticsTemplateChainGraph() {
  return {
    id: "workflow.test",
    name: "workflow test",
    version: "1.11.5",
    nodes: [
      { id: "extract_electrostatic_diagnostics" },
      { id: "existing_node" },
    ],
    edges: [],
  };
}

export function buildDiagnosticsCreatedNodes() {
  return [
    { id: "node_1" },
    { id: "node_2" },
    { id: "node_3" },
    { id: "node_4" },
    { id: "node_5" },
    { id: "node_6" },
    { id: "node_7" },
  ];
}
