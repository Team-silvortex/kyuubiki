use kyuubiki_protocol::{SolidTetra3dElementInput, SolidTetra3dElementResult};

use crate::linear_algebra::{MatrixAssembler, add_at};
use crate::solid_tetra_3d_geometry::TetraGeometry;

pub(crate) struct SolidTetra3dElementKernel {
    volume: f64,
    mean_ratio_quality: f64,
    b: [[f64; 12]; 6],
    d: [[f64; 6]; 6],
    stiffness: [[f64; 12]; 12],
}

impl SolidTetra3dElementKernel {
    pub(crate) fn new(
        points: [[f64; 3]; 4],
        element: &SolidTetra3dElementInput,
    ) -> Result<Self, String> {
        validate_properties(element)?;
        if points.iter().flatten().any(|value| !value.is_finite()) {
            return Err(format!(
                "solid tetra element {} coordinates must be finite",
                element.id
            ));
        }
        let (volume, mean_ratio_quality, b) = geometry(points, &element.id)?;
        let d = elasticity_matrix(element.youngs_modulus, element.poisson_ratio);
        let db = multiply_6x6_6x12(&d, &b);
        let stiffness = multiply_12x6_6x12(&b, &db, volume);
        if d.iter()
            .flatten()
            .chain(stiffness.iter().flatten())
            .any(|value| !value.is_finite())
        {
            return Err(format!(
                "solid tetra element {} constitutive or stiffness coefficients are not representable",
                element.id
            ));
        }
        Ok(Self {
            volume,
            mean_ratio_quality,
            b,
            d,
            stiffness,
        })
    }

    pub(crate) fn assemble<M: MatrixAssembler + ?Sized>(
        &self,
        dofs: &[usize; 12],
        displacements: &[f64],
        internal_forces: &mut [f64],
        tangent: &mut M,
    ) {
        for row in 0..12 {
            let internal = (0..12)
                .map(|column| self.stiffness[row][column] * displacements[dofs[column]])
                .sum::<f64>();
            internal_forces[dofs[row]] += internal;
            for column in 0..12 {
                add_at(
                    tangent,
                    dofs[row],
                    dofs[column],
                    self.stiffness[row][column],
                );
            }
        }
    }

    pub(crate) fn result(
        &self,
        index: usize,
        element: &SolidTetra3dElementInput,
        dofs: &[usize; 12],
        displacements: &[f64],
    ) -> SolidTetra3dElementResult {
        let local = std::array::from_fn(|i| displacements[dofs[i]]);
        let strain = multiply_6x12_12(&self.b, &local);
        let stress = multiply_6x6_6(&self.d, &strain);
        SolidTetra3dElementResult {
            index,
            id: element.id.clone(),
            node_a: element.node_a,
            node_b: element.node_b,
            node_c: element.node_c,
            node_d: element.node_d,
            volume: self.volume,
            strain_x: strain[0],
            strain_y: strain[1],
            strain_z: strain[2],
            gamma_xy: strain[3],
            gamma_yz: strain[4],
            gamma_zx: strain[5],
            stress_x: stress[0],
            stress_y: stress[1],
            stress_z: stress[2],
            shear_xy: stress[3],
            shear_yz: stress[4],
            shear_zx: stress[5],
            von_mises_stress: von_mises_stress(&stress),
            strain_energy_density: strain_energy_density(&stress, &strain),
            mean_ratio_quality: self.mean_ratio_quality,
        }
    }
}

pub(crate) fn element_dof_map(element: &SolidTetra3dElementInput) -> [usize; 12] {
    let nodes = [
        element.node_a,
        element.node_b,
        element.node_c,
        element.node_d,
    ];
    std::array::from_fn(|local_dof| nodes[local_dof / 3] * 3 + local_dof % 3)
}

fn validate_properties(element: &SolidTetra3dElementInput) -> Result<(), String> {
    if !element.youngs_modulus.is_finite() || element.youngs_modulus <= 0.0 {
        return Err(format!(
            "solid tetra element {} must have finite positive youngs_modulus",
            element.id
        ));
    }
    if !(element.poisson_ratio.is_finite()
        && element.poisson_ratio > -1.0
        && element.poisson_ratio < 0.5)
    {
        return Err(format!(
            "solid tetra element {} must have poisson_ratio in (-1, 0.5)",
            element.id
        ));
    }
    Ok(())
}

fn geometry(points: [[f64; 3]; 4], id: &str) -> Result<(f64, f64, [[f64; 12]; 6]), String> {
    let geometry = TetraGeometry::new(points, id)?;
    let mean_ratio_quality = geometry.mean_ratio_quality();
    if !mean_ratio_quality.is_finite() || mean_ratio_quality <= 1.0e-12 {
        return Err(format!(
            "solid tetra element {id} is near-degenerate (mean_ratio_quality={mean_ratio_quality:.6e})"
        ));
    }
    let b = geometry.strain_matrix();
    if b.iter().flatten().any(|value| !value.is_finite()) {
        return Err(format!(
            "solid tetra element {id} shape gradients are not representable"
        ));
    }
    Ok((geometry.volume, mean_ratio_quality, b))
}

