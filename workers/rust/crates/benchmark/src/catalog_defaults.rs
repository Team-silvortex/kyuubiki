use crate::{
    catalog::{
        BenchmarkCatalogSpec, BenchmarkFamily, BenchmarkMatrixSpec, CaseTemplateSpec,
        FrameGridScale, PanelScale, ProfileScaleSpec, TrussScale,
    },
    config::BenchmarkProfile,
};

pub(crate) fn default_catalog_spec() -> BenchmarkCatalogSpec {
    BenchmarkCatalogSpec {
        templates: default_templates(),
        matrices: default_matrices(),
        profiles: default_profiles(),
    }
}

fn default_templates() -> Vec<CaseTemplateSpec> {
    vec![
        template("axial-bar", BenchmarkFamily::AxialBar),
        template("thermal-bar", BenchmarkFamily::ThermalBar1d),
        template("acoustic-bar", BenchmarkFamily::AcousticBar1d),
        template("heat-bar", BenchmarkFamily::HeatBar1d),
        template("electrostatic-bar", BenchmarkFamily::ElectrostaticBar1d),
        template("magnetostatic-bar", BenchmarkFamily::MagnetostaticBar1d),
        template(
            "advection-diffusion-bar",
            BenchmarkFamily::AdvectionDiffusionBar1d,
        ),
        template("torsion-shaft", BenchmarkFamily::Torsion1d),
        template("spring-chain", BenchmarkFamily::Spring1d),
        template("spring-grid", BenchmarkFamily::Spring2d),
        template("spring-cage", BenchmarkFamily::Spring3d),
        template("nonlinear-spring-chain", BenchmarkFamily::NonlinearSpring1d),
        template("contact-gap-chain", BenchmarkFamily::ContactGap1d),
        template("beam-line", BenchmarkFamily::Beam1d),
        template("thermal-beam-line", BenchmarkFamily::ThermalBeam1d),
        template("frame-2d", BenchmarkFamily::Frame2d),
        template("frame-3d", BenchmarkFamily::Frame3d),
        template("thermal-frame-2d", BenchmarkFamily::ThermalFrame2d),
        template("thermal-frame-3d", BenchmarkFamily::ThermalFrame3d),
        template("modal-frame-2d", BenchmarkFamily::ModalFrame2d),
        template("modal-frame-3d", BenchmarkFamily::ModalFrame3d),
        template("truss-roof", BenchmarkFamily::Truss2d),
        template("space-frame", BenchmarkFamily::TrussFrame3d),
        template("thermal-truss-2d", BenchmarkFamily::ThermalTruss2d),
        template("thermal-truss-3d", BenchmarkFamily::ThermalTruss3d),
        template("plane-panel", BenchmarkFamily::PlaneTriangle2d),
        template("plane-quad-panel", BenchmarkFamily::PlaneQuad2d),
        template(
            "thermal-plane-triangle",
            BenchmarkFamily::ThermalPlaneTriangle2d,
        ),
        template("thermal-plane-quad", BenchmarkFamily::ThermalPlaneQuad2d),
        template("heat-plane-triangle", BenchmarkFamily::HeatPlaneTriangle2d),
        template("heat-plane-quad", BenchmarkFamily::HeatPlaneQuad2d),
        template(
            "electrostatic-plane-triangle",
            BenchmarkFamily::ElectrostaticPlaneTriangle2d,
        ),
        template(
            "electrostatic-plane-quad",
            BenchmarkFamily::ElectrostaticPlaneQuad2d,
        ),
        template(
            "magnetostatic-plane-triangle",
            BenchmarkFamily::MagnetostaticPlaneTriangle2d,
        ),
        template(
            "magnetostatic-plane-quad",
            BenchmarkFamily::MagnetostaticPlaneQuad2d,
        ),
        template("stokes-plane-quad", BenchmarkFamily::StokesFlowPlaneQuad2d),
    ]
}

