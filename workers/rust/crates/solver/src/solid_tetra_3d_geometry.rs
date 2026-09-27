pub(crate) struct TetraGeometry {
    edges: [[f64; 3]; 3],
    first_cofactor: [f64; 3],
    determinant: f64,
    scale: f64,
    pub(crate) volume: f64,
}

impl TetraGeometry {
    pub(crate) fn new(points: [[f64; 3]; 4], id: &str) -> Result<Self, String> {
        let edges: [[f64; 3]; 3] = std::array::from_fn(|i| {
            std::array::from_fn(|axis| points[i + 1][axis] - points[0][axis])
        });
        if edges.iter().flatten().any(|value| !value.is_finite()) {
            return Err(format!(
                "solid tetra element {id} edge differences are not representable"
            ));
        }
        let scale = edges
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        if scale == 0.0 {
            return Err(format!("solid tetra element {id} has zero volume"));
        }
        // Translation is removed before normalization. Absolute-coordinate
        // cofactors can cancel even when all local edges are represented exactly.
        let edges = edges.map(|edge| edge.map(|value| value / scale));
        let first_cofactor = cross(edges[1], edges[2]);
        let determinant = (0..3)
            .map(|axis| edges[0][axis] * first_cofactor[axis])
            .sum::<f64>();
        if determinant == 0.0 {
            return Err(format!("solid tetra element {id} has zero volume"));
        }
        // Divide before restoring units: six times a valid volume may overflow.
        let volume = ((determinant.abs() / 6.0 * scale) * scale) * scale;
        if !volume.is_finite() || volume <= 0.0 {
            return Err(format!(
                "solid tetra element {id} volume is not representable"
            ));
        }
        Ok(Self {
            edges,
            first_cofactor,
            determinant,
            scale,
            volume,
        })
    }

    pub(crate) fn mean_ratio_quality(&self) -> f64 {
        let [a, b, c] = self.edges;
        let edges = [a, b, c, subtract(b, a), subtract(c, a), subtract(c, b)];
        let squared_sum = edges
            .iter()
            .flatten()
            .map(|value| value * value)
            .sum::<f64>();
        12.0 * (self.determinant.abs() / 2.0).powf(2.0 / 3.0) / squared_sum
    }

    pub(crate) fn strain_matrix(&self) -> [[f64; 12]; 6] {
        let [a, b, c] = self.edges;
        let gradients = [self.first_cofactor, cross(c, a), cross(a, b)]
            .map(|cofactor| cofactor.map(|value| (value / self.determinant) / self.scale));
        let first = std::array::from_fn(|axis| {
            -(gradients[0][axis] + gradients[1][axis] + gradients[2][axis])
        });
        let mut matrix = [[0.0; 12]; 6];
        for (node, [x, y, z]) in [first, gradients[0], gradients[1], gradients[2]]
            .into_iter()
            .enumerate()
        {
            let offset = node * 3;
            matrix[0][offset] = x;
            matrix[1][offset + 1] = y;
            matrix[2][offset + 2] = z;
            matrix[3][offset] = y;
            matrix[3][offset + 1] = x;
            matrix[4][offset + 1] = z;
            matrix[4][offset + 2] = y;
            matrix[5][offset] = z;
            matrix[5][offset + 2] = x;
        }
        matrix
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn subtract(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|axis| a[axis] - b[axis])
}
