import {
  buildProjectedBounds, lineInsideViewport, pointInsideViewport, projectTruss3dPoint,
  rotatePoint, type CameraState, type DisplayTruss3dElement, type DisplayTruss3dNode, type ProjectionMode,
} from "./workbench-viewport-core";
import { computeTruss3dSceneBounds, resolveDeformationScale } from "./workbench-truss3d-webgl-scene";
import type { ViewportRenderStrategy } from "./workbench-render-diagnostics";

type Box = { minX: number; minY: number; minZ: number; maxX: number; maxY: number; maxZ: number };
type Cell = Box & { start: number; end: number; left: number; right: number };
type ReadBox = (index: number, out: Float64Array) => boolean;
type Tree = { order: Uint32Array; cells: Cell[]; read: ReadBox };
export type Truss3dLodView = {
  camera: CameraState;
  projected3d: ReturnType<typeof buildProjectedBounds>;
  projectionMode: ProjectionMode;
};
export type Truss3dLodBudget = { nodes: number; elements: number };

export function truss3dLodBudget(strategy: ViewportRenderStrategy): Truss3dLodBudget {
  // Even Full is a bounded high-detail view, not permission to exhaust the WebView.
  if (strategy === "full") return { nodes: 2400, elements: 4800 };
  if (strategy === "focus") return { nodes: 600, elements: 1200 };
  return { nodes: 1200, elements: 2400 };
}

export function truss3dBoxCorners(box: Box) {
  return Array.from({ length: 8 }, (_, i) => ({
    x: i & 1 ? box.maxX : box.minX,
    y: i & 2 ? box.maxY : box.minY,
    z: i & 4 ? box.maxZ : box.minZ,
  }));
}

function buildTree(length: number, read: ReadBox): Tree {
  const order = new Uint32Array(length), scratch = new Float64Array(6), cells: Cell[] = [];
  let count = 0;
  for (let i = 0; i < length; i++) if (read(i, scratch)) order[count++] = i;
  const visit = (start: number, end: number, depth: number): number => {
    let minX = Infinity, minY = Infinity, minZ = Infinity, maxX = -Infinity, maxY = -Infinity, maxZ = -Infinity;
    for (let j = start; j < end; j++) {
      read(order[j], scratch);
      minX = Math.min(minX, scratch[0]); minY = Math.min(minY, scratch[1]); minZ = Math.min(minZ, scratch[2]);
      maxX = Math.max(maxX, scratch[3]); maxY = Math.max(maxY, scratch[4]); maxZ = Math.max(maxZ, scratch[5]);
    }
    const index = cells.length;
    const cell: Cell = { minX, minY, minZ, maxX, maxY, maxZ, start, end, left: -1, right: -1 };
    cells.push(cell);
    if (end - start <= 32 || depth >= 24) return index;
    const spans = [maxX - minX, maxY - minY, maxZ - minZ];
    const axis = spans[1] > spans[0] ? (spans[2] > spans[1] ? 2 : 1) : (spans[2] > spans[0] ? 2 : 0);
    const split = ([minX, minY, minZ][axis] + [maxX, maxY, maxZ][axis]) / 2;
    let pivot = start;
    for (let j = start; j < end; j++) {
      read(order[j], scratch);
      if ((scratch[axis] + scratch[axis + 3]) / 2 < split) {
        const previous = order[pivot]; order[pivot++] = order[j]; order[j] = previous;
      }
    }
    // Coincident nodes and long crossing members still get a balanced, finite tree.
    if (pivot - start < (end - start) / 16 || end - pivot < (end - start) / 16) pivot = (start + end) >>> 1;
    cell.left = visit(start, pivot, depth + 1);
    cell.right = visit(pivot, end, depth + 1);
    return index;
  };
  if (count) visit(0, count, 0);
  return { order: order.subarray(0, count), cells, read };
}

