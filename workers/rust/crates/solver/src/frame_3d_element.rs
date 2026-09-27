use crate::frame_3d_math::{
    frame3d_dof_map, frame3d_local_stiffness, frame3d_rotation_with_local_y, frame3d_transform,
    multiply_matrix_vector_12x12, transform_frame3d_stiffness, transpose_12x12,
};
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::solver_control::{SolverStage, checkpoint, checkpoint_chunk};
use crate::solver_postprocess::fold_results;
use kyuubiki_protocol::{
    Frame3dElementInput, SolveFrame3dRequest, SolveThermalFrame3dRequest,
    ThermalFrame3dElementInput,
};

pub(super) struct FrameElement<'a> {
    pub id: &'a str,
    pub length: f64,
    pub temperature: f64,
    pub thermal_strain: f64,
    pub thermal_curvatures: [f64; 2],
    dofs: [usize; 12],
    transform: [[f64; 12]; 12],
    stiffness: [[f64; 12]; 12],
    thermal_load: [f64; 12],
    rigidities: [f64; 4],
    sections: [f64; 3],
}

pub(super) struct FrameRecovery {
    pub forces: [f64; 12],
    pub total_strain: f64,
    pub mechanical_strain: f64,
    pub axial_stress: f64,
    pub bending_stress: f64,
    pub combined_stress: f64,
    pub energy: f64,
}

pub(super) fn finite_fields(id: &str, field: &str, values: &[f64]) -> Result<(), String> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("3d frame {id}: {field} is not representable"));
    }
    Ok(())
}

pub(super) fn norm3(values: [f64; 3]) -> f64 {
    values[0].hypot(values[1]).hypot(values[2])
}

pub(super) fn elastic_energy(
    id: &str,
    strain: f64,
    rigidity: f64,
    length: f64,
) -> Result<f64, String> {
    let amplitude = strain.abs() * rigidity.sqrt();
    let mut factors = [amplitude, amplitude, length / 2.0];
    factors.sort_unstable_by(f64::total_cmp);
    let energy = (factors[0] * factors[2]) * factors[1];
    if !energy.is_finite() || (strain != 0.0 && energy == 0.0) {
        return Err(format!("3d frame {id}: strain energy is not representable"));
    }
    Ok(energy)
}

pub(super) fn total_energy(values: impl ExactSizeIterator<Item = f64>) -> Result<f64, String> {
    let total = fold_results(SolverStage::ResultTotals, values, 0.0, |sum, value| {
        sum + value
    })?;
    finite_fields("summary", "total strain energy", &[total])?;
    Ok(total)
}

pub(super) fn validate_matrix(matrix: &SparseMatrix, count: usize) -> Result<(), String> {
    checkpoint(SolverStage::SparseValidateMatrix, 0)?;
    for row in 0..count {
        if matrix
            .row_entries(row)
            .iter()
            .any(|(_, value)| !value.is_finite())
        {
            return Err("3d frame assembled stiffness is not representable".into());
        }
        checkpoint_chunk(SolverStage::SparseValidateMatrix, row + 1, count)?;
    }
    Ok(())
}

impl<'a> FrameElement<'a> {
    pub fn mechanical(
        request: &SolveFrame3dRequest,
        e: &'a Frame3dElementInput,
    ) -> Result<Self, String> {
        let a = &request.nodes[e.node_i];
        let b = &request.nodes[e.node_j];
        Self::new(
            &e.id,
            [e.node_i, e.node_j],
            [b.x - a.x, b.y - a.y, b.z - a.z],
            e.local_y_axis,
            [
                e.area,
                e.youngs_modulus,
                e.shear_modulus,
                e.torsion_constant,
                e.moment_of_inertia_y,
                e.moment_of_inertia_z,
                e.section_modulus_y,
                e.section_modulus_z,
            ],
            [0.0; 4],
        )
    }

    pub fn thermal(
        request: &SolveThermalFrame3dRequest,
        e: &'a ThermalFrame3dElementInput,
    ) -> Result<Self, String> {
        let a = &request.nodes[e.node_i];
        let b = &request.nodes[e.node_j];
        let temperature = a.temperature_delta.midpoint(b.temperature_delta);
        Self::new(
            &e.id,
            [e.node_i, e.node_j],
            [b.x - a.x, b.y - a.y, b.z - a.z],
            e.local_y_axis,
            [
                e.area,
                e.youngs_modulus,
                e.shear_modulus,
                e.torsion_constant,
                e.moment_of_inertia_y,
                e.moment_of_inertia_z,
                e.section_modulus_y,
                e.section_modulus_z,
            ],
            [
                temperature,
                e.thermal_expansion * temperature,
                e.thermal_expansion * e.temperature_gradient_y / e.section_depth_y,
                e.thermal_expansion * e.temperature_gradient_z / e.section_depth_z,
            ],
        )
    }

