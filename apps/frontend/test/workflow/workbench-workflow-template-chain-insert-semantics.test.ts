import test from "node:test";
import assert from "node:assert/strict";

import { applyTemplateChainNodeSemantics } from "../../src/components/workbench/workflow/workbench-workflow-template-chain-insert-semantics.ts";
import {
  DIAGNOSTICS_BUNDLE_GUARD_REPORT_TEMPLATE_CHAIN,
  PEAK_DIAGNOSTICS_BUNDLE_REPORT_TEMPLATE_CHAIN,
} from "../../src/components/workbench/workflow/workbench-workflow-template-chain-diagnostics-preset.ts";
import {
  buildDiagnosticsCreatedNodes,
  buildDiagnosticsTemplateChainGraph,
} from "../support/workflow-template-chain-fixtures.ts";

test("applyTemplateChainNodeSemantics assigns semantic ids for diagnostics chains", () => {
  const graph = buildDiagnosticsTemplateChainGraph();
  const createdNodes = buildDiagnosticsCreatedNodes();
  graph.nodes = [...graph.nodes, ...createdNodes];

  applyTemplateChainNodeSemantics(
    graph as never,
    DIAGNOSTICS_BUNDLE_GUARD_REPORT_TEMPLATE_CHAIN as never,
    createdNodes as never,
  );

  assert.deepEqual(
    createdNodes.map((node) => node.id),
    [
      "extract_electrostatic_diagnostics_2",
      "extract_thermal_diagnostics",
      "extract_thermo_diagnostics",
      "compose_diagnostics_bundle",
      "evaluate_diagnostics_guard",
      "compose_diagnostics_report",
      "export_diagnostics_markdown",
    ],
  );
});

test("applyTemplateChainNodeSemantics leaves non-diagnostics chains untouched", () => {
  const graph = { id: "workflow.other", nodes: [{ id: "node_a" }], edges: [] };
  const createdNodes = [{ id: "node_1" }, { id: "node_2" }];

  applyTemplateChainNodeSemantics(
    graph as never,
    { id: "condition_branch_merge_export" } as never,
    createdNodes as never,
  );

  assert.deepEqual(createdNodes.map((node) => node.id), ["node_1", "node_2"]);
});

test("applyTemplateChainNodeSemantics assigns semantic ids for peak diagnostics chains", () => {
  const graph = buildDiagnosticsTemplateChainGraph();
  const createdNodes = buildDiagnosticsCreatedNodes();
  graph.nodes = [...graph.nodes, ...createdNodes];

  applyTemplateChainNodeSemantics(
    graph as never,
    PEAK_DIAGNOSTICS_BUNDLE_REPORT_TEMPLATE_CHAIN as never,
    createdNodes as never,
  );

  assert.deepEqual(
    createdNodes.map((node) => node.id),
    [
      "extract_electrostatic_peak",
      "extract_thermal_peak",
      "extract_thermo_peak",
      "compose_diagnostics_bundle",
      "evaluate_diagnostics_guard",
      "compose_diagnostics_report",
      "export_diagnostics_markdown",
    ],
  );
});
