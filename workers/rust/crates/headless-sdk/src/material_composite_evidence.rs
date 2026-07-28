use crate::{
    MaterialEvidenceRef, MaterialModelAssumption, material_evidence_ref, material_model_assumption,
};

pub(crate) fn composite_evidence_refs() -> Vec<MaterialEvidenceRef> {
    vec![
        material_evidence_ref(
            "evidence.layered_dielectric_closed_form",
            "Layered dielectric series closed form",
            "analytic_cross_check",
            "kyuubiki.composite-electrostatic-cross-validation/v1",
            "retained",
            "Independent one-dimensional displacement-continuity solution for the three-region electrostatic fixture.",
        ),
        material_evidence_ref(
            "evidence.electrostatic_mesh_convergence",
            "Electrostatic structured mesh convergence",
            "mesh_convergence",
            "kyuubiki.composite-electrostatic-mesh-convergence/v1",
            "retained",
            "Real Rust solver runs at one, two, four, and eight quad elements per material layer.",
        ),
        material_evidence_ref(
            "evidence.layered_thermal_resistance_closed_form",
            "Layered thermal-resistance closed form",
            "analytic_cross_check",
            "kyuubiki.composite-heat-cross-validation/v1",
            "retained",
            "Independent downstream thermal-resistance solution for interface heating and fixed right-edge temperature.",
        ),
        material_evidence_ref(
            "evidence.heat_mesh_convergence",
            "Heat structured mesh convergence",
            "mesh_convergence",
            "kyuubiki.composite-heat-mesh-convergence/v1",
            "retained",
            "Real Rust solver runs at one, two, four, and eight heat quads per material layer.",
        ),
        material_evidence_ref(
            "evidence.thermal_structural_mesh_convergence",
            "Thermal-structural two-dimensional mesh convergence",
            "mesh_convergence",
            "kyuubiki.composite-thermal-mesh-convergence/v1",
            "active_gate",
            "Real Rust solver runs at one, two, four, and eight subdivisions in both panel directions; displacement and strain energy control the gate.",
        ),
        material_evidence_ref(
            "evidence.thermal_convergence_regime",
            "Thermal-structural observed order and GCI",
            "discretization_uncertainty",
            "kyuubiki.composite-thermal-convergence-regime/v1",
            "active_gate",
            "Four refinement levels classify asymptotic behavior before permitting Richardson extrapolation or fine-grid GCI.",
        ),
        material_evidence_ref(
            "evidence.thermal_algebraic_residual",
            "Thermal-structural algebraic residual",
            "solver_convergence",
            "kyuubiki.composite-thermal-algebraic-validation/v1",
            "active_gate",
            "Recomputes original-system residuals for every uniform, regularized, and graded mesh solve.",
        ),
        material_evidence_ref(
            "evidence.thermal_constraint_sensitivity",
            "Thermal-structural restraint sensitivity",
            "boundary_condition_sensitivity",
            "kyuubiki.composite-thermal-constraint-sensitivity/v1",
            "diagnostic",
            "Compares the full edge clamp with a roller edge and one vertical anchor without overriding the primary quality gates.",
        ),
        material_evidence_ref(
            "evidence.thermal_stress_recovery",
            "Area-weighted thermal-stress recovery",
            "stress_recovery",
            "kyuubiki.composite-thermal-stress-recovery/v1",
            "active_gate",
            "Tracks area-weighted von Mises RMS and P95 convergence while retaining the raw maximum as a singularity diagnostic.",
        ),
        material_evidence_ref(
            "evidence.prototype_material_cards",
            "Prototype material cards",
            "internal_screening_card",
            "kyuubiki.material-cards.composite-panel.v1",
            "screening",
            "Scalar room-temperature values for conductor, dielectric, and substrate families.",
        ),
        material_evidence_ref(
            "evidence.synthetic_multiphysics_fixture",
            "Synthetic mixed-material panel fixture",
            "solver_fixture",
            "kyuubiki.composite_thermo_electric_panel.fixture.v1",
            "prototype",
            "Three-region quad panel used to validate mixed-material sequential coupling.",
        ),
    ]
}

pub(crate) fn composite_model_assumptions() -> Vec<MaterialModelAssumption> {
    vec![
        material_model_assumption(
            "assumption.sequential_coupling",
            "Sequential coupling",
            "electrostatic -> heat -> thermal stress",
            "Good for workflow validation, but misses strong bidirectional coupling.",
        ),
        material_model_assumption(
            "assumption.scalar_regions",
            "Scalar isotropic regions",
            "one material parameter set per region and field",
            "Fast to evaluate, but does not represent anisotropy or interfaces.",
        ),
        material_model_assumption(
            "assumption.prototype_geometry",
            "Prototype panel geometry",
            "three quad regions sharing boundary nodes",
            "Captures multi-region topology without CAD-level geometric fidelity.",
        ),
    ]
}
