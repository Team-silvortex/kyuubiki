function assertPositiveNumber(value, name) {
  if (!Number.isFinite(value) || value <= 0) {
    throw new Error(`${name} must be a positive number`);
  }
}

function zeros(length) {
  return Array.from({ length }, () => 0);
}

function zeroMatrix(size) {
  return Array.from({ length: size }, () => zeros(size));
}

function solveLinearSystem(matrix, vector) {
  const size = vector.length;
  const augmented = matrix.map((row, index) => [...row, vector[index]]);

  for (let pivot = 0; pivot < size; pivot += 1) {
    let maxRow = pivot;

    for (let row = pivot + 1; row < size; row += 1) {
      if (Math.abs(augmented[row][pivot]) > Math.abs(augmented[maxRow][pivot])) {
        maxRow = row;
      }
    }

    if (Math.abs(augmented[maxRow][pivot]) < 1e-12) {
      throw new Error("system is singular");
    }

    [augmented[pivot], augmented[maxRow]] = [augmented[maxRow], augmented[pivot]];

    const pivotValue = augmented[pivot][pivot];
    for (let column = pivot; column <= size; column += 1) {
      augmented[pivot][column] /= pivotValue;
    }

    for (let row = 0; row < size; row += 1) {
      if (row === pivot) {
        continue;
      }

      const factor = augmented[row][pivot];
      for (let column = pivot; column <= size; column += 1) {
        augmented[row][column] -= factor * augmented[pivot][column];
      }
    }
  }

  return augmented.map((row) => row[size]);
}

export function solveBar1D({
  length,
  area,
  youngsModulus,
  elements,
  tipForce,
}) {
  assertPositiveNumber(length, "length");
  assertPositiveNumber(area, "area");
  assertPositiveNumber(youngsModulus, "youngsModulus");

  if (!Number.isInteger(elements) || elements <= 0) {
    throw new Error("elements must be a positive integer");
  }

  if (!Number.isFinite(tipForce)) {
    throw new Error("tipForce must be finite");
  }

  const nodeCount = elements + 1;
  const elementLength = length / elements;
  const stiffnessPerElement = (youngsModulus * area) / elementLength;
  const globalStiffness = zeroMatrix(nodeCount);
  const forceVector = zeros(nodeCount);

  for (let elementIndex = 0; elementIndex < elements; elementIndex += 1) {
    const left = elementIndex;
    const right = elementIndex + 1;

    globalStiffness[left][left] += stiffnessPerElement;
    globalStiffness[left][right] -= stiffnessPerElement;
    globalStiffness[right][left] -= stiffnessPerElement;
    globalStiffness[right][right] += stiffnessPerElement;
  }

  forceVector[nodeCount - 1] = tipForce;

  const reducedStiffness = globalStiffness
    .slice(1)
    .map((row) => row.slice(1));
  const reducedForce = forceVector.slice(1);
  const reducedDisplacement = solveLinearSystem(reducedStiffness, reducedForce);
  const displacements = [0, ...reducedDisplacement];

  const reactionForce = globalStiffness[0].reduce(
    (sum, stiffness, index) => sum + stiffness * displacements[index],
    0
  );

  const nodes = displacements.map((displacement, index) => ({
    index,
    x: (length * index) / elements,
    displacement,
  }));

  const elementResults = Array.from({ length: elements }, (_, index) => {
    const leftNode = nodes[index];
    const rightNode = nodes[index + 1];
    const strain = (rightNode.displacement - leftNode.displacement) / elementLength;
    const stress = youngsModulus * strain;
    const axialForce = stress * area;

    return {
      index,
      x1: leftNode.x,
      x2: rightNode.x,
      strain,
      stress,
      axialForce,
    };
  });

  return {
    input: {
      length,
      area,
      youngsModulus,
      elements,
      tipForce,
    },
    nodes,
    elements: elementResults,
    reactionForce,
    maxDisplacement: Math.max(...nodes.map((node) => Math.abs(node.displacement))),
    maxStress: Math.max(...elementResults.map((element) => Math.abs(element.stress))),
  };
}
