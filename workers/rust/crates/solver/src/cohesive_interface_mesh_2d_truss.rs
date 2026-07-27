use std::collections::HashSet;

use kyuubiki_protocol::{CohesiveInterfaceMesh2dNodeInput, TrussElementInput, TrussElementResult};

use crate::cohesive_interface_1d::validate_id;

const MAX_HOST_TRUSSES: usize = 4096;
const MIN_LENGTH: f64 = 1.0e-12;

pub(crate) struct HostTruss<'a> {
    input: &'a TrussElementInput,
    index: usize,
    dofs: [usize; 4],
    length: f64,
    direction: [f64; 2],
    axial_stiffness: f64,
}

impl<'a> HostTruss<'a> {
    pub(crate) fn build_all(
        inputs: &'a [TrussElementInput],
        nodes: &[CohesiveInterfaceMesh2dNodeInput],
    ) -> Result<Vec<Self>, String> {
        if inputs.len() > MAX_HOST_TRUSSES {
            return Err(format!(
                "cohesive interface mesh 2d supports at most {MAX_HOST_TRUSSES} host trusses"
            ));
        }
        let mut ids = HashSet::new();
        inputs
            .iter()
            .enumerate()
            .map(|(index, input)| {
                validate_id(&input.id)?;
                if !ids.insert(input.id.as_str()) {
                    return Err(format!("duplicate host truss id '{}'", input.id));
                }
                if input.node_i >= nodes.len() || input.node_j >= nodes.len() {
                    return Err(format!(
                        "host truss '{}' node index is out of bounds",
                        input.id
                    ));
                }
                if input.node_i == input.node_j {
                    return Err(format!("host truss '{}' requires distinct nodes", input.id));
                }
                if !input.area.is_finite() || input.area <= 0.0 {
                    return Err(format!("host truss '{}' area must be positive", input.id));
                }
                if !input.youngs_modulus.is_finite() || input.youngs_modulus <= 0.0 {
                    return Err(format!(
                        "host truss '{}' youngs_modulus must be positive",
                        input.id
                    ));
                }
                let dx = nodes[input.node_j].x - nodes[input.node_i].x;
                let dy = nodes[input.node_j].y - nodes[input.node_i].y;
                let length = dx.hypot(dy);
                if !length.is_finite() || length <= MIN_LENGTH {
                    return Err(format!("host truss '{}' length must be positive", input.id));
                }
                Ok(Self {
                    input,
                    index,
                    dofs: [
                        2 * input.node_i,
                        2 * input.node_i + 1,
                        2 * input.node_j,
                        2 * input.node_j + 1,
                    ],
                    length,
                    direction: [dx / length, dy / length],
                    axial_stiffness: input.youngs_modulus * input.area / length,
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
        let [c, s] = self.direction;
        let extension = (displacements[self.dofs[2]] - displacements[self.dofs[0]]) * c
            + (displacements[self.dofs[3]] - displacements[self.dofs[1]]) * s;
        let axial_force = self.axial_stiffness * extension;
        let direction = [-c, -s, c, s];
        for row in 0..4 {
            internal_forces[self.dofs[row]] += axial_force * direction[row];
            for column in 0..4 {
                tangent[self.dofs[row]][self.dofs[column]] +=
                    self.axial_stiffness * direction[row] * direction[column];
            }
        }
    }

    pub(crate) fn result(&self, displacements: &[f64]) -> TrussElementResult {
        let [c, s] = self.direction;
        let extension = (displacements[self.dofs[2]] - displacements[self.dofs[0]]) * c
            + (displacements[self.dofs[3]] - displacements[self.dofs[1]]) * s;
        let strain = extension / self.length;
        let stress = self.input.youngs_modulus * strain;
        TrussElementResult {
            index: self.index,
            id: self.input.id.clone(),
            node_i: self.input.node_i,
            node_j: self.input.node_j,
            length: self.length,
            strain,
            stress,
            axial_force: stress * self.input.area,
            strain_energy_density: 0.5 * stress * strain,
        }
    }
}