    fn new(
        id: &'a str,
        nodes: [usize; 2],
        delta: [f64; 3],
        local_y: Option<[f64; 3]>,
        properties: [f64; 8],
        thermal: [f64; 4],
    ) -> Result<Self, String> {
        let [area, e, g, j, iy, iz, zy, zz] = properties;
        let length = norm3(delta);
        let rotation =
            frame3d_rotation_with_local_y(delta[0], delta[1], delta[2], length, local_y)?;
        let rigidities = [e * area, g * j, e * iy, e * iz];
        let stiffness = frame3d_local_stiffness(area, e, g, j, iy, iz, length);
        if rigidities.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || (0..12).any(|i| stiffness[i][i] <= 0.0)
        {
            return Err(format!("3d frame {id}: stiffness is not representable"));
        }
        for row in &stiffness {
            finite_fields(id, "stiffness", row)?;
        }
        finite_fields(id, "thermal state", &thermal)?;
        let n = rigidities[0] * thermal[1];
        let my = rigidities[2] * thermal[3];
        let mz = rigidities[3] * thermal[2];
        let thermal_load = [-n, 0.0, 0.0, 0.0, -my, -mz, n, 0.0, 0.0, 0.0, my, mz];
        finite_fields(id, "equivalent load", &thermal_load)?;
        Ok(Self {
            id,
            length,
            temperature: thermal[0],
            thermal_strain: thermal[1],
            thermal_curvatures: [thermal[2], thermal[3]],
            dofs: frame3d_dof_map(nodes[0], nodes[1]),
            transform: frame3d_transform(&rotation),
            stiffness,
            thermal_load,
            rigidities,
            sections: [area, zy, zz],
        })
    }

    pub fn assemble(&self, matrix: &mut SparseMatrix, forces: &mut [f64]) -> Result<(), String> {
        let global = transform_frame3d_stiffness(&self.stiffness, &self.transform);
        let load =
            multiply_matrix_vector_12x12(&transpose_12x12(&self.transform), &self.thermal_load);
        finite_fields(self.id, "global equivalent load", &load)?;
        for (row, &global_row) in self.dofs.iter().enumerate() {
            finite_fields(self.id, "global stiffness", &global[row])?;
            forces[global_row] += load[row];
            finite_fields(self.id, "assembled load", &[forces[global_row]])?;
            for (column, &global_column) in self.dofs.iter().enumerate() {
                add_at(matrix, global_row, global_column, global[row][column]);
            }
        }
        Ok(())
    }

    pub fn recover(&self, displacements: &[f64]) -> Result<FrameRecovery, String> {
        let d = multiply_matrix_vector_12x12(&self.transform, &self.dofs.map(|i| displacements[i]));
        finite_fields(self.id, "local displacements", &d)?;
        let elastic = multiply_matrix_vector_12x12(&self.stiffness, &d);
        let forces: [f64; 12] = std::array::from_fn(|i| elastic[i] - self.thermal_load[i]);
        finite_fields(self.id, "end forces", &forces)?;
        let total_strain = (d[6] - d[0]) / self.length;
        let mechanical_strain = total_strain - self.thermal_strain;
        finite_fields(
            self.id,
            "recovered strain",
            &[total_strain, mechanical_strain],
        )?;
        let twist = (d[9] - d[3]) / self.length;
        let cy = (d[10] - d[4]) / self.length - self.thermal_curvatures[1];
        let cz = (d[11] - d[5]) / self.length - self.thermal_curvatures[0];
        // Local rotations satisfy ry=-dw/dx, rz=dv/dx. Each Hermite curvature
        // contributes its squared mean and variance, without thermal-work subtraction.
        let sy = (d[7] - d[1]) / self.length;
        let sz = (d[8] - d[2]) / self.length;
        let vy = 3.0_f64.sqrt() * ((d[4] + sz) + (d[10] + sz)) / self.length;
        let vz = 3.0_f64.sqrt() * ((d[5] - sy) + (d[11] - sy)) / self.length;
        let [ea, gj, eiy, eiz] = self.rigidities;
        let mut energy = 0.0;
        for (strain, rigidity) in [
            (mechanical_strain, ea),
            (twist, gj),
            (cy, eiy),
            (vy, eiy),
            (cz, eiz),
            (vz, eiz),
        ] {
            energy += elastic_energy(self.id, strain, rigidity, self.length)?;
        }
        finite_fields(self.id, "strain energy", &[energy])?;
        let axial_stress = forces[0].abs().max(forces[6].abs()) / self.sections[0];
        let bending_stress = forces[4].abs().max(forces[10].abs()) / self.sections[1]
            + forces[5].abs().max(forces[11].abs()) / self.sections[2];
        let combined_stress = axial_stress + bending_stress;
        finite_fields(
            self.id,
            "stress",
            &[axial_stress, bending_stress, combined_stress],
        )?;
        Ok(FrameRecovery {
            forces,
            total_strain,
            mechanical_strain,
            axial_stress,
            bending_stress,
            combined_stress,
            energy,
        })
    }
}
