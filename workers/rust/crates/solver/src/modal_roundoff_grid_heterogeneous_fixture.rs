use super::super::super::super::{hybrid_tests::reference, wide_projection};
use crate::linear_algebra::{SparseMatrix, add_at};
use crate::modal_frame_assembly::element_masses;
use crate::modal_frame_spectrum::{checked_mode_shape, frame_eigenpairs};
use crate::modal_sparse::{ReducedSparseModalSystem, reduce_sparse_modal_system};
use crate::modal_test_wide::Wide;
use kyuubiki_protocol::{
    ModalFrame2dModeResult, SolveModalFrame2dRequest, SolveModalFrame2dResult,
};
use serde_json::json;

#[derive(Clone, Copy, Debug)]
pub(super) enum Profile {
    Graded,
    Layered,
    UnequalLengths,
}

impl Profile {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Graded => "graded",
            Self::Layered => "layered",
            Self::UnequalLengths => "unequal-lengths",
        }
    }

    fn parameters(self, element: usize, segments: usize) -> [f64; 5] {
        let fraction = (element as f64 + 0.5) / segments as f64;
        // Length, elastic modulus, area, second moment, density multipliers.
        match self {
            Self::Graded => [
                1.0,
                0.7 + 0.6 * fraction,
                1.0 + 0.3 * fraction,
                0.75 + 0.5 * fraction,
                1.25 - 0.5 * fraction,
            ],
            Self::Layered if element < segments / 2 => [1.0; 5],
            Self::Layered => [1.0, 6.0, 0.7, 0.4, 2.5],
            Self::UnequalLengths => [
                [0.7, 1.1, 1.4, 0.8][element % 4],
                [0.85, 1.2][element % 2],
                [1.0, 0.8][element % 2],
                [0.9, 1.1][element % 2],
                [1.15, 0.85][element % 2],
            ],
        }
    }
}

pub(super) struct Fixture {
    input: SolveModalFrame2dRequest,
    total_mass: f64,
    pub(super) system: ReducedSparseModalSystem,
    pub(super) value: f64,
    pub(super) seed: Vec<f64>,
    pub(super) directions: Vec<Vec<Wide>>,
}

impl Fixture {
    pub(super) fn prepare(profile: Profile, segments: usize, scale: f64) -> Result<Self, String> {
        let input = Self::request(profile, segments, scale);
        crate::modal_frame_validation::validate_modal_frame_2d_request(&input)?;
        Self::from_input(input)
    }

    pub(super) fn request(
        profile: Profile,
        segments: usize,
        scale: f64,
    ) -> SolveModalFrame2dRequest {
        let mut position = 0.0;
        let mut nodes = Vec::with_capacity(segments + 1);
        let mut elements = Vec::with_capacity(segments);
        for i in 0..=segments {
            nodes.push(
                json!({"id":format!("node-{i}"), "x":position * scale, "y":0.0,
                "fix_x":true, "fix_y":i==0, "fix_rz":i==0,
                "load_x":0.0, "load_y":0.0, "moment_z":0.0}),
            );
            if i < segments {
                let [length, young, area, inertia, density] = profile.parameters(i, segments);
                position += length;
                elements.push(json!({"id":format!("beam-{i}"), "node_i":i, "node_j":i+1,
                    "area":area, "youngs_modulus":scale.powi(3) * young,
                    "moment_of_inertia":inertia, "section_modulus":1.0,
                    "density":scale.recip() * density}));
            }
        }
        serde_json::from_value(json!({"nodes":nodes, "elements":elements, "mode_count":1})).unwrap()
    }

    pub(super) fn assemble(
        input: &SolveModalFrame2dRequest,
    ) -> Result<(ReducedSparseModalSystem, Vec<Vec<f64>>, f64), String> {
        let size = 2 * input.nodes.len();
        let mut matrix = SparseMatrix::new(size);
        let mut mass = vec![0.0; size];
        let mut total_mass = 0.0;
        for element in &input.elements {
            let length = (input.nodes[element.node_j].x - input.nodes[element.node_i].x).abs();
            let local = crate::frame_2d_math::frame_local_stiffness(
                element.area,
                element.youngs_modulus,
                element.moment_of_inertia,
                length,
            );
            let bending = [1, 2, 4, 5];
            let dofs = [
                2 * element.node_i,
                2 * element.node_i + 1,
                2 * element.node_j,
                2 * element.node_j + 1,
            ];
            for (i, &row) in bending.iter().enumerate() {
                for (j, &column) in bending.iter().enumerate() {
                    add_at(&mut matrix, dofs[i], dofs[j], local[row][column]);
                }
            }
            let [total, translation, rotation] =
                element_masses(&element.id, element.density, element.area, length)?;
            total_mass += total;
            for node in [element.node_i, element.node_j] {
                mass[2 * node] += translation;
                mass[2 * node + 1] += rotation;
            }
        }
        let system = reduce_sparse_modal_system(&matrix, &mass, &[0, 1])?;
        let mut physical = vec![vec![0.0; size - 2]; size - 2];
        for row in 2..size {
            for &(column, value) in matrix.row_entries(row) {
                if column >= 2 {
                    physical[row - 2][column - 2] = value;
                }
            }
        }
        Ok((system, physical, total_mass))
    }

