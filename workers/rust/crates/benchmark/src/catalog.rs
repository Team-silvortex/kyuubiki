use std::fs;

use serde::{Deserialize, Serialize};

use crate::{
    config::BenchmarkProfile,
    generators::{
        generate_bar_case, generate_heat_quad_panel_mesh, generate_lattice_truss_10k,
        generate_panel_mesh, generate_pratt_truss, generate_quad_panel_mesh,
        generate_space_frame_grid,
    },
    models::{BenchmarkCase, BenchmarkWorkload},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct BenchmarkCatalogSpec {
    pub(crate) templates: Vec<CaseTemplateSpec>,
    pub(crate) matrices: Vec<BenchmarkMatrixSpec>,
    pub(crate) profiles: Vec<ProfileScaleSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct CaseTemplateSpec {
    pub(crate) stem: String,
    pub(crate) family: BenchmarkFamily,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct BenchmarkMatrixSpec {
    pub(crate) name: String,
    #[serde(alias = "templates")]
    pub(crate) template_stems: Vec<String>,
    #[serde(default)]
    pub(crate) owned_templates: Vec<CaseTemplateSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub(crate) struct ProfileScaleSpec {
    pub(crate) profile: BenchmarkProfile,
    pub(crate) suffix: String,
    pub(crate) axial_elements: usize,
    pub(crate) truss: TrussScale,
    pub(crate) space_frame: FrameGridScale,
    pub(crate) plane_triangle: PanelScale,
    pub(crate) plane_quad: PanelScale,
    pub(crate) heat_plane_quad: PanelScale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum BenchmarkFamily {
    AxialBar,
    Truss2d,
    TrussFrame3d,
    PlaneTriangle2d,
    PlaneQuad2d,
    HeatPlaneQuad2d,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TrussScale {
    Pratt {
        bays: usize,
        span: f64,
        height: f64,
    },
    Lattice {
        nx: usize,
        ny: usize,
        width: f64,
        height: f64,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub(crate) struct PanelScale {
    pub(crate) nx: usize,
    pub(crate) ny: usize,
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub(crate) struct FrameGridScale {
    pub(crate) nx: usize,
    pub(crate) ny: usize,
    pub(crate) width: f64,
    pub(crate) depth: f64,
    pub(crate) height: f64,
}

pub(crate) const DEFAULT_CATALOG_PATH: &str = "benchmarks/catalog.default.json";
const DEFAULT_CATALOG_FALLBACK_PATH: &str = "../../benchmarks/catalog.default.json";

pub(crate) fn load_catalog_spec() -> BenchmarkCatalogSpec {
    catalog_spec_path_candidates()
        .into_iter()
        .find_map(|path| fs::read_to_string(path).ok())
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(default_catalog_spec)
}

pub(crate) fn default_catalog_spec() -> BenchmarkCatalogSpec {
    BenchmarkCatalogSpec {
        templates: vec![
            CaseTemplateSpec {
                stem: "axial-bar".to_string(),
                family: BenchmarkFamily::AxialBar,
            },
            CaseTemplateSpec {
                stem: "truss-roof".to_string(),
                family: BenchmarkFamily::Truss2d,
            },
            CaseTemplateSpec {
                stem: "space-frame".to_string(),
                family: BenchmarkFamily::TrussFrame3d,
            },
            CaseTemplateSpec {
                stem: "plane-panel".to_string(),
                family: BenchmarkFamily::PlaneTriangle2d,
            },
            CaseTemplateSpec {
                stem: "plane-quad-panel".to_string(),
                family: BenchmarkFamily::PlaneQuad2d,
            },
            CaseTemplateSpec {
                stem: "heat-plane-quad".to_string(),
                family: BenchmarkFamily::HeatPlaneQuad2d,
            },
        ],
        matrices: vec![
            BenchmarkMatrixSpec {
                name: "core".to_string(),
                template_stems: vec![
                    "axial-bar".to_string(),
                    "truss-roof".to_string(),
                    "space-frame".to_string(),
                    "plane-panel".to_string(),
                    "plane-quad-panel".to_string(),
                    "heat-plane-quad".to_string(),
                ],
                owned_templates: vec![],
            },
            BenchmarkMatrixSpec {
                name: "structural".to_string(),
                template_stems: vec![
                    "axial-bar".to_string(),
                    "truss-roof".to_string(),
                    "space-frame".to_string(),
                    "plane-panel".to_string(),
                    "plane-quad-panel".to_string(),
                ],
                owned_templates: vec![],
            },
            BenchmarkMatrixSpec {
                name: "mechanical-core".to_string(),
                template_stems: vec![
                    "axial-bar".to_string(),
                    "truss-roof".to_string(),
                    "space-frame".to_string(),
                    "plane-panel".to_string(),
                    "plane-quad-panel".to_string(),
                ],
                owned_templates: vec![],
            },
            BenchmarkMatrixSpec {
                name: "thermal".to_string(),
                template_stems: vec!["heat-plane-quad".to_string()],
                owned_templates: vec![],
            },
            BenchmarkMatrixSpec {
                name: "thermal-core".to_string(),
                template_stems: vec!["heat-plane-quad".to_string()],
                owned_templates: vec![],
            },
            BenchmarkMatrixSpec {
                name: "surface".to_string(),
                template_stems: vec![
                    "plane-panel".to_string(),
                    "plane-quad-panel".to_string(),
                    "heat-plane-quad".to_string(),
                ],
                owned_templates: vec![],
            },
            BenchmarkMatrixSpec {
                name: "compound".to_string(),
                template_stems: vec![
                    "truss-roof".to_string(),
                    "space-frame".to_string(),
                    "heat-plane-quad".to_string(),
                ],
                owned_templates: vec![CaseTemplateSpec {
                    stem: "compound-surface-panel".to_string(),
                    family: BenchmarkFamily::PlaneQuad2d,
                }],
            },
            BenchmarkMatrixSpec {
                name: "compound-core".to_string(),
                template_stems: vec![
                    "truss-roof".to_string(),
                    "space-frame".to_string(),
                    "heat-plane-quad".to_string(),
                ],
                owned_templates: vec![CaseTemplateSpec {
                    stem: "compound-surface-panel".to_string(),
                    family: BenchmarkFamily::PlaneQuad2d,
                }],
            },
        ],
        profiles: vec![
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
                PanelScale {
                    nx: 6,
                    ny: 4,
                    width: 6.0,
                    height: 4.0,
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
                PanelScale {
                    nx: 21,
                    ny: 21,
                    width: 21.0,
                    height: 21.0,
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
                PanelScale {
                    nx: 70,
                    ny: 70,
                    width: 70.0,
                    height: 70.0,
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
                PanelScale {
                    nx: 99,
                    ny: 99,
                    width: 99.0,
                    height: 99.0,
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
                PanelScale {
                    nx: 121,
                    ny: 121,
                    width: 121.0,
                    height: 121.0,
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
                PanelScale {
                    nx: 140,
                    ny: 140,
                    width: 140.0,
                    height: 140.0,
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
                PanelScale {
                    nx: 315,
                    ny: 315,
                    width: 315.0,
                    height: 315.0,
                },
                PanelScale {
                    nx: 315,
                    ny: 315,
                    width: 315.0,
                    height: 315.0,
                },
            ),
        ],
    }
}

pub(crate) fn benchmark_cases(profile: BenchmarkProfile, matrix: &str) -> Vec<BenchmarkCase> {
    let spec = load_catalog_spec();
    let profile_spec = spec
        .profiles
        .iter()
        .find(|candidate| candidate.profile == profile)
        .expect("benchmark profile spec should exist");
    let matrix_spec = spec
        .matrices
        .iter()
        .find(|candidate| candidate.name == matrix)
        .or_else(|| {
            spec.matrices
                .iter()
                .find(|candidate| candidate.name == "core")
        })
        .expect("benchmark matrix spec should exist");

    resolve_matrix_templates(&spec, matrix_spec)
        .into_iter()
        .map(|template| build_case(template, profile_spec))
        .collect()
}

fn resolve_matrix_templates<'a>(
    spec: &'a BenchmarkCatalogSpec,
    matrix: &'a BenchmarkMatrixSpec,
) -> Vec<&'a CaseTemplateSpec> {
    let mut resolved = spec
        .templates
        .iter()
        .filter(|template| {
            matrix
                .template_stems
                .iter()
                .any(|stem| stem == &template.stem)
        })
        .collect::<Vec<_>>();
    resolved.extend(matrix.owned_templates.iter());
    resolved
}

fn profile_scale_spec(
    profile: BenchmarkProfile,
    suffix: &str,
    axial_elements: usize,
    truss: TrussScale,
    space_frame: FrameGridScale,
    plane_triangle: PanelScale,
    plane_quad: PanelScale,
    heat_plane_quad: PanelScale,
) -> ProfileScaleSpec {
    ProfileScaleSpec {
        profile,
        suffix: suffix.to_string(),
        axial_elements,
        truss,
        space_frame,
        plane_triangle,
        plane_quad,
        heat_plane_quad,
    }
}

fn build_case(template: &CaseTemplateSpec, profile: &ProfileScaleSpec) -> BenchmarkCase {
    let id = format!("{}-{}", template.stem, profile.suffix);

    match template.family {
        BenchmarkFamily::AxialBar => BenchmarkCase {
            id,
            family: "axial_bar_1d",
            workload: BenchmarkWorkload::AxialBar(generate_bar_case(profile.axial_elements)),
        },
        BenchmarkFamily::Truss2d => build_truss_case(id, &profile.truss),
        BenchmarkFamily::TrussFrame3d => BenchmarkCase {
            id,
            family: "truss_3d",
            workload: BenchmarkWorkload::Truss3d(generate_space_frame_grid(
                profile.space_frame.nx,
                profile.space_frame.ny,
                profile.space_frame.width,
                profile.space_frame.depth,
                profile.space_frame.height,
            )),
        },
        BenchmarkFamily::PlaneTriangle2d => BenchmarkCase {
            id,
            family: "plane_triangle_2d",
            workload: BenchmarkWorkload::PlaneTriangle2d(generate_panel_mesh(
                profile.plane_triangle.nx,
                profile.plane_triangle.ny,
                profile.plane_triangle.width,
                profile.plane_triangle.height,
            )),
        },
        BenchmarkFamily::PlaneQuad2d => BenchmarkCase {
            id,
            family: "plane_quad_2d",
            workload: BenchmarkWorkload::PlaneQuad2d(generate_quad_panel_mesh(
                profile.plane_quad.nx,
                profile.plane_quad.ny,
                profile.plane_quad.width,
                profile.plane_quad.height,
            )),
        },
        BenchmarkFamily::HeatPlaneQuad2d => BenchmarkCase {
            id,
            family: "heat_plane_quad_2d",
            workload: BenchmarkWorkload::HeatPlaneQuad2d(generate_heat_quad_panel_mesh(
                profile.heat_plane_quad.nx,
                profile.heat_plane_quad.ny,
                profile.heat_plane_quad.width,
                profile.heat_plane_quad.height,
            )),
        },
    }
}

fn build_truss_case(id: String, truss: &TrussScale) -> BenchmarkCase {
    let workload = match truss {
        TrussScale::Pratt { bays, span, height } => {
            BenchmarkWorkload::Truss2d(generate_pratt_truss(*bays, *span, *height))
        }
        TrussScale::Lattice {
            nx,
            ny,
            width,
            height,
        } => BenchmarkWorkload::Truss2d(generate_lattice_truss_10k(*nx, *ny, *width, *height)),
    };

    BenchmarkCase {
        id,
        family: "truss_2d",
        workload,
    }
}

pub(crate) fn catalog_spec_path_candidates() -> [String; 2] {
    [
        DEFAULT_CATALOG_PATH.to_string(),
        format!(
            "{}/{}",
            env!("CARGO_MANIFEST_DIR"),
            DEFAULT_CATALOG_FALLBACK_PATH
        ),
    ]
}
