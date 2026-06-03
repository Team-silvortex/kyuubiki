import type { WorkbenchCopy } from "@/components/workbench/workbench-copy";
import type {
  StabilitySummary,
  TrussDiagnostics,
  TrussSuggestion,
} from "@/components/workbench/workbench-defaults";
import type { Truss2dJobInput } from "@/lib/api";

export function summarizeTrussStability(
  model: Truss2dJobInput,
  diagnostics: TrussDiagnostics,
): StabilitySummary {
  const nodeIssueEntries = Object.entries(diagnostics.nodeIssues);
  const issueCount =
    diagnostics.blockingMessages.length +
    nodeIssueEntries.reduce((sum, [, issues]) => sum + issues.length, 0);
  const supportCount = model.nodes.reduce(
    (sum, node) => sum + (node.fix_x ? 1 : 0) + (node.fix_y ? 1 : 0),
    0,
  );
  const structuralScore = Math.max(
    0,
    100 - issueCount * 14 - Math.max(0, model.nodes.length - model.elements.length - 1) * 8,
  );
  const supportBoost = Math.min(12, supportCount * 3);
  const score = Math.max(0, Math.min(100, structuralScore + supportBoost));
  const hotspotNodes = nodeIssueEntries
    .sort((left, right) => right[1].length - left[1].length)
    .slice(0, 3)
    .map(([nodeIndex]) => Number(nodeIndex));

  if (score >= 80) return { score, tone: "good", hotspotNodes };
  if (score >= 55) return { score, tone: "watch", hotspotNodes };
  return { score, tone: "risk", hotspotNodes };
}

export function getTrussBounds(nodes: Array<{ x: number; y: number }>) {
  const xs = nodes.map((node) => node.x);
  const ys = nodes.map((node) => node.y);
  const minX = Math.min(...xs, 0);
  const maxX = Math.max(...xs, 1);
  const minY = Math.min(...ys, 0);
  const maxY = Math.max(...ys, 1);
  return {
    minX,
    maxX,
    minY,
    maxY,
    width: Math.max(maxX - minX, 1),
    height: Math.max(maxY - minY, 1),
  };
}

export function toSvgPoint(
  node: { x: number; y: number },
  bounds: ReturnType<typeof getTrussBounds>,
) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;

  return {
    x: paddingX + ((node.x - bounds.minX) / bounds.width) * usableWidth,
    y: 460 - paddingY - ((node.y - bounds.minY) / bounds.height) * usableHeight,
  };
}

export function fromSvgPoint(
  clientX: number,
  clientY: number,
  rect: DOMRect,
  bounds: ReturnType<typeof getTrussBounds>,
  round: (value: number) => number,
) {
  const paddingX = 120;
  const paddingY = 80;
  const usableWidth = 980 - paddingX * 2;
  const usableHeight = 460 - paddingY * 2;
  const x = ((clientX - rect.left) / rect.width) * 980;
  const y = ((clientY - rect.top) / rect.height) * 460;

  const normalizedX = Math.min(Math.max((x - paddingX) / usableWidth, 0), 1);
  const normalizedY = Math.min(Math.max((460 - paddingY - y) / usableHeight, 0), 1);

  return {
    x: round(bounds.minX + normalizedX * bounds.width),
    y: round(bounds.minY + normalizedY * bounds.height),
  };
}

export function round(value: number): number {
  return Math.round(value * 1000) / 1000;
}

function pushNodeIssue(
  nodeIssues: Record<number, string[]>,
  nodeIndex: number,
  issue: string,
) {
  const issues = nodeIssues[nodeIndex] ?? [];
  if (!issues.includes(issue)) {
    nodeIssues[nodeIndex] = [...issues, issue];
  }
}

export function findNearestConnectableNode(
  model: Truss2dJobInput,
  nodeIndex: number,
): number | null {
  const origin = model.nodes[nodeIndex];
  if (!origin) return null;

  let bestIndex: number | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;

  for (const [candidateIndex, candidate] of model.nodes.entries()) {
    if (candidateIndex === nodeIndex) continue;

    const alreadyLinked = model.elements.some(
      (element) =>
        (element.node_i === nodeIndex && element.node_j === candidateIndex) ||
        (element.node_i === candidateIndex && element.node_j === nodeIndex),
    );
    if (alreadyLinked) continue;

    const distance = Math.hypot(candidate.x - origin.x, candidate.y - origin.y);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestIndex = candidateIndex;
    }
  }

  return bestIndex;
}