export function buildTruss3dLodIndex(nodes: DisplayTruss3dNode[], elements: DisplayTruss3dElement[],
  hiddenMaterials: readonly string[] = [], results = false) {
  const bounds = computeTruss3dSceneBounds(nodes);
  const deformationScale = resolveDeformationScale(nodes, results);
  const hidden = new Set(hiddenMaterials);
  // Result windows carry global IDs; only dense modeling arrays can use IDs as array offsets.
  const nodePositions = nodes.some((node, i) => node.index !== i) ? new Map(nodes.map((node, i) => [node.index, i])) : null;
  const elementPositions = elements.some((element, i) => element.index !== i) ? new Map(elements.map((element, i) => [element.index, i])) : null;
  const nodePosition = (id: number) => nodePositions ? nodePositions.get(id) ?? -1 : id;
  const elementPosition = (id: number) => elementPositions ? elementPositions.get(id) ?? -1 : id;
  const nodeAt = (id: number) => nodes[nodePosition(id)];
  const elementAt = (id: number) => elements[elementPosition(id)];
  const nodeBox: ReadBox = (index, out) => {
    const n = nodes[index];
    if (!n || !Number.isFinite(n.x) || !Number.isFinite(n.y) || !Number.isFinite(n.z)) return false;
    const x = n.x + (results && Number.isFinite(n.ux) ? n.ux * deformationScale : 0);
    const y = n.y + (results && Number.isFinite(n.uy) ? n.uy * deformationScale : 0);
    const z = n.z + (results && Number.isFinite(n.uz) ? n.uz * deformationScale : 0);
    out[0] = Math.min(n.x, x); out[1] = Math.min(n.y, y); out[2] = Math.min(n.z, z);
    out[3] = Math.max(n.x, x); out[4] = Math.max(n.y, y); out[5] = Math.max(n.z, z);
    return true;
  };
  const endBox = new Float64Array(6);
  const elementBox: ReadBox = (index, out) => {
    const e = elements[index];
    if (!e || (e.material_id && hidden.has(e.material_id)) || !nodeBox(nodePosition(e.node_i), out) || !nodeBox(nodePosition(e.node_j), endBox)) return false;
    for (let axis = 0; axis < 3; axis++) {
      out[axis] = Math.min(out[axis], endBox[axis]);
      out[axis + 3] = Math.max(out[axis + 3], endBox[axis + 3]);
    }
    return true;
  };
  return {
    nodes, elements, bounds, deformationScale, results, nodeAt, elementAt, nodePosition, elementPosition,
    corners: truss3dBoxCorners(bounds),
    nodeTree: buildTree(nodes.length, nodeBox), elementTree: buildTree(elements.length, elementBox),
  };
}

export type Truss3dLodIndex = ReturnType<typeof buildTruss3dLodIndex>;

function screenBox(box: Box, view: Truss3dLodView) {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const point of truss3dBoxCorners(box)) {
    if (view.projectionMode === "persp") {
      const depth = rotatePoint(point, view.camera).y - view.projected3d.depthCenter;
      // A box crossing the near clamp cannot be safely culled by its corners alone.
      if (depth <= -view.projected3d.depthDistance * 0.99) return { visible: true, size: 1e12 };
    }
    const p = projectTruss3dPoint(point, view.projected3d, view.camera, view.projectionMode);
    minX = Math.min(minX, p.x); minY = Math.min(minY, p.y);
    maxX = Math.max(maxX, p.x); maxY = Math.max(maxY, p.y);
  }
  return { visible: lineInsideViewport({ x: minX, y: minY }, { x: maxX, y: maxY }, 36),
    size: Math.max(maxX - minX, maxY - minY) };
}

type QueueEntry = { cell: Cell; size: number };
function pushHeap(heap: QueueEntry[], entry: QueueEntry) {
  let i = heap.length;
  heap.push(entry);
  while (i > 0) {
    const p = (i - 1) >>> 1;
    if (heap[p].size >= entry.size) break;
    heap[i] = heap[p]; i = p;
  }
  heap[i] = entry;
}
function popHeap(heap: QueueEntry[]) {
  const first = heap[0], last = heap.pop()!;
  if (!heap.length) return first;
  let i = 0;
  while (i * 2 + 1 < heap.length) {
    let child = i * 2 + 1;
    if (child + 1 < heap.length && heap[child + 1].size > heap[child].size) child++;
    if (heap[child].size <= last.size) break;
    heap[i] = heap[child]; i = child;
  }
  heap[i] = last;
  return first;
}

