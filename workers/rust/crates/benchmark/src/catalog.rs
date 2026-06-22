use std::fs;

use serde::{Deserialize, Serialize};

pub(crate) use crate::catalog_defaults::default_catalog_spec;

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
