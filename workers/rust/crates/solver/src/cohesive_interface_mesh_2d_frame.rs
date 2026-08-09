use std::collections::HashSet;

use kyuubiki_protocol::{
    CohesiveInterfaceMesh2dNodeInput, Frame2dElementInput, Frame2dElementResult,
};

use crate::cohesive_interface_1d::validate_id;
use crate::frame_2d_math::{
    frame_local_stiffness, frame_transform, multiply_matrix_vector_6x6, transform_frame_stiffness,
};
use crate::frame_energy::frame_strain_energy_6;
use crate::linear_algebra::{MatrixAssembler, add_at};

const MAX_HOST_FRAMES: usize = 4096;
const MIN_LENGTH: f64 = 1.0e-12;

pub(crate) struct HostFrame<'a> {
    input: &'a Frame2dElementInput,
    index: usize,
    dofs: [usize; 6],
    length: f64,
    local_stiffness: [[f64; 6]; 6],
    transform: [[f64; 6]; 6],
    global_stiffness: [[f64; 6]; 6],
}

impl<'a> HostFrame<'a> {
    pub(crate) fn build_all(
        inputs: &'a [Frame2dElementInput],
        nodes: &[CohesiveInterfaceMesh2dNodeInput],
        rotation_offset: usize,
    ) -> Result<Vec<Self>, String> {
        if inputs.len() > MAX_HOST_FRAMES {
            return Err(format!(
                "cohesive interface mesh 2d supports at most {MAX_HOST_FRAMES} host frames"
            ));
        }
        let mut ids = HashSet::new();
        inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                validate_frame(input, nodes, &mut ids)?;
                let dx = nodes[input.node_j].x - nodes[input.node_i].x;
                let dy = nodes[input.node_j].y - nodes[input.node_i].y;
                let length = dx.hypot(dy);
                if !length.is_finite() || length <= MIN_LENGTH {
                    return Err(format!("host frame '{}' length must be positive", input.id));
                }
                let transform = frame_transform(dx / length, dy / length);
                let local_stiffness = frame_local_stiffness(
                    input.area,
                    input.youngs_modulus,
                    input.moment_of_inertia,
                    length,
                );
                let global_stiffness = transform_frame_stiffness(&local_stiffness, &transform);
                Ok(Self {
                    input,
                    index,
                    dofs: [
                        2 * input.node_i,
                        2 * input.node_i + 1,
                        rotation_offset + input.node_i,
                        2 * input.node_j,
                        2 * input.node_j + 1,
                        rotation_offset + input.node_j,
                    ],
                    length,
                    local_stiffness,
                    transform,
                    global_stiffness,
                })
            })
            .collect()
    }

    pub(crate) fn assemble<M: MatrixAssembler + ?Sized>(
        &self,
        displacements: &[f64],
        internal_forces: &mut [f64],
        tangent: &mut M,
    ) {
        let element_displacements = self.global_displacements(displacements);
        let element_forces =
            multiply_matrix_vector_6x6(&self.global_stiffness, &element_displacements);
        for row in 0..6 {
            internal_forces[self.dofs[row]] += element_forces[row];
            for column in 0..6 {
                add_at(
                    tangent,
                    self.dofs[row],
                    self.dofs[column],
                    self.global_stiffness[row][column],
                );
            }
        }
    }

    pub(crate) fn result(&self, displacements: &[f64]) -> Frame2dElementResult {
        let global_displacements = self.global_displacements(displacements);
        let local_displacements =
            multiply_matrix_vector_6x6(&self.transform, &global_displacements);
        let local_forces = multiply_matrix_vector_6x6(&self.local_stiffness, &local_displacements);
        let axial_stress = local_forces[0].abs().max(local_forces[3].abs()) / self.input.area;
        let max_bending_stress =
            local_forces[2].abs().max(local_forces[5].abs()) / self.input.section_modulus;
        Frame2dElementResult {
            index: self.index,
            id: self.input.id.clone(),
            node_i: self.input.node_i,
            node_j: self.input.node_j,
            length: self.length,
            axial_force_i: local_forces[0],
            shear_force_i: local_forces[1],
            moment_i: local_forces[2],
            axial_force_j: local_forces[3],
            shear_force_j: local_forces[4],
            moment_j: local_forces[5],
            axial_stress,
            max_bending_stress,
            max_combined_stress: axial_stress + max_bending_stress,
            strain_energy: frame_strain_energy_6(&local_forces, &local_displacements),
        }
    }

    fn global_displacements(&self, displacements: &[f64]) -> [f64; 6] {
        self.dofs.map(|dof| displacements[dof])
    }
}

fn validate_frame<'a>(
    input: &'a Frame2dElementInput,
    nodes: &[CohesiveInterfaceMesh2dNodeInput],
    ids: &mut HashSet<&'a str>,
) -> Result<(), String> {
    validate_id(&input.id)?;
    if !ids.insert(input.id.as_str()) {
        return Err(format!("duplicate host frame id '{}'", input.id));
    }
    if input.node_i >= nodes.len() || input.node_j >= nodes.len() {
        return Err(format!(
            "host frame '{}' node index is out of bounds",
            input.id
        ));
    }
    if input.node_i == input.node_j {
        return Err(format!("host frame '{}' requires distinct nodes", input.id));
    }
    for (name, value) in [
        ("area", input.area),
        ("youngs_modulus", input.youngs_modulus),
        ("moment_of_inertia", input.moment_of_inertia),
        ("section_modulus", input.section_modulus),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("host frame '{}' {name} must be positive", input.id));
        }
    }
    Ok(())
}
