pub(crate) fn frame_strain_energy_6(
    local_forces: &[f64; 6],
    local_displacements: &[f64; 6],
) -> f64 {
    0.5 * (0..6)
        .map(|index| local_forces[index] * local_displacements[index])
        .sum::<f64>()
}

pub(crate) fn thermal_frame2d_strain_energy(
    youngs_modulus: f64,
    area: f64,
    moment_of_inertia: f64,
    length: f64,
    local_displacements: &[f64; 6],
    mechanical_strain: f64,
    thermal_curvature: f64,
) -> f64 {
    let total_curvature = (local_displacements[5] - local_displacements[2]) / length;
    let mechanical_curvature = total_curvature - thermal_curvature;

    0.5 * youngs_modulus
        * (area * mechanical_strain.powi(2) + moment_of_inertia * mechanical_curvature.powi(2))
        * length
}

pub(crate) fn thermal_frame3d_strain_energy(
    local_stiffness: &[[f64; 12]; 12],
    equivalent_thermal_load: &[f64; 12],
    local_displacements: &[f64; 12],
    initial_thermal_energy: f64,
) -> f64 {
    let elastic_energy = 0.5
        * (0..12)
            .map(|row| {
                local_displacements[row]
                    * (0..12)
                        .map(|column| local_stiffness[row][column] * local_displacements[column])
                        .sum::<f64>()
            })
            .sum::<f64>();
    let thermal_work = (0..12)
        .map(|index| equivalent_thermal_load[index] * local_displacements[index])
        .sum::<f64>();
    elastic_energy - thermal_work + initial_thermal_energy
}