function queryTree(tree: Tree, view: Truss3dLodView, budget: number, pins: readonly number[], hit: (i: number) => boolean) {
  const selected = new Set<number>(), scratch = new Float64Array(6);
  for (let j = 0; j < pins.length && selected.size < budget; j++) {
    const i = pins[j];
    if (Number.isInteger(i) && i >= 0 && tree.read(i, scratch)) selected.add(i);
  }
  const queue: QueueEntry[] = [], terminal: QueueEntry[] = [];
  let visits = 0, tested = 0;
  const visitLimit = Math.max(128, budget * 4), cellLimit = Math.max(1, Math.floor(budget / 8));
  const add = (cell: Cell) => {
    visits++;
    const projected = screenBox(cell, view);
    if (projected.visible) pushHeap(queue, { cell, size: projected.size });
  };
  if (tree.cells.length) add(tree.cells[0]);
  while (queue.length && queue.length + terminal.length < cellLimit && visits + 2 <= visitLimit) {
    const entry = popHeap(queue);
    if (entry.cell.left < 0 || entry.size <= 12) terminal.push(entry);
    else {
      add(tree.cells[entry.cell.left]); add(tree.cells[entry.cell.right]);
    }
  }
  const frontier = [...terminal, ...queue].sort((a, b) => a.cell.start - b.cell.start);
  let candidates = 0;
  for (const { cell } of frontier) candidates += cell.end - cell.start;
  for (let f = 0; f < frontier.length && selected.size < budget; f++) {
    const { cell } = frontier[f], count = cell.end - cell.start;
    const quota = candidates <= budget - pins.length ? count : Math.min(count, Math.ceil((budget - selected.size) / (frontier.length - f)));
    const attempts = Math.min(count, quota * 4 + 8);
    let accepted = 0;
    // Stride across each spatial cell, not across the file's first N records.
    for (let j = 0; j < attempts && accepted < quota && selected.size < budget; j++) {
      const slot = quota === 1 ? Math.floor(count / 2) : Math.floor((j % quota) * (count - 1) / (quota - 1) + Math.floor(j / quota)) % count;
      const i = tree.order[cell.start + slot];
      tested++;
      if (!selected.has(i) && hit(i)) { selected.add(i); accepted++; }
    }
  }
  return { indices: [...selected].sort((a, b) => a - b), visits, tested,
    limited: candidates > selected.size || visits + 2 > visitLimit };
}

export function queryTruss3dLod(index: Truss3dLodIndex, view: Truss3dLodView,
  budget: Truss3dLodBudget, selected: { node?: number | null; element?: number | null; nodes?: readonly number[]; draft?: readonly number[] } = {}) {
  const project = (node: DisplayTruss3dNode, deformed: boolean) => projectTruss3dPoint(deformed ? {
    x: node.x + node.ux * index.deformationScale, y: node.y + node.uy * index.deformationScale,
    z: node.z + node.uz * index.deformationScale,
  } : node, view.projected3d, view.camera, view.projectionMode);
  const pinNodes: number[] = [];
  if (selected.node != null) pinNodes.push(selected.node);
  if (selected.element != null) {
    const e = index.elementAt(selected.element);
    if (e) pinNodes.push(e.node_i, e.node_j);
  }
  pinNodes.push(...(selected.draft ?? []).slice(0, budget.nodes), ...(selected.nodes ?? []).slice(0, budget.nodes));
  const nodes = queryTree(index.nodeTree, view, budget.nodes, pinNodes.map(index.nodePosition), (i) =>
    pointInsideViewport(project(index.nodes[i], false), 36) || (index.results && pointInsideViewport(project(index.nodes[i], true), 36)));
  const elements = queryTree(index.elementTree, view, budget.elements, selected.element == null ? [] : [index.elementPosition(selected.element)], (i) => {
    const e = index.elements[i], a = index.nodeAt(e.node_i), b = index.nodeAt(e.node_j);
    return lineInsideViewport(project(a, false), project(b, false), 36) ||
      (index.results && lineInsideViewport(project(a, true), project(b, true), 36));
  });
  return {
    nodes: nodes.indices.map((i) => index.nodes[i]), elements: elements.indices.map((i) => index.elements[i]),
    limited: nodes.limited || elements.limited, visits: nodes.visits + elements.visits,
    tested: nodes.tested + elements.tested, budget,
  };
}
