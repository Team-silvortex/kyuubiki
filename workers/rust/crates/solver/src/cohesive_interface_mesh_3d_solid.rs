use kyuubiki_protocol::{
    CohesiveInterfaceMesh3dNodeInput, SolidTetra3dElementInput, SolidTetra3dElementResult,
};

use crate::cohesive_interface_1d::validate_id;
use crate::linear_algebra::MatrixAssembler;
use crate::solid_tetra_3d_element::{SolidTetra3dElementKernel, element_dof_map};

pub(crate) struct HostTetra<'a> {
    input: &'a SolidTetra3dElementInput,
    index: usize,
    dofs: [usize; 12],
    kernel: SolidTetra3dElementKernel,
}

impl<'a> HostTetra<'a> {
    pub(crate) fn build_all(
        inputs: &'a [SolidTetra3dElementInput],
        nodes: &[CohesiveInterfaceMesh3dNodeInput],
    ) -> Result<Vec<Self>, String> {
        inputs
            .iter()
            .enumerate()
            .map(|(index, input)| Self::new(index, input, nodes))
            .collect()
    }

    fn new(
        index: usize,
        input: &'a SolidTetra3dElementInput,
        nodes: &[CohesiveInterfaceMesh3dNodeInput],
    ) -> Result<Self, String> {
        validate_id(&input.id)?;
        let node_indices = [input.node_a, input.node_b, input.node_c, input.node_d];
        if node_indices.iter().any(|&node| node >= nodes.len()) {
            return Err(format!(
                "host tetra '{}' references a missing cohesive mesh node",
                input.id
            ));
        }
        for left in 0..4 {
            for right in (left + 1)..4 {
                if node_indices[left] == node_indices[right] {
                    return Err(format!(
                        "host tetra '{}' requires four distinct nodes",
                        input.id
                    ));
                }
            }
        }
        let points = node_indices.map(|node| [nodes[node].x, nodes[node].y, nodes[node].z]);
        Ok(Self {
            input,
            index,
            dofs: element_dof_map(input),
            kernel: SolidTetra3dElementKernel::new(points, input)?,
        })
    }

    pub(crate) fn assemble<M: MatrixAssembler + ?Sized>(
        &self,
        displacements: &[f64],
        internal_forces: &mut [f64],
        tangent: &mut M,
    ) {
        self.kernel
            .assemble(&self.dofs, displacements, internal_forces, tangent);
    }

    pub(crate) fn result(&self, displacements: &[f64]) -> SolidTetra3dElementResult {
        self.kernel
            .result(self.index, self.input, &self.dofs, displacements)
    }
}
