use crate::frame_2d_math::{
    frame_dof_map, frame_local_stiffness, frame_transform, multiply_matrix_vector_6x6,
    transform_frame_stiffness, transpose_6x6,
};
use crate::frame_energy::frame2d_field_energy;
use kyuubiki_protocol::{
    Frame2dElementInput, SolveFrame2dRequest, SolveThermalFrame2dRequest,
    ThermalFrame2dElementInput,
};

pub(super) struct FrameElement<'a> {
    pub id: &'a str,
    pub length: f64,
    pub dofs: [usize; 6],
    pub average_temperature: f64,
    pub thermal_strain: f64,
    pub thermal_curvature: f64,
    transform: [[f64; 6]; 6],
    stiffness: [[f64; 6]; 6],
    equivalent_load: [f64; 6],
    rigidities: [f64; 2],
    area: f64,
    section_modulus: f64,
}

pub(super) struct FrameRecovery {
    pub forces: [f64; 6],
    pub total_strain: f64,
    pub mechanical_strain: f64,
    pub axial_stress: f64,
    pub bending_stress: f64,
    pub combined_stress: f64,
    pub energy: f64,
}

pub(super) fn finite_fields(id: &str, field: &str, values: &[f64]) -> Result<(), String> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("2d frame {id}: {field} is not representable"));
    }
    Ok(())
}

impl<'a> FrameElement<'a> {
    pub fn mechanical(
        request: &SolveFrame2dRequest,
        e: &'a Frame2dElementInput,
    ) -> Result<Self, String> {
        let a = &request.nodes[e.node_i];
        let b = &request.nodes[e.node_j];
        Self::new(
            &e.id,
            [e.node_i, e.node_j],
            [b.x - a.x, b.y - a.y],
            [
                e.youngs_modulus,
                e.area,
                e.moment_of_inertia,
                e.section_modulus,
            ],
            [0.0; 3],
        )
    }

    pub fn thermal(
        request: &SolveThermalFrame2dRequest,
        e: &'a ThermalFrame2dElementInput,
    ) -> Result<Self, String> {
        let a = &request.nodes[e.node_i];
        let b = &request.nodes[e.node_j];
        let temperature = a.temperature_delta.midpoint(b.temperature_delta);
        Self::new(
            &e.id,
            [e.node_i, e.node_j],
            [b.x - a.x, b.y - a.y],
            [
                e.youngs_modulus,
                e.area,
                e.moment_of_inertia,
                e.section_modulus,
            ],
            [
                temperature,
                e.thermal_expansion * temperature,
                e.thermal_expansion * e.temperature_gradient_y / e.section_depth,
            ],
        )
    }

    fn new(
        id: &'a str,
        nodes: [usize; 2],
        delta: [f64; 2],
        properties: [f64; 4],
        thermal: [f64; 3],
    ) -> Result<Self, String> {
        let [youngs_modulus, area, inertia, section_modulus] = properties;
        let length = delta[0].hypot(delta[1]);
        let rigidities = [youngs_modulus * area, youngs_modulus * inertia];
        let stiffness = frame_local_stiffness(area, youngs_modulus, inertia, length);
        if rigidities.iter().any(|v| !v.is_finite() || *v <= 0.0)
            || (0..6).any(|i| stiffness[i][i] <= 0.0)
        {
            return Err(format!("2d frame {id}: stiffness is not representable"));
        }
        for row in &stiffness {
            finite_fields(id, "stiffness", row)?;
        }
        finite_fields(id, "thermal state", &thermal)?;
        let axial = rigidities[0] * thermal[1];
        let moment = rigidities[1] * thermal[2];
        let equivalent_load = [-axial, 0.0, -moment, axial, 0.0, moment];
        finite_fields(id, "equivalent load", &equivalent_load)?;
        Ok(Self {
            id,
            length,
            dofs: frame_dof_map(nodes[0], nodes[1]),
            average_temperature: thermal[0],
            thermal_strain: thermal[1],
            thermal_curvature: thermal[2],
            transform: frame_transform(delta[0] / length, delta[1] / length),
            stiffness,
            equivalent_load,
            rigidities,
            area,
            section_modulus,
        })
    }

    pub fn global_system(&self) -> Result<([[f64; 6]; 6], [f64; 6]), String> {
        let stiffness = transform_frame_stiffness(&self.stiffness, &self.transform);
        let load =
            multiply_matrix_vector_6x6(&transpose_6x6(&self.transform), &self.equivalent_load);
        for row in &stiffness {
            finite_fields(self.id, "global stiffness", row)?;
        }
        finite_fields(self.id, "global equivalent load", &load)?;
        Ok((stiffness, load))
    }

    pub fn recover(&self, displacement: &[f64]) -> Result<FrameRecovery, String> {
        let local =
            multiply_matrix_vector_6x6(&self.transform, &self.dofs.map(|dof| displacement[dof]));
        finite_fields(self.id, "local displacements", &local)?;
        let elastic = multiply_matrix_vector_6x6(&self.stiffness, &local);
        let forces = std::array::from_fn(|i| elastic[i] - self.equivalent_load[i]);
        finite_fields(self.id, "end forces", &forces)?;
        let total_strain = (local[3] - local[0]) / self.length;
        let mechanical_strain = total_strain - self.thermal_strain;
        finite_fields(
            self.id,
            "recovered strain",
            &[total_strain, mechanical_strain],
        )?;
        let energy = frame2d_field_energy(
            self.id,
            self.rigidities,
            self.length,
            &local,
            mechanical_strain,
            self.thermal_curvature,
        )?;
        let axial_stress = forces[0].abs().max(forces[3].abs()) / self.area;
        let bending_stress = forces[2].abs().max(forces[5].abs()) / self.section_modulus;
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
