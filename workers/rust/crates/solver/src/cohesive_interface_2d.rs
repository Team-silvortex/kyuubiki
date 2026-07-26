use kyuubiki_protocol::{
    CohesiveInterface2dIntegrationPointResult, CohesiveInterface2dMaterialInput,
    CohesiveInterface2dStepResult, CohesiveTractionRegime, SolveCohesiveInterface2dRequest,
    SolveCohesiveInterface2dResult,
};

use crate::cohesive_interface_1d::validate_id;
use crate::cohesive_law::{CohesiveHistory, CohesiveLaw};

const MAX_HISTORY_STEPS: usize = 4096;
const GEOMETRY_TOLERANCE: f64 = 1.0e-9;
const GAUSS_POINTS: [f64; 2] = [-0.577_350_269_189_625_8, 0.577_350_269_189_625_8];

pub fn solve_cohesive_interface_2d(
    request: &SolveCohesiveInterface2dRequest,
) -> Result<SolveCohesiveInterface2dResult, String> {
    let kernel = validate_request(request)?;

    let mut state = CohesiveInterface2dState::default();
    let mut steps = Vec::with_capacity(request.displacement_history.len());
    for (step, input) in request.displacement_history.iter().enumerate() {
        let displacements = local_displacements(request, &input.nodal_displacements);
        let evaluation = kernel.trial(step, displacements, &state);
        state = evaluation.state;
        steps.push(evaluation.step);
    }

    let max_resultant_traction = steps
        .iter()
        .map(|step| norm(step.global_traction))
        .fold(0.0_f64, f64::max);
    let max_shear_damage = steps
        .iter()
        .map(|step| step.shear_damage)
        .fold(0.0_f64, f64::max);
    let max_normal_damage = steps
        .iter()
        .map(|step| step.normal_damage)
        .fold(0.0_f64, f64::max);

    Ok(SolveCohesiveInterface2dResult {
        input: request.clone(),
        interface_length: kernel.geometry.length,
        interface_area: kernel.geometry.area,
        local_tangent_direction: kernel.geometry.tangent,
        local_normal_direction: kernel.geometry.normal,
        shear_onset_separation: kernel.shear_law.onset_separation(),
        normal_onset_separation: kernel.normal_law.onset_separation(),
        shear_fracture_energy: kernel.shear_law.fracture_energy(),
        normal_fracture_energy: kernel.normal_law.fracture_energy(),
        steps,
        max_resultant_traction,
        max_shear_damage,
        max_normal_damage,
        shear_failed: max_shear_damage >= 1.0,
        normal_failed: max_normal_damage >= 1.0,
    })
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct CohesiveInterface2dKernel {
    geometry: InterfaceGeometry,
    normal_law: CohesiveLaw,
    shear_law: CohesiveLaw,
    compression_stiffness: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CohesiveInterface2dState {
    histories: [IntegrationPointHistory; 2],
}

pub(crate) struct CohesiveInterface2dEvaluation {
    pub(crate) step: CohesiveInterface2dStepResult,
    pub(crate) state: CohesiveInterface2dState,
}

impl CohesiveInterface2dKernel {
    pub(crate) fn new(
        id: &str,
        points: [[f64; 2]; 4],
        thickness: f64,
        material: &CohesiveInterface2dMaterialInput,
    ) -> Result<Self, String> {
        validate_id(id)?;
        if points.iter().flatten().any(|value| !value.is_finite()) {
            return Err("cohesive interface 2d node coordinates must be finite".to_string());
        }
        if !thickness.is_finite() || thickness <= 0.0 {
            return Err("cohesive interface 2d thickness must be finite and positive".to_string());
        }
        if !material.normal_compression_stiffness.is_finite()
            || material.normal_compression_stiffness <= 0.0
        {
            return Err("normal compression stiffness must be finite and positive".to_string());
        }

        let direction = subtract(points[1], points[0]);
        let length = norm(direction);
        if !length.is_finite() || length <= GEOMETRY_TOLERANCE {
            return Err("cohesive interface 2d length is degenerate".to_string());
        }
        let pair_tolerance = GEOMETRY_TOLERANCE * length.max(1.0);
        if norm(subtract(points[2], points[0])) > pair_tolerance
            || norm(subtract(points[3], points[1])) > pair_tolerance
        {
            return Err(
                "cohesive interface 2d upper and lower node pairs must initially coincide"
                    .to_string(),
            );
        }
        let tangent = scale(direction, 1.0 / length);
        Ok(Self {
            geometry: InterfaceGeometry {
                length,
                area: length * thickness,
                tangent,
                normal: [-tangent[1], tangent[0]],
            },
            normal_law: CohesiveLaw::new(
                material.normal_initial_stiffness,
                material.normal_peak_traction,
                material.normal_failure_separation,
                "normal cohesive",
            )?,
            shear_law: CohesiveLaw::new(
                material.shear_initial_stiffness,
                material.shear_peak_traction,
                material.shear_failure_separation,
                "shear cohesive",
            )?,
            compression_stiffness: material.normal_compression_stiffness,
        })
    }

    pub(crate) fn trial(
        &self,
        step: usize,
        displacements: [[f64; 2]; 4],
        committed: &CohesiveInterface2dState,
    ) -> CohesiveInterface2dEvaluation {
        let mut state = *committed;
        let step = evaluate_step(
            step,
            &displacements,
            self.geometry,
            self.normal_law,
            self.shear_law,
            self.compression_stiffness,
            &mut state.histories,
        );
        CohesiveInterface2dEvaluation { step, state }
    }
}

fn evaluate_step(
    step: usize,
    displacements: &[[f64; 2]; 4],
    geometry: InterfaceGeometry,
    normal_law: CohesiveLaw,
    shear_law: CohesiveLaw,
    compression_stiffness: f64,
    histories: &mut [IntegrationPointHistory; 2],
) -> CohesiveInterface2dStepResult {
    let mut nodal_forces = [[0.0; 2]; 4];
    let mut element_tangent = [[0.0; 8]; 8];
    let mut integration_points = Vec::with_capacity(2);
    let mut average_separation = [0.0; 2];
    let mut average_traction = [0.0; 2];
    let mut average_tangent = [0.0; 2];
    let mut average_global_traction = [0.0; 2];

    for (index, &natural_coordinate) in GAUSS_POINTS.iter().enumerate() {
        let shape = shape_functions(natural_coordinate);
        let jump = displacement_jump(displacements, shape);
        let shear_separation = dot(jump, geometry.tangent);
        let normal_separation = dot(jump, geometry.normal);
        let shear = shear_law.evaluate(&mut histories[index].shear, shear_separation, None);
        let normal = normal_law.evaluate(
            &mut histories[index].normal,
            normal_separation,
            Some(compression_stiffness),
        );
        let global_traction = add(
            scale(geometry.tangent, shear.traction),
            scale(geometry.normal, normal.traction),
        );
        let differential_area = 0.5 * geometry.area;
        assemble_force(&mut nodal_forces, shape, global_traction, differential_area);
        assemble_tangent(
            &mut element_tangent,
            shape,
            geometry,
            [shear.tangent, normal.tangent],
            differential_area,
        );

        accumulate(
            &mut average_separation,
            [shear_separation, normal_separation],
        );
        accumulate(&mut average_traction, [shear.traction, normal.traction]);
        accumulate(&mut average_tangent, [shear.tangent, normal.tangent]);
        accumulate(&mut average_global_traction, global_traction);
        integration_points.push(CohesiveInterface2dIntegrationPointResult {
            natural_coordinate,
            local_separation: [shear_separation, normal_separation],
            local_traction: [shear.traction, normal.traction],
            local_tangent: [shear.tangent, normal.tangent],
            shear_damage: shear.damage,
            normal_damage: normal.damage,
            max_shear_separation: shear.max_separation,
            max_normal_opening: normal.max_separation,
            shear_regime: shear.regime,
            normal_regime: normal.regime,
        });
    }

    let shear_summary = dominant_direction(&integration_points, true);
    let normal_summary = dominant_direction(&integration_points, false);
    CohesiveInterface2dStepResult {
        step,
        local_separation: scale(average_separation, 0.5),
        local_traction: scale(average_traction, 0.5),
        local_tangent: scale(average_tangent, 0.5),
        global_traction: scale(average_global_traction, 0.5),
        element_nodal_internal_forces: nodal_forces,
        element_tangent,
        integration_points,
        shear_damage: shear_summary.damage,
        normal_damage: normal_summary.damage,
        max_shear_separation: shear_summary.max_separation,
        max_normal_opening: normal_summary.max_separation,
        shear_regime: shear_summary.regime,
        normal_regime: normal_summary.regime,
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct IntegrationPointHistory {
    shear: CohesiveHistory,
    normal: CohesiveHistory,
}

#[derive(Debug, Clone, Copy)]
struct DirectionSummary {
    damage: f64,
    max_separation: f64,
    regime: CohesiveTractionRegime,
}

fn dominant_direction(
    points: &[CohesiveInterface2dIntegrationPointResult],
    shear: bool,
) -> DirectionSummary {
    points
        .iter()
        .map(|point| {
            if shear {
                DirectionSummary {
                    damage: point.shear_damage,
                    max_separation: point.max_shear_separation,
                    regime: point.shear_regime,
                }
            } else {
                DirectionSummary {
                    damage: point.normal_damage,
                    max_separation: point.max_normal_opening,
                    regime: point.normal_regime,
                }
            }
        })
        .max_by(|left, right| left.max_separation.total_cmp(&right.max_separation))
        .expect("two integration points are retained")
}

fn assemble_force(
    forces: &mut [[f64; 2]; 4],
    shape: [f64; 2],
    traction: [f64; 2],
    differential_area: f64,
) {
    let weights = [-shape[0], -shape[1], shape[0], shape[1]];
    for (force, weight) in forces.iter_mut().zip(weights) {
        *force = add(*force, scale(traction, weight * differential_area));
    }
}

fn assemble_tangent(
    tangent: &mut [[f64; 8]; 8],
    shape: [f64; 2],
    geometry: InterfaceGeometry,
    local_tangent: [f64; 2],
    differential_area: f64,
) {
    let weights = [-shape[0], -shape[1], shape[0], shape[1]];
    let mut strain_jump = [[0.0; 8]; 2];
    for (node, weight) in weights.into_iter().enumerate() {
        strain_jump[0][2 * node] = weight * geometry.tangent[0];
        strain_jump[0][2 * node + 1] = weight * geometry.tangent[1];
        strain_jump[1][2 * node] = weight * geometry.normal[0];
        strain_jump[1][2 * node + 1] = weight * geometry.normal[1];
    }
    for row in 0..8 {
        for column in 0..8 {
            tangent[row][column] += differential_area
                * (local_tangent[0] * strain_jump[0][row] * strain_jump[0][column]
                    + local_tangent[1] * strain_jump[1][row] * strain_jump[1][column]);
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct InterfaceGeometry {
    length: f64,
    area: f64,
    tangent: [f64; 2],
    normal: [f64; 2],
}

fn validate_request(
    request: &SolveCohesiveInterface2dRequest,
) -> Result<CohesiveInterface2dKernel, String> {
    validate_id(&request.element.id)?;
    if request.nodes.len() < 4 {
        return Err("cohesive interface 2d requires at least four nodes".to_string());
    }
    for node in &request.nodes {
        validate_id(&node.id)?;
        if !node.x.is_finite() || !node.y.is_finite() {
            return Err("cohesive interface 2d node coordinates must be finite".to_string());
        }
    }
    let indices = element_indices(request);
    if indices.iter().any(|&index| index >= request.nodes.len()) {
        return Err("cohesive interface 2d node index is out of bounds".to_string());
    }
    let mut unique = indices;
    unique.sort_unstable();
    unique.dedup();
    if unique.len() != 4 {
        return Err("cohesive interface 2d element requires four distinct nodes".to_string());
    }

    validate_history(request)?;
    CohesiveInterface2dKernel::new(
        &request.element.id,
        [
            point(request, request.element.lower_i),
            point(request, request.element.lower_j),
            point(request, request.element.upper_i),
            point(request, request.element.upper_j),
        ],
        request.element.thickness,
        &request.material,
    )
}

fn validate_history(request: &SolveCohesiveInterface2dRequest) -> Result<(), String> {
    if request.displacement_history.is_empty() {
        return Err("cohesive interface 2d displacement_history must not be empty".to_string());
    }
    if request.displacement_history.len() > MAX_HISTORY_STEPS {
        return Err(format!(
            "cohesive interface 2d displacement_history must contain at most {MAX_HISTORY_STEPS} steps"
        ));
    }
    for step in &request.displacement_history {
        if step.nodal_displacements.len() != request.nodes.len() {
            return Err(
                "each cohesive interface 2d displacement step must match the node count"
                    .to_string(),
            );
        }
        if step
            .nodal_displacements
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err("cohesive interface 2d displacements must be finite".to_string());
        }
    }
    Ok(())
}

fn displacement_jump(displacements: &[[f64; 2]; 4], shape: [f64; 2]) -> [f64; 2] {
    let lower = add(
        scale(displacements[0], shape[0]),
        scale(displacements[1], shape[1]),
    );
    let upper = add(
        scale(displacements[2], shape[0]),
        scale(displacements[3], shape[1]),
    );
    subtract(upper, lower)
}

fn local_displacements(
    request: &SolveCohesiveInterface2dRequest,
    displacements: &[[f64; 2]],
) -> [[f64; 2]; 4] {
    [
        displacements[request.element.lower_i],
        displacements[request.element.lower_j],
        displacements[request.element.upper_i],
        displacements[request.element.upper_j],
    ]
}

fn shape_functions(natural_coordinate: f64) -> [f64; 2] {
    [
        0.5 * (1.0 - natural_coordinate),
        0.5 * (1.0 + natural_coordinate),
    ]
}

fn element_indices(request: &SolveCohesiveInterface2dRequest) -> Vec<usize> {
    vec![
        request.element.lower_i,
        request.element.lower_j,
        request.element.upper_i,
        request.element.upper_j,
    ]
}

fn point(request: &SolveCohesiveInterface2dRequest, index: usize) -> [f64; 2] {
    [request.nodes[index].x, request.nodes[index].y]
}

fn accumulate(target: &mut [f64; 2], value: [f64; 2]) {
    target[0] += value[0];
    target[1] += value[1];
}

fn add(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] + right[0], left[1] + right[1]]
}

fn subtract(left: [f64; 2], right: [f64; 2]) -> [f64; 2] {
    [left[0] - right[0], left[1] - right[1]]
}

fn scale(value: [f64; 2], factor: f64) -> [f64; 2] {
    [factor * value[0], factor * value[1]]
}

fn dot(left: [f64; 2], right: [f64; 2]) -> f64 {
    left[0] * right[0] + left[1] * right[1]
}

fn norm(value: [f64; 2]) -> f64 {
    value[0].hypot(value[1])
}