fn default_matrices() -> Vec<BenchmarkMatrixSpec> {
    vec![
        matrix(
            "core",
            &[
                "axial-bar",
                "truss-roof",
                "space-frame",
                "plane-panel",
                "plane-quad-panel",
                "heat-plane-quad",
            ],
            vec![],
        ),
        matrix(
            "structural",
            &[
                "axial-bar",
                "truss-roof",
                "space-frame",
                "plane-panel",
                "plane-quad-panel",
            ],
            vec![],
        ),
        matrix(
            "mechanical-core",
            &[
                "axial-bar",
                "truss-roof",
                "space-frame",
                "plane-panel",
                "plane-quad-panel",
            ],
            vec![],
        ),
        matrix("thermal", &["heat-plane-quad"], vec![]),
        matrix("thermal-core", &["heat-plane-quad"], vec![]),
        matrix(
            "surface",
            &["plane-panel", "plane-quad-panel", "heat-plane-quad"],
            vec![],
        ),
        compound_matrix("compound"),
        compound_matrix("compound-core"),
        matrix(
            "extended-physics",
            &[
                "heat-bar",
                "electrostatic-bar",
                "magnetostatic-bar",
                "advection-diffusion-bar",
                "acoustic-bar",
                "torsion-shaft",
                "heat-plane-triangle",
                "electrostatic-plane-triangle",
                "electrostatic-plane-quad",
                "magnetostatic-plane-triangle",
                "magnetostatic-plane-quad",
                "stokes-plane-quad",
            ],
            vec![],
        ),
        matrix(
            "structural-extended",
            &[
                "spring-chain",
                "spring-grid",
                "spring-cage",
                "nonlinear-spring-chain",
                "contact-gap-chain",
                "beam-line",
                "thermal-beam-line",
                "modal-frame-2d",
                "modal-frame-3d",
            ],
            vec![],
        ),
        matrix(
            "thermal-structural",
            &[
                "thermal-bar",
                "thermal-truss-2d",
                "thermal-truss-3d",
                "thermal-plane-triangle",
                "thermal-plane-quad",
                "frame-2d",
                "frame-3d",
                "thermal-frame-2d",
                "thermal-frame-3d",
            ],
            vec![],
        ),
        matrix(
            "physics-coverage",
            &[
                "axial-bar",
                "thermal-bar",
                "acoustic-bar",
                "heat-bar",
                "electrostatic-bar",
                "magnetostatic-bar",
                "advection-diffusion-bar",
                "torsion-shaft",
                "spring-chain",
                "spring-grid",
                "spring-cage",
                "nonlinear-spring-chain",
                "contact-gap-chain",
                "beam-line",
                "thermal-beam-line",
                "frame-2d",
                "frame-3d",
                "thermal-frame-2d",
                "thermal-frame-3d",
                "modal-frame-2d",
                "modal-frame-3d",
                "truss-roof",
                "space-frame",
                "thermal-truss-2d",
                "thermal-truss-3d",
                "plane-panel",
                "plane-quad-panel",
                "thermal-plane-triangle",
                "thermal-plane-quad",
                "heat-plane-triangle",
                "heat-plane-quad",
                "electrostatic-plane-triangle",
                "electrostatic-plane-quad",
                "magnetostatic-plane-triangle",
                "magnetostatic-plane-quad",
                "stokes-plane-quad",
            ],
            vec![],
        ),
    ]
}

