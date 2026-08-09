use kyuubiki_protocol::{
    CohesiveInterface3dIntegrationPointResult, CohesiveInterface3dMaterialInput,
};

use crate::cohesive_interface_1d::validate_id;
use crate::cohesive_law::{CohesiveHistory, CohesiveLaw};

const GEOMETRY_TOLERANCE: f64 = 1.0e-9;
const INTEGRATION_POINTS: [[f64; 3]; 3] = [
    [2.0 / 3.0, 1.0 / 6.0, 1.0 / 6.0],
    [1.0 / 6.0, 2.0 / 3.0, 1.0 / 6.0],
    [1.0 / 6.0, 1.0 / 6.0, 2.0 / 3.0],
];

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CohesiveInterface3dState {
    histories: [IntegrationPointHistory; 3],
}

#[derive(Debug, Clone)]
pub(crate) struct CohesiveInterface3dStep {
    pub(crate) local_separation: [f64; 3],
    pub(crate) local_traction: [f64; 3],
    pub(crate) global_traction: [f64; 3],
    pub(crate) nodal_internal_forces: [[f64; 3]; 6],
    pub(crate) tangent: [[f64; 18]; 18],
    pub(crate) integration_points: Vec<CohesiveInterface3dIntegrationPointResult>,
    pub(crate) max_tangential_damage: f64,
    pub(crate) max_normal_damage: f64,
}

pub(crate) struct CohesiveInterface3dEvaluation {
    pub(crate) step: CohesiveInterface3dStep,
    pub(crate) state: CohesiveInterface3dState,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CohesiveInterface3dKernel {
    area: f64,
    basis: [[f64; 3]; 3],
    shear_law: CohesiveLaw,
    normal_law: CohesiveLaw,
    compression_stiffness: f64,
}

impl CohesiveInterface3dKernel {
    pub(crate) fn new(
        id: &str,
        points: [[f64; 3]; 6],
        material: &CohesiveInterface3dMaterialInput,
    ) -> Result<Self, String> {
        validate_id(id)?;
        if points.iter().flatten().any(|value| !value.is_finite()) {
            return Err("cohesive interface 3d coordinates must be finite".to_string());
        }
        Self::validate_material(material)?;

        let edge_ab = subtract(points[1], points[0]);
        let edge_ac = subtract(points[2], points[0]);
        let cross_product = cross(edge_ab, edge_ac);
        let double_area = norm(cross_product);
        if !double_area.is_finite() || double_area <= GEOMETRY_TOLERANCE {
            return Err("cohesive interface 3d triangle is degenerate".to_string());
        }
        let pair_tolerance = GEOMETRY_TOLERANCE * double_area.sqrt().max(1.0);
        for pair in 0..3 {
            if norm(subtract(points[pair + 3], points[pair])) > pair_tolerance {
                return Err(
                    "cohesive interface 3d upper and lower node pairs must initially coincide"
                        .to_string(),
                );
            }
        }
        let tangent_1 = scale(edge_ab, 1.0 / norm(edge_ab));
        let normal = scale(cross_product, 1.0 / double_area);
        let tangent_2 = cross(normal, tangent_1);

        Ok(Self {
            area: 0.5 * double_area,
            basis: [tangent_1, tangent_2, normal],
            shear_law: CohesiveLaw::new(
                material.shear_initial_stiffness,
                material.shear_peak_traction,
                material.shear_failure_separation,
                "tangential cohesive",
            )?,
            normal_law: CohesiveLaw::new(
                material.normal_initial_stiffness,
                material.normal_peak_traction,
                material.normal_failure_separation,
                "normal cohesive",
            )?,
            compression_stiffness: material.normal_compression_stiffness,
        })
    }

    pub(crate) fn validate_material(
        material: &CohesiveInterface3dMaterialInput,
    ) -> Result<(), String> {
        if !material.normal_compression_stiffness.is_finite()
            || material.normal_compression_stiffness <= 0.0
        {
            return Err("normal compression stiffness must be finite and positive".to_string());
        }
        CohesiveLaw::new(
            material.shear_initial_stiffness,
            material.shear_peak_traction,
            material.shear_failure_separation,
            "tangential cohesive",
        )?;
        CohesiveLaw::new(
            material.normal_initial_stiffness,
            material.normal_peak_traction,
            material.normal_failure_separation,
            "normal cohesive",
        )?;
        Ok(())
    }

    pub(crate) fn area(self) -> f64 {
        self.area
    }

    pub(crate) fn basis(self) -> [[f64; 3]; 3] {
        self.basis
    }

