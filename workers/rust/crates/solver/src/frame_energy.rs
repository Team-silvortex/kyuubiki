pub(crate) fn frame_strain_energy_6(
    local_forces: &[f64; 6],
    local_displacements: &[f64; 6],
) -> f64 {
    0.5 * (0..6)
        .map(|index| local_forces[index] * local_displacements[index])
        .sum::<f64>()
}

pub(crate) fn frame2d_field_energy(
    id: &str,
    rigidities: [f64; 2],
    length: f64,
    local_displacements: &[f64; 6],
    mechanical_strain: f64,
    thermal_curvature: f64,
) -> Result<f64, String> {
    let total_curvature = (local_displacements[5] - local_displacements[2]) / length;
    let mechanical_curvature = total_curvature - thermal_curvature;
    let chord_slope = (local_displacements[4] - local_displacements[1]) / length;
    // The Hermite curvature is linear. Its mean square includes this variance,
    // which is lost when only the difference of endpoint rotations is used.
    let curvature_rms = 3.0_f64.sqrt()
        * ((local_displacements[2] - chord_slope) + (local_displacements[5] - chord_slope))
        / length;
    let mut total = 0.0;
    for (strain, rigidity) in [
        (mechanical_strain, rigidities[0]),
        (mechanical_curvature, rigidities[1]),
        (curvature_rms, rigidities[1]),
    ] {
        let amplitude = strain.abs() * rigidity.sqrt();
        let mut factors = [amplitude, amplitude, length / 2.0];
        factors.sort_unstable_by(f64::total_cmp);
        let energy = (factors[0] * factors[2]) * factors[1];
        if !energy.is_finite() || (strain != 0.0 && energy == 0.0) {
            return Err(format!("2d frame {id}: strain energy is not representable"));
        }
        total += energy;
        if !total.is_finite() {
            return Err(format!("2d frame {id}: strain energy is not representable"));
        }
    }
    Ok(total)
}
