use std::collections::HashSet;

use kyuubiki_protocol::{
    CohesiveInterfaceMesh2dNodeInput, PlaneNodeInput, PlaneTriangleElementInput,
    PlaneTriangleElementResult,
};

use crate::cohesive_interface_1d::validate_id;
use crate::plane_2d_math::{
    PlaneTriangleComputed, plane_triangle_state, precompute_plane_triangle_element_from_nodes,
};

const MAX_HOST_PLANE_TRIANGLES: usize = 4096;

pub(crate) struct HostPlaneTriangle<'a> {
    input: &'a PlaneTriangleElementInput,
    index: usize,
    dofs: [usize; 6],
    computed: PlaneTriangleComputed,
}

impl<'a> HostPlaneTriangle<'a> {
    pub(crate) fn build_all(
        inputs: &'a [PlaneTriangleElementInput],
        nodes: &[CohesiveInterfaceMesh2dNodeInput],
    ) -> Result<Vec<Self>, String> {
        if inputs.len() > MAX_HOST_PLANE_TRIANGLES {
            return Err(format!(
                "cohesive interface mesh 2d supports at most {MAX_HOST_PLANE_TRIANGLES} host plane triangles"
            ));
        }
        let plane_nodes = nodes
            .iter()
            .map(|node| PlaneNodeInput {
                id: node.id.clone(),
                x: node.x,
                y: node.y,
                fix_x: node.fixed[0],
                fix_y: node.fixed[1],
                load_x: node.load[0],
                load_y: node.load[1],
            })
            .collect::<Vec<_>>();
        let mut ids = HashSet::new();
        inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                validate_id(&input.id)?;
                if !ids.insert(input.id.as_str()) {
                    return Err(format!("duplicate host plane triangle id '{}'", input.id));
                }
                let node_indices = [input.node_i, input.node_j, input.node_k];
                if node_indices.iter().any(|&node| node >= nodes.len()) {
                    return Err(format!(
                        "host plane triangle '{}' node index is out of bounds",
                        input.id
                    ));
                }
                if input.node_i == input.node_j
                    || input.node_i == input.node_k
                    || input.node_j == input.node_k
                {
                    return Err(format!(
                        "host plane triangle '{}' requires distinct nodes",
                        input.id
                    ));
                }
                if !input.thickness.is_finite() || input.thickness <= 0.0 {
                    return Err(format!(
                        "host plane triangle '{}' thickness must be positive",
                        input.id
                    ));
                }
                if !input.youngs_modulus.is_finite() || input.youngs_modulus <= 0.0 {
                    return Err(format!(
                        "host plane triangle '{}' youngs_modulus must be positive",
                        input.id
                    ));
                }
                if !input.poisson_ratio.is_finite()
                    || input.poisson_ratio <= -1.0
                    || input.poisson_ratio >= 0.5
                {
                    return Err(format!(
                        "host plane triangle '{}' poisson_ratio must be between -1.0 and 0.5",
                        input.id
                    ));
                }
                let computed = precompute_plane_triangle_element_from_nodes(&plane_nodes, input)
                    .map_err(|error| format!("host plane triangle '{}': {error}", input.id))?;
                Ok(Self {
                    input,
                    index,
                    dofs: [
                        2 * input.node_i,
                        2 * input.node_i + 1,
                        2 * input.node_j,
                        2 * input.node_j + 1,
                        2 * input.node_k,
                        2 * input.node_k + 1,
                    ],
                    computed,
                })
            })
            .collect()
    }

    pub(crate) fn assemble(
        &self,
        displacements: &[f64],
        internal_forces: &mut [f64],
        tangent: &mut [Vec<f64>],
    ) {
        for row in 0..6 {
            let global_row = self.dofs[row];
            for column in 0..6 {
                let stiffness = self.computed.stiffness[row][column];
                internal_forces[global_row] += stiffness * displacements[self.dofs[column]];
                tangent[global_row][self.dofs[column]] += stiffness;
            }
        }
    }

    pub(crate) fn result(&self, displacements: &[f64]) -> PlaneTriangleElementResult {
        let element_displacements = self.dofs.map(|dof| displacements[dof]);
        let state = plane_triangle_state(&self.computed, &element_displacements);
        PlaneTriangleElementResult {
            index: self.index,
            id: self.input.id.clone(),
            node_i: self.input.node_i,
            node_j: self.input.node_j,
            node_k: self.input.node_k,
            area: self.computed.area,
            strain_x: state.strain[0],
            strain_y: state.strain[1],
            gamma_xy: state.strain[2],
            stress_x: state.stress[0],
            stress_y: state.stress[1],
            tau_xy: state.stress[2],
            principal_stress_1: state.principal_stress_1,
            principal_stress_2: state.principal_stress_2,
            max_in_plane_shear: state.max_in_plane_shear,
            von_mises: state.von_mises,
            strain_energy_density: state.strain_energy_density,
        }
    }
}