    pub(crate) fn trial(
        self,
        displacements: [[f64; 3]; 6],
        committed: &CohesiveInterface3dState,
    ) -> CohesiveInterface3dEvaluation {
        let mut state = *committed;
        let mut nodal_internal_forces = [[0.0; 3]; 6];
        let mut tangent = [[0.0; 18]; 18];
        let mut integration_points = Vec::with_capacity(3);
        let mut average_separation = [0.0; 3];
        let mut average_traction = [0.0; 3];
        let mut average_global_traction = [0.0; 3];

        for (point_index, shape) in INTEGRATION_POINTS.iter().copied().enumerate() {
            let jump = displacement_jump(&displacements, shape);
            let separation = self.basis.map(|direction| dot(jump, direction));
            let shear_1 = self.shear_law.evaluate(
                &mut state.histories[point_index].tangential[0],
                separation[0],
                None,
            );
            let shear_2 = self.shear_law.evaluate(
                &mut state.histories[point_index].tangential[1],
                separation[1],
                None,
            );
            let normal = self.normal_law.evaluate(
                &mut state.histories[point_index].normal,
                separation[2],
                Some(self.compression_stiffness),
            );
            let responses = [shear_1, shear_2, normal];
            let local_traction = responses.map(|response| response.traction);
            let local_tangent = responses.map(|response| response.tangent);
            let global_traction = local_to_global(local_traction, self.basis);
            let weight = self.area / 3.0;
            assemble_force(&mut nodal_internal_forces, shape, global_traction, weight);
            assemble_tangent(&mut tangent, shape, self.basis, local_tangent, weight);
            accumulate(&mut average_separation, separation);
            accumulate(&mut average_traction, local_traction);
            accumulate(&mut average_global_traction, global_traction);
            integration_points.push(CohesiveInterface3dIntegrationPointResult {
                barycentric_coordinates: shape,
                local_separation: separation,
                local_traction,
                local_tangent,
                tangential_damage: [shear_1.damage, shear_2.damage],
                normal_damage: normal.damage,
                max_tangential_separation: [shear_1.max_separation, shear_2.max_separation],
                max_normal_opening: normal.max_separation,
                regimes: [shear_1.regime, shear_2.regime, normal.regime],
            });
        }

        let max_tangential_damage = integration_points
            .iter()
            .flat_map(|point| point.tangential_damage)
            .fold(0.0_f64, f64::max);
        let max_normal_damage = integration_points
            .iter()
            .map(|point| point.normal_damage)
            .fold(0.0_f64, f64::max);
        CohesiveInterface3dEvaluation {
            step: CohesiveInterface3dStep {
                local_separation: scale(average_separation, 1.0 / 3.0),
                local_traction: scale(average_traction, 1.0 / 3.0),
                global_traction: scale(average_global_traction, 1.0 / 3.0),
                nodal_internal_forces,
                tangent,
                integration_points,
                max_tangential_damage,
                max_normal_damage,
            },
            state,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct IntegrationPointHistory {
    tangential: [CohesiveHistory; 2],
    normal: CohesiveHistory,
}

fn displacement_jump(displacements: &[[f64; 3]; 6], shape: [f64; 3]) -> [f64; 3] {
    let mut jump = [0.0; 3];
    for node in 0..3 {
        for (axis, component) in jump.iter_mut().enumerate() {
            *component += shape[node] * (displacements[node + 3][axis] - displacements[node][axis]);
        }
    }
    jump
}

fn assemble_force(forces: &mut [[f64; 3]; 6], shape: [f64; 3], traction: [f64; 3], weight: f64) {
    for (node, force) in forces.iter_mut().enumerate() {
        let sign = if node < 3 { -1.0 } else { 1.0 };
        for (component, traction_component) in force.iter_mut().zip(traction) {
            *component += sign * shape[node % 3] * traction_component * weight;
        }
    }
}

fn assemble_tangent(
    tangent: &mut [[f64; 18]; 18],
    shape: [f64; 3],
    basis: [[f64; 3]; 3],
    local_tangent: [f64; 3],
    weight: f64,
) {
    for (row, row_entries) in tangent.iter_mut().enumerate() {
        let row_node = row / 3;
        let row_sign = if row_node < 3 { -1.0 } else { 1.0 };
        for (column, entry) in row_entries.iter_mut().enumerate() {
            let column_node = column / 3;
            let column_sign = if column_node < 3 { -1.0 } else { 1.0 };
            let directional = (0..3)
                .map(|direction| {
                    basis[direction][row % 3]
                        * local_tangent[direction]
                        * basis[direction][column % 3]
                })
                .sum::<f64>();
            *entry += row_sign
                * column_sign
                * shape[row_node % 3]
                * shape[column_node % 3]
                * directional
                * weight;
        }
    }
}

fn local_to_global(local: [f64; 3], basis: [[f64; 3]; 3]) -> [f64; 3] {
    std::array::from_fn(|axis| {
        (0..3)
            .map(|direction| local[direction] * basis[direction][axis])
            .sum()
    })
}

fn accumulate(target: &mut [f64; 3], value: [f64; 3]) {
    for axis in 0..3 {
        target[axis] += value[axis];
    }
}

fn subtract(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    std::array::from_fn(|axis| left[axis] - right[axis])
}

fn scale(vector: [f64; 3], factor: f64) -> [f64; 3] {
    vector.map(|value| value * factor)
}

fn dot(left: [f64; 3], right: [f64; 3]) -> f64 {
    (0..3).map(|axis| left[axis] * right[axis]).sum()
}

fn cross(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [
        left[1] * right[2] - left[2] * right[1],
        left[2] * right[0] - left[0] * right[2],
        left[0] * right[1] - left[1] * right[0],
    ]
}

fn norm(vector: [f64; 3]) -> f64 {
    dot(vector, vector).sqrt()
}