    fn from_input(input: SolveModalFrame2dRequest) -> Result<Self, String> {
        let (system, physical, total_mass) = Self::assemble(&input)?;
        let size = 2 * input.nodes.len();
        let spectrum = frame_eigenpairs(&system, Some(1))
            .map_err(|error| format!("spectrum preparation: {error}"))?;
        let (value, vector) = &spectrum.pairs[0];
        let (seed, _) = checked_mode_shape(
            vector,
            &system.mass,
            &(0..size - 2).collect::<Vec<_>>(),
            size - 2,
        )?;
        let seed = system
            .operator
            .roundoff_comparison_seed(*value, &seed, &system.mass)?;
        let directions = wide_projection::physical_directions(&physical, &system.mass, *value)?;
        Ok(Self {
            input,
            total_mass,
            value: *value,
            system,
            seed,
            directions,
        })
    }

    pub(super) fn checked(&self, shape: &[f64]) -> Result<(f64, Vec<f64>), String> {
        let applied = self.system.operator.apply_physical_compensated(shape)?;
        self.system
            .operator
            .physical_residual(self.value, shape, &self.system.mass, &applied)
    }

    pub(super) fn check_readback(&self, candidate: &[f64]) -> f64 {
        Self::check_recovered_readback(&self.input, self.total_mass, self.value, candidate)
    }

    pub(super) fn check_recovered_readback(
        input: &SolveModalFrame2dRequest,
        total_mass: f64,
        value: f64,
        candidate: &[f64],
    ) -> f64 {
        let free_dofs: Vec<_> = (1..input.nodes.len())
            .flat_map(|i| [3 * i + 1, 3 * i + 2])
            .collect();
        assert_eq!(free_dofs.len(), candidate.len());
        let mut shape = vec![0.0; 3 * input.nodes.len()];
        for (&dof, &value) in free_dofs.iter().zip(candidate) {
            shape[dof] = value;
        }
        let norm = crate::modal_math::checked_shape_norm(&shape).unwrap();
        assert!((norm - 1.0).abs() < 1e-10);
        let frequency = value.sqrt() / std::f64::consts::TAU;
        let result = SolveModalFrame2dResult {
            input: input.clone(),
            modes: vec![ModalFrame2dModeResult {
                index: 0,
                eigenvalue_rad_s_squared: value,
                natural_frequency_rad_s: value.sqrt(),
                natural_frequency_hz: frequency,
                period_s: frequency.recip(),
                participation_norm: norm,
                shape,
            }],
            free_dofs,
            total_mass,
            min_frequency_hz: frequency,
            max_frequency_hz: frequency,
        };
        Self::check_public_readback(&result)
    }

    pub(super) fn check_public_readback(result: &SolveModalFrame2dResult) -> f64 {
        assert_eq!(result.modes.len(), 1);
        let norm = crate::modal_math::checked_shape_norm(&result.modes[0].shape).unwrap();
        assert!((norm - 1.0).abs() < 1e-10);
        let restored: SolveModalFrame2dResult =
            serde_json::from_slice(&serde_json::to_vec(result).unwrap()).unwrap();
        assert_eq!(&restored, result);
        assert!(
            restored.modes[0]
                .shape
                .iter()
                .zip(&result.modes[0].shape)
                .all(|(a, b)| a.to_bits() == b.to_bits())
        );
        assert_eq!(
            restored.modes[0].eigenvalue_rad_s_squared.to_bits(),
            result.modes[0].eigenvalue_rad_s_squared.to_bits()
        );
        let relative = reference::reassembled_residual(&restored);
        assert!(
            relative.is_finite() && relative <= 1e-8,
            "independent heterogeneous residual={relative:e}"
        );
        relative
    }
}