export function analyzeTrussModel(
  model: Truss2dJobInput,
  languageCopy: WorkbenchCopy,
  selectedNode: number | null,
): TrussDiagnostics {
  const nodeCount = model.nodes.length;
  const elementCount = model.elements.length;
  const fixedXCount = model.nodes.filter((node) => node.fix_x).length;
  const fixedYCount = model.nodes.filter((node) => node.fix_y).length;
  const constrainedDofs = model.nodes.reduce(
    (count, node) => count + (node.fix_x ? 1 : 0) + (node.fix_y ? 1 : 0),
    0,
  );
  const blockingMessages: string[] = [];
  const nodeIssues: Record<number, string[]> = {};
  const suggestions: TrussSuggestion[] = [];
  const suggestionIds = new Set<string>();
  const connectionCounts = new Array(nodeCount).fill(0);
  const supportTarget = selectedNode ?? 0;

  const addSuggestion = (suggestion: TrussSuggestion) => {
    if (!suggestionIds.has(suggestion.id)) {
      suggestionIds.add(suggestion.id);
      suggestions.push(suggestion);
    }
  };

  if (constrainedDofs < 2) {
    blockingMessages.push(languageCopy.unstableSupport);
    if (nodeCount > 0) pushNodeIssue(nodeIssues, supportTarget, languageCopy.unstableSupport);
  }

  if (fixedXCount === 0) {
    blockingMessages.push(languageCopy.supportXMissing);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.supportXMissing);
      addSuggestion({
        id: `fix-x-${supportTarget}`,
        kind: "fix_support",
        axis: "x",
        nodeIndex: supportTarget,
        label:
          selectedNode !== null
            ? languageCopy.fixCurrentNodeXAction
            : languageCopy.fixNodeXAction,
      });
    }
  }

  if (fixedYCount === 0) {
    blockingMessages.push(languageCopy.supportYMissing);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, supportTarget, languageCopy.supportYMissing);
      addSuggestion({
        id: `fix-y-${supportTarget}`,
        kind: "fix_support",
        axis: "y",
        nodeIndex: supportTarget,
        label:
          selectedNode !== null
            ? languageCopy.fixCurrentNodeYAction
            : languageCopy.fixNodeYAction,
      });
    }
  }

  if (fixedXCount === 0 || fixedYCount === 0) {
    blockingMessages.push(languageCopy.freeRigidBody);
  }

  if (elementCount < nodeCount - 1) {
    blockingMessages.push(languageCopy.underconnected);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, selectedNode ?? 0, languageCopy.underconnected);
    }
  }

  if (elementCount + constrainedDofs < nodeCount * 2) {
    blockingMessages.push(languageCopy.mechanismRisk);
    if (nodeCount > 0) {
      pushNodeIssue(nodeIssues, selectedNode ?? 0, languageCopy.mechanismRisk);
    }
  }

  for (const element of model.elements) {
    if (element.node_i < nodeCount) connectionCounts[element.node_i] += 1;
    if (element.node_j < nodeCount) connectionCounts[element.node_j] += 1;
  }

  const isolatedNodes = connectionCounts.flatMap((count, index) => (count === 0 ? [index] : []));
  for (const nodeIndex of isolatedNodes) {
    pushNodeIssue(nodeIssues, nodeIndex, languageCopy.isolatedNode);
  }
  if (isolatedNodes.length > 0) {
    blockingMessages.push(languageCopy.isolatedNode);
  }

  const connectTarget =
    (selectedNode !== null &&
    (isolatedNodes.includes(selectedNode) || connectionCounts[selectedNode] < 2)
      ? selectedNode
      : isolatedNodes[0] ?? selectedNode) ?? null;

  if (connectTarget !== null && nodeCount > 1) {
    addSuggestion({
      id: `connect-${connectTarget}`,
      kind: "connect_nearest",
      nodeIndex: connectTarget,
      label:
        selectedNode !== null
          ? languageCopy.connectCurrentNodeAction
          : languageCopy.connectNodeAction,
    });
  }

  return {
    blockingMessages: [...new Set(blockingMessages)],
    nodeIssues,
    suggestions,
  };
}

export function renderSupportGlyph(
  point: { x: number; y: number },
  constraints: { fix_x: boolean; fix_y: boolean },
  key: string,
) {
  if (!constraints.fix_x && !constraints.fix_y) return null;
  return (
    <g key={key} className="support-glyph">
      {constraints.fix_y ? (
        <line x1={point.x - 12} y1={point.y + 14} x2={point.x + 12} y2={point.y + 14} />
      ) : null}
      {constraints.fix_x ? (
        <line x1={point.x - 14} y1={point.y - 12} x2={point.x - 14} y2={point.y + 12} />
      ) : null}
    </g>
  );
}

export function renderLoadGlyph(
  point: { x: number; y: number },
  load: { load_x: number; load_y: number },
  key: string,
) {
  if (Math.abs(load.load_x) < 1.0e-9 && Math.abs(load.load_y) < 1.0e-9) return null;

  const scale = 0.01;
  const x2 = point.x + load.load_x * scale;
  const y2 = point.y - load.load_y * scale;
  return (
    <g key={key} className="load-glyph">
      <line x1={point.x} y1={point.y} x2={x2} y2={y2} />
      <circle cx={x2} cy={y2} r={3.5} />
    </g>
  );
}