fn elasticity_matrix(youngs_modulus: f64, poisson_ratio: f64) -> [[f64; 6]; 6] {
    let factor = youngs_modulus / ((1.0 + poisson_ratio) * (1.0 - 2.0 * poisson_ratio));
    let normal = 1.0 - poisson_ratio;
    let shear = (1.0 - 2.0 * poisson_ratio) * 0.5;
    [
        [
            factor * normal,
            factor * poisson_ratio,
            factor * poisson_ratio,
            0.0,
            0.0,
            0.0,
        ],
        [
            factor * poisson_ratio,
            factor * normal,
            factor * poisson_ratio,
            0.0,
            0.0,
            0.0,
        ],
        [
            factor * poisson_ratio,
            factor * poisson_ratio,
            factor * normal,
            0.0,
            0.0,
            0.0,
        ],
        [0.0, 0.0, 0.0, factor * shear, 0.0, 0.0],
        [0.0, 0.0, 0.0, 0.0, factor * shear, 0.0],
        [0.0, 0.0, 0.0, 0.0, 0.0, factor * shear],
    ]
}

fn multiply_6x6_6x12(a: &[[f64; 6]; 6], b: &[[f64; 12]; 6]) -> [[f64; 12]; 6] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| (0..6).map(|k| a[row][k] * b[k][column]).sum())
    })
}

fn multiply_12x6_6x12(b: &[[f64; 12]; 6], db: &[[f64; 12]; 6], volume: f64) -> [[f64; 12]; 12] {
    std::array::from_fn(|row| {
        std::array::from_fn(|column| {
            (0..6).map(|k| b[k][row] * db[k][column]).sum::<f64>() * volume
        })
    })
}

fn multiply_6x12_12(a: &[[f64; 12]; 6], vector: &[f64; 12]) -> [f64; 6] {
    std::array::from_fn(|row| (0..12).map(|column| a[row][column] * vector[column]).sum())
}

fn multiply_6x6_6(a: &[[f64; 6]; 6], vector: &[f64; 6]) -> [f64; 6] {
    std::array::from_fn(|row| (0..6).map(|column| a[row][column] * vector[column]).sum())
}

fn von_mises_stress(stress: &[f64; 6]) -> f64 {
    let [sx, sy, sz, txy, tyz, tzx] = *stress;
    let differences = [sx - sy, sy - sz, sz - sx];
    let scale = if differences.iter().all(|value| value.is_finite()) {
        // Scale the deviatoric components, not a potentially huge hydrostatic pressure.
        differences
            .iter()
            .chain(stress[3..].iter())
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max)
    } else {
        stress
            .iter()
            .map(|value| value.abs())
            .fold(0.0_f64, f64::max)
    };
    if scale == 0.0 {
        return 0.0;
    }
    let differences = if differences.iter().all(|value| value.is_finite()) {
        differences.map(|value| value / scale)
    } else {
        [
            sx / scale - sy / scale,
            sy / scale - sz / scale,
            sz / scale - sx / scale,
        ]
    };
    (0.5 * differences.iter().map(|value| value * value).sum::<f64>()
        + 3.0 * ((txy / scale).powi(2) + (tyz / scale).powi(2) + (tzx / scale).powi(2)))
    .sqrt()
        * scale
}

fn strain_energy_density(stress: &[f64; 6], strain: &[f64; 6]) -> f64 {
    0.5 * (0..6)
        .map(|index| stress[index] * strain[index])
        .sum::<f64>()
}

#[cfg(test)]
#[path = "solid_tetra_3d_geometry_tests.rs"]
mod geometry_tests;

#[cfg(test)]
#[path = "solid_tetra_3d_reference.rs"]
mod reference;

#[cfg(test)]
#[path = "solid_tetra_3d_benchmark.rs"]
mod benchmark;

#[cfg(test)]
mod tests {
    use super::{geometry, von_mises_stress};

    #[test]
    fn equivalent_stress_preserves_deviatoric_scales_and_hydrostatic_invariance() {
        for scale in [1e-300, 1e-200, 1.0, 1e200, 1e300] {
            assert_eq!(von_mises_stress(&[scale, 0.0, 0.0, 0.0, 0.0, 0.0]), scale);
            assert_eq!(von_mises_stress(&[scale, scale, scale, 0.0, 0.0, 0.0]), 0.0);
            let shear = von_mises_stress(&[1e308, 1e308, 1e308, scale, 0.0, 0.0]);
            assert!((shear / scale - 3.0_f64.sqrt()).abs() < 1e-14);
        }
    }

    #[test]
    fn opposite_extreme_normals_have_representable_equivalent_stress() {
        let actual = von_mises_stress(&[1e308, -1e308, 0.0, 0.0, 0.0, 0.0]);
        assert!((actual / 1e308 - 3.0_f64.sqrt()).abs() < 1e-14);
    }

    #[test]
    fn mean_ratio_is_one_for_a_regular_tetra_and_scale_invariant() {
        let regular = [
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.5, 3.0_f64.sqrt() / 2.0, 0.0],
            [0.5, 3.0_f64.sqrt() / 6.0, (2.0_f64 / 3.0).sqrt()],
        ];
        let quality = geometry(regular, "regular").unwrap().1;
        assert!((quality - 1.0).abs() <= 1.0e-14);

        let scale = 1.0e-9;
        let microscopic = regular.map(|point| point.map(|coordinate| coordinate * scale));
        let microscopic_quality = geometry(microscopic, "microscopic").unwrap().1;
        assert!((microscopic_quality - quality).abs() <= 1.0e-14);
    }
}
