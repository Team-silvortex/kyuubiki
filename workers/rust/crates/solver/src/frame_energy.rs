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
    youngs_modulus: f64,
    shear_modulus: f64,
    area: f64,
    torsion_constant: f64,
    moment_of_inertia_y: f64,
    moment_of_inertia_z: f64,
    length: f64,
    local_displacements: &[f64; 12],
    mechanical_strain: f64,
    thermal_curvature_y: f64,
    thermal_curvature_z: f64,
) -> f64 {
    let axial_energy = 0.5 * youngs_modulus * area * mechanical_strain.powi(2) * length;
    let total_curvature_y = (local_displacements[10] - local_displacements[4]) / length;
    let total_curvature_z = (local_displacements[11] - local_displacements[5]) / length;
    let mechanical_curvature_y = total_curvature_y - thermal_curvature_y;
    let mechanical_curvature_z = total_curvature_z - thermal_curvature_z;
    let bending_energy = 0.5
        * youngs_modulus
        * (moment_of_inertia_z * mechanical_curvature_y.powi(2)
            + moment_of_inertia_y * mechanical_curvature_z.powi(2))
        * length;
    let torsion_rate = (local_displacements[9] - local_displacements[3]) / length;
    let torsion_energy = 0.5 * shear_modulus * torsion_constant * torsion_rate.powi(2) * length;

    axial_energy + bending_energy + torsion_energy
}
