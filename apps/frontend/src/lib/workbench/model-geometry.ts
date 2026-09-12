import type { Truss2dJobInput } from "@/lib/api";

export function getTrussBounds(nodes: Array<{ x: number; y: number }>) {
  let minX = 0, maxX = 1, minY = 0, maxY = 1;
  for (const node of nodes) {
    minX = Math.min(minX, node.x);
    maxX = Math.max(maxX, node.x);
    minY = Math.min(minY, node.y);
    maxY = Math.max(maxY, node.y);
  }
  return {
    minX,
    maxX,
    minY,
    maxY,
    width: Math.max(maxX - minX, 1),
    height: Math.max(maxY - minY, 1),
  };
}

export function findNearestConnectableNode(model: Truss2dJobInput, nodeIndex: number): number | null {
  const origin = model.nodes[nodeIndex];
  if (!origin) return null;

  let bestIndex: number | null = null;
  let bestDistance = Number.POSITIVE_INFINITY;
  const linkedNodes = new Set<number>();
  for (const element of model.elements) {
    if (element.node_i === nodeIndex) linkedNodes.add(element.node_j);
    if (element.node_j === nodeIndex) linkedNodes.add(element.node_i);
  }
  for (const [candidateIndex, candidate] of model.nodes.entries()) {
    if (candidateIndex === nodeIndex || linkedNodes.has(candidateIndex)) continue;
    const distance = Math.hypot(candidate.x - origin.x, candidate.y - origin.y);
    if (distance < bestDistance) {
      bestDistance = distance;
      bestIndex = candidateIndex;
    }
  }
  return bestIndex;
}
