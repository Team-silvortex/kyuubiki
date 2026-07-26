use std::collections::HashSet;

use kyuubiki_protocol::{
    CohesiveInterfaceMesh2dConnectorSpringInput, CohesiveInterfaceMesh2dConnectorSpringResult,
};

use crate::cohesive_interface_1d::validate_id;

const MAX_CONNECTOR_SPRINGS: usize = 4096;

pub(crate) struct ConnectorSpring<'a> {
    input: &'a CohesiveInterfaceMesh2dConnectorSpringInput,
    dofs: [[usize; 2]; 2],
}

impl<'a> ConnectorSpring<'a> {
    pub(crate) fn build_all(
        inputs: &'a [CohesiveInterfaceMesh2dConnectorSpringInput],
        node_count: usize,
    ) -> Result<Vec<Self>, String> {
        if inputs.len() > MAX_CONNECTOR_SPRINGS {
            return Err(format!(
                "cohesive interface mesh 2d supports at most {MAX_CONNECTOR_SPRINGS} connector springs"
            ));
        }
        let mut ids = HashSet::new();
        inputs
            .iter()
            .map(|input| {
                validate_id(&input.id)?;
                if !ids.insert(input.id.as_str()) {
                    return Err(format!("duplicate connector spring id '{}'", input.id));
                }
                if input.node_i >= node_count || input.node_j >= node_count {
                    return Err(format!(
                        "connector spring '{}' node index is out of bounds",
                        input.id
                    ));
                }
                if input.node_i == input.node_j {
                    return Err(format!(
                        "connector spring '{}' requires two distinct nodes",
                        input.id
                    ));
                }
                if input
                    .stiffness
                    .iter()
                    .any(|value| !value.is_finite() || *value < 0.0)
                    || input.stiffness == [0.0, 0.0]
                {
                    return Err(format!(
                        "connector spring '{}' stiffness must be finite, non-negative, and non-zero",
                        input.id
                    ));
                }
                Ok(Self {
                    input,
                    dofs: [
                        [2 * input.node_i, 2 * input.node_i + 1],
                        [2 * input.node_j, 2 * input.node_j + 1],
                    ],
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
        for axis in 0..2 {
            let dof_i = self.dofs[0][axis];
            let dof_j = self.dofs[1][axis];
            let stiffness = self.input.stiffness[axis];
            let force = stiffness * (displacements[dof_j] - displacements[dof_i]);
            internal_forces[dof_i] -= force;
            internal_forces[dof_j] += force;
            tangent[dof_i][dof_i] += stiffness;
            tangent[dof_i][dof_j] -= stiffness;
            tangent[dof_j][dof_i] -= stiffness;
            tangent[dof_j][dof_j] += stiffness;
        }
    }

    pub(crate) fn result(
        &self,
        displacements: &[f64],
    ) -> CohesiveInterfaceMesh2dConnectorSpringResult {
        let relative_displacement = [
            displacements[self.dofs[1][0]] - displacements[self.dofs[0][0]],
            displacements[self.dofs[1][1]] - displacements[self.dofs[0][1]],
        ];
        let force = [
            self.input.stiffness[0] * relative_displacement[0],
            self.input.stiffness[1] * relative_displacement[1],
        ];
        CohesiveInterfaceMesh2dConnectorSpringResult {
            id: self.input.id.clone(),
            node_i: self.input.node_i,
            node_j: self.input.node_j,
            relative_displacement,
            force,
            strain_energy: 0.5
                * (force[0] * relative_displacement[0] + force[1] * relative_displacement[1]),
        }
    }
}
