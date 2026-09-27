use kyuubiki_protocol::{
    Beam1dElementInput, SolveBeam1dRequest, SolveThermalBeam1dRequest, ThermalBeam1dElementInput,
};

pub(super) struct BeamElement<'a> {
    pub id: &'a str,
    pub length: f64,
    pub dofs: [usize; 4],
    pub stiffness: [[f64; 4]; 4],
    pub load: [f64; 4],
    pub thermal_curvature: f64,
    flexural: f64,
    distributed_load: f64,
    reversed: bool,
}

pub(super) struct BeamRecovery {
    pub forces: [f64; 4],
    pub max_moment: f64,
    pub max_stress: f64,
    pub energy: f64,
}

pub(super) fn finite_fields(id: &str, field: &str, values: &[f64]) -> Result<(), String> {
    if values.iter().any(|value| !value.is_finite()) {
        return Err(format!("1d beam {id}: {field} is not representable"));
    }
    Ok(())
}

impl<'a> BeamElement<'a> {
    pub fn mechanical(
        request: &SolveBeam1dRequest,
        element: &'a Beam1dElementInput,
    ) -> Result<Self, String> {
        Self::new(
            &element.id,
            [element.node_i, element.node_j],
            [
                request.nodes[element.node_i].x,
                request.nodes[element.node_j].x,
            ],
            element.youngs_modulus * element.moment_of_inertia,
            element.distributed_load_y,
            0.0,
        )
    }

    pub fn thermal(
        request: &SolveThermalBeam1dRequest,
        element: &'a ThermalBeam1dElementInput,
    ) -> Result<Self, String> {
        Self::new(
            &element.id,
            [element.node_i, element.node_j],
            [
                request.nodes[element.node_i].x,
                request.nodes[element.node_j].x,
            ],
            element.youngs_modulus * element.moment_of_inertia,
            element.distributed_load_y,
            element.thermal_expansion * element.temperature_gradient_y / element.section_depth,
        )
    }

    fn new(
        id: &'a str,
        mut nodes: [usize; 2],
        x: [f64; 2],
        flexural: f64,
        distributed_load: f64,
        thermal_curvature: f64,
    ) -> Result<Self, String> {
        let length = (x[1] - x[0]).abs();
        let reversed = x[0] > x[1];
        // Rotations are global dw/dx, not local to the supplied connectivity order.
        if reversed {
            nodes.swap(0, 1);
        }
        let bending = flexural / length;
        let coupling = bending / length;
        let transverse = coupling / length;
        let a = 12.0 * transverse;
        let b = 6.0 * coupling;
        let c = 4.0 * bending;
        let d = 2.0 * bending;
        if [flexural, a, b, c, d]
            .iter()
            .any(|value| !value.is_finite() || *value <= 0.0)
        {
            return Err(format!("1d beam {id}: stiffness is not representable"));
        }
        let shear = distributed_load * (length / 2.0);
        let moment = (distributed_load * (length / 12.0)) * length;
        let thermal_moment = flexural * thermal_curvature;
        finite_fields(
            id,
            "equivalent load",
            &[shear, moment, thermal_curvature, thermal_moment],
        )?;
        let load = [
            shear,
            moment - thermal_moment,
            shear,
            -moment + thermal_moment,
        ];
        finite_fields(id, "equivalent load", &load)?;
        Ok(Self {
            id,
            length,
            dofs: [
                nodes[0] * 2,
                nodes[0] * 2 + 1,
                nodes[1] * 2,
                nodes[1] * 2 + 1,
            ],
            stiffness: [[a, b, -a, b], [b, c, -b, d], [-a, -b, a, -b], [b, d, -b, c]],
            load,
            thermal_curvature,
            flexural,
            distributed_load,
            reversed,
        })
    }

    pub fn recover(
        &self,
        displacements: &[f64],
        section_modulus: f64,
    ) -> Result<BeamRecovery, String> {
        let local = self.dofs.map(|dof| displacements[dof]);
        finite_fields(self.id, "displacements", &local)?;
        let mut forces: [f64; 4] = std::array::from_fn(|row| {
            self.stiffness[row]
                .iter()
                .zip(local)
                .map(|(k, u)| k * u)
                .sum::<f64>()
                - self.load[row]
        });
        finite_fields(self.id, "end forces", &forces)?;

        // M(t) interpolates end moments plus the uniform-load particular solution.
        // This recovers the interior field, not just the cubic nodal interpolant.
        let quadratic = (self.distributed_load * (self.length / 2.0)) * self.length;
        let coefficients = [-forces[1], forces[3] + forces[1] - quadratic, quadratic];
        finite_fields(self.id, "moment field", &coefficients)?;
        let scale = coefficients
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max);
        let (max_moment, energy) = if scale == 0.0 {
            (0.0, 0.0)
        } else {
            let [a, b, c] = coefficients.map(|value| value / scale);
            let moment = |t: f64| a + t * (b + t * c);
            let mut peak = a.abs().max(moment(1.0).abs());
            if c != 0.0 {
                let vertex = -0.5 * (b / c);
                if vertex > 0.0 && vertex < 1.0 {
                    peak = peak.max(moment(vertex).abs());
                }
            }
            let max_moment = scale * peak;
            // Three Gauss points integrate the squared quadratic moment exactly.
            let offset = (3.0_f64 / 5.0).sqrt() / 2.0;
            let integral = [
                (0.5 - offset, 5.0 / 18.0),
                (0.5, 4.0 / 9.0),
                (0.5 + offset, 5.0 / 18.0),
            ]
            .into_iter()
            .map(|(t, weight)| weight * (moment(t) / peak).powi(2))
            .sum::<f64>();
            let amplitude = max_moment / self.flexural.sqrt();
            let mut factors = [amplitude, amplitude, 0.5 * integral * self.length];
            factors.sort_unstable_by(f64::total_cmp);
            let energy = (factors[0] * factors[2]) * factors[1];
            if !energy.is_finite() || energy <= 0.0 {
                return Err(format!(
                    "1d beam {}: strain energy is not representable",
                    self.id
                ));
            }
            (max_moment, energy)
        };
        let max_stress = max_moment / section_modulus;
        finite_fields(self.id, "moment or stress", &[max_moment, max_stress])?;
        if max_moment > 0.0 && max_stress == 0.0 {
            return Err(format!("1d beam {}: stress is not representable", self.id));
        }
        // Preserve input node_i/node_j identities and their global nodal actions.
        if self.reversed {
            forces.swap(0, 2);
            forces.swap(1, 3);
        }
        Ok(BeamRecovery {
            forces,
            max_moment,
            max_stress,
            energy,
        })
    }
}
