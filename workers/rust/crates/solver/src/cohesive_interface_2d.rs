use kyuubiki_protocol::{
    CohesiveInterface2dStepResult, SolveCohesiveInterface2dRequest, SolveCohesiveInterface2dResult,
};

use crate::cohesive_interface_1d::validate_id;
use crate::cohesive_law::{CohesiveHistory, CohesiveLaw};

const MAX_HISTORY_STEPS: usize = 4096;
const GEOMETRY_TOLERANCE: f64 = 1.0e-9;

pub fn solve_cohesive_interface_2d(
    request: &SolveCohesiveInterface2dRequest,
) -> Result<SolveCohesiveInterface2dResult, String> {
    let geometry = validate_request(request)?;
    let normal_law = CohesiveLaw::new(
        request.material.normal_initial_stiffness,
        request.material.normal_peak_traction,
        request.material.normal_failure_separation,
        "normal cohesive",
    )?;
    let shear_law = CohesiveLaw::new(
        request.material.shear_initial_stiffness,
        request.material.shear_peak_traction,
        request.material.shear_failure_separation,
        "shear cohesive",
    )?;

    let mut normal_history = CohesiveHistory::default();
    let mut shear_history = CohesiveHistory::default();
    let mut steps = Vec::with_capacity(request.displacement_history.len());
    for (step, input) in request.displacement_history.iter().enumerate() {
        let jump = displacement_jump(request, &input.nodal_displacements);
        let shear_separation = dot(jump, geometry.tangent);
        let normal_separation = dot(jump, geometry.normal);
        let shear = shear_law.evaluate(&mut shear_history, shear_separation, None);
        let normal = normal_law.evaluate(
            &mut normal_history,
            normal_separation,
            Some(request.material.normal_compression_stiffness),
        );
        let global_traction = add(
            scale(geometry.tangent, shear.traction),
            scale(geometry.normal, normal.traction),
        );
        let half_force = scale(global_traction, 0.5 * geometry.area);

        steps.push(CohesiveInterface2dStepResult {
            step,
            local_separation: [shear_separation, normal_separation],
            local_traction: [shear.traction, normal.traction],
            local_tangent: [shear.tangent, normal.tangent],
            global_traction,
            element_nodal_internal_forces: [
                scale(half_force, -1.0),
                scale(half_force, -1.0),
                half_force,
                half_force,
            ],
            shear_damage: shear.damage,
            normal_damage: normal.damage,
            max_shear_separation: shear.max_separation,
            max_normal_opening: normal.max_separation,
            shear_regime: shear.regime,
            normal_regime: normal.regime,
        });
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
        interface_length: geometry.length,
        interface_area: geometry.area,
        local_tangent_direction: geometry.tangent,
        local_normal_direction: geometry.normal,
        shear_onset_separation: shear_law.onset_separation(),
        normal_onset_separation: normal_law.onset_separation(),
        shear_fracture_energy: shear_law.fracture_energy(),
        normal_fracture_energy: normal_law.fracture_energy(),
        steps,
        max_resultant_traction,
        max_shear_damage,
        max_normal_damage,
        shear_failed: max_shear_damage >= 1.0,
        normal_failed: max_normal_damage >= 1.0,
    })
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
) -> Result<InterfaceGeometry, String> {
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
    if !request.element.thickness.is_finite() || request.element.thickness <= 0.0 {
        return Err("cohesive interface 2d thickness must be finite and positive".to_string());
    }
    if !request.material.normal_compression_stiffness.is_finite()
        || request.material.normal_compression_stiffness <= 0.0
    {
        return Err("normal compression stiffness must be finite and positive".to_string());
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

    let lower_i = point(request, request.element.lower_i);
    let lower_j = point(request, request.element.lower_j);
    let upper_i = point(request, request.element.upper_i);
    let upper_j = point(request, request.element.upper_j);
    let direction = subtract(lower_j, lower_i);
    let length = norm(direction);
    if !length.is_finite() || length <= GEOMETRY_TOLERANCE {
        return Err("cohesive interface 2d length is degenerate".to_string());
    }
    let pair_tolerance = GEOMETRY_TOLERANCE * length.max(1.0);
    if norm(subtract(upper_i, lower_i)) > pair_tolerance
        || norm(subtract(upper_j, lower_j)) > pair_tolerance
    {
        return Err(
            "cohesive interface 2d upper and lower node pairs must initially coincide".to_string(),
        );
    }

    validate_history(request)?;
    let tangent = scale(direction, 1.0 / length);
    Ok(InterfaceGeometry {
        length,
        area: length * request.element.thickness,
        tangent,
        normal: [-tangent[1], tangent[0]],
    })
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

fn displacement_jump(
    request: &SolveCohesiveInterface2dRequest,
    displacements: &[[f64; 2]],
) -> [f64; 2] {
    let lower = scale(
        add(
            displacements[request.element.lower_i],
            displacements[request.element.lower_j],
        ),
        0.5,
    );
    let upper = scale(
        add(
            displacements[request.element.upper_i],
            displacements[request.element.upper_j],
        ),
        0.5,
    );
    subtract(upper, lower)
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