fn default_profiles() -> Vec<ProfileScaleSpec> {
    vec![
        profile_scale_spec(
            BenchmarkProfile::Medium,
            "medium",
            120,
            TrussScale::Pratt {
                bays: 12,
                span: 24.0,
                height: 5.0,
            },
            FrameGridScale {
                nx: 4,
                ny: 4,
                width: 8.0,
                depth: 8.0,
                height: 2.8,
            },
            PanelScale {
                nx: 6,
                ny: 4,
                width: 6.0,
                height: 4.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::Large,
            "large",
            2500,
            TrussScale::Pratt {
                bays: 127,
                span: 64.0,
                height: 12.0,
            },
            FrameGridScale {
                nx: 14,
                ny: 14,
                width: 28.0,
                depth: 28.0,
                height: 4.8,
            },
            PanelScale {
                nx: 21,
                ny: 21,
                width: 21.0,
                height: 21.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::V2,
            "v2",
            5000,
            TrussScale::Pratt {
                bays: 2500,
                span: 1250.0,
                height: 80.0,
            },
            FrameGridScale {
                nx: 34,
                ny: 34,
                width: 68.0,
                depth: 68.0,
                height: 10.0,
            },
            PanelScale {
                nx: 70,
                ny: 70,
                width: 70.0,
                height: 70.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::TenK,
            "10k",
            10_000,
            TrussScale::Lattice {
                nx: 99,
                ny: 99,
                width: 120.0,
                height: 120.0,
            },
            FrameGridScale {
                nx: 70,
                ny: 70,
                width: 140.0,
                depth: 140.0,
                height: 16.0,
            },
            PanelScale {
                nx: 99,
                ny: 99,
                width: 99.0,
                height: 99.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::FifteenK,
            "15k",
            15_000,
            TrussScale::Lattice {
                nx: 121,
                ny: 121,
                width: 146.0,
                height: 146.0,
            },
            FrameGridScale {
                nx: 86,
                ny: 86,
                width: 172.0,
                depth: 172.0,
                height: 18.0,
            },
            PanelScale {
                nx: 121,
                ny: 121,
                width: 121.0,
                height: 121.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::TwentyK,
            "20k",
            20_000,
            TrussScale::Lattice {
                nx: 140,
                ny: 140,
                width: 168.0,
                height: 168.0,
            },
            FrameGridScale {
                nx: 99,
                ny: 99,
                width: 198.0,
                depth: 198.0,
                height: 20.0,
            },
            PanelScale {
                nx: 140,
                ny: 140,
                width: 140.0,
                height: 140.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::HundredK,
            "100k",
            100_000,
            TrussScale::Lattice {
                nx: 315,
                ny: 315,
                width: 378.0,
                height: 378.0,
            },
            FrameGridScale {
                nx: 223,
                ny: 223,
                width: 446.0,
                depth: 446.0,
                height: 28.0,
            },
            PanelScale {
                nx: 315,
                ny: 315,
                width: 315.0,
                height: 315.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::TwoHundredK,
            "200k",
            200_000,
            TrussScale::Lattice {
                nx: 447,
                ny: 447,
                width: 536.0,
                height: 536.0,
            },
            FrameGridScale {
                nx: 316,
                ny: 316,
                width: 632.0,
                depth: 632.0,
                height: 32.0,
            },
            PanelScale {
                nx: 447,
                ny: 447,
                width: 447.0,
                height: 447.0,
            },
        ),
        profile_scale_spec(
            BenchmarkProfile::ThreeHundredK,
            "300k",
            300_000,
            TrussScale::Lattice {
                nx: 548,
                ny: 548,
                width: 658.0,
                height: 658.0,
            },
            FrameGridScale {
                nx: 387,
                ny: 387,
                width: 774.0,
                depth: 774.0,
                height: 36.0,
            },
            PanelScale {
                nx: 548,
                ny: 548,
                width: 548.0,
                height: 548.0,
            },
        ),
    ]
}

fn profile_scale_spec(
    profile: BenchmarkProfile,
    suffix: &str,
    axial_elements: usize,
    truss: TrussScale,
    space_frame: FrameGridScale,
    panel: PanelScale,
) -> ProfileScaleSpec {
    ProfileScaleSpec {
        profile,
        suffix: suffix.to_string(),
        axial_elements,
        truss,
        space_frame,
        plane_triangle: panel,
        plane_quad: panel,
        heat_plane_quad: panel,
    }
}

fn template(stem: &str, family: BenchmarkFamily) -> CaseTemplateSpec {
    CaseTemplateSpec {
        stem: stem.to_string(),
        family,
    }
}

fn matrix(
    name: &str,
    template_stems: &[&str],
    owned_templates: Vec<CaseTemplateSpec>,
) -> BenchmarkMatrixSpec {
    BenchmarkMatrixSpec {
        name: name.to_string(),
        template_stems: template_stems
            .iter()
            .map(|stem| (*stem).to_string())
            .collect(),
        owned_templates,
    }
}

fn compound_matrix(name: &str) -> BenchmarkMatrixSpec {
    matrix(
        name,
        &["truss-roof", "space-frame", "heat-plane-quad"],
        vec![template(
            "compound-surface-panel",
            BenchmarkFamily::PlaneQuad2d,
        )],
    )
}
