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
    generators_extended::{
        generate_acoustic_bar_case, generate_electrostatic_bar_case,
        generate_electrostatic_quad_panel, generate_electrostatic_triangle_panel,
        generate_heat_bar_case, generate_heat_triangle_panel, generate_magnetostatic_bar_case,
        generate_magnetostatic_quad_panel, generate_magnetostatic_triangle_panel,
        generate_stokes_quad_panel, generate_torsion_case,
    },
    generators_structural::{
        generate_beam_1d_case, generate_contact_gap_1d_case, generate_modal_frame_2d_case,
        generate_modal_frame_3d_case, generate_nonlinear_spring_1d_case, generate_spring_1d_case,
        generate_spring_2d_case, generate_spring_3d_case, generate_thermal_beam_1d_case,
    },
    generators_thermal_structural::{
        generate_frame_2d_case, generate_frame_3d_case, generate_thermal_bar_case,
        generate_thermal_frame_2d_case, generate_thermal_frame_3d_case,
        generate_thermal_quad_panel, generate_thermal_triangle_panel,
        generate_thermal_truss_2d_case, generate_thermal_truss_3d_case,
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
    ThermalBar1d,
    AcousticBar1d,
    HeatBar1d,
    ElectrostaticBar1d,
    MagnetostaticBar1d,
    Torsion1d,
    Spring1d,
    Spring2d,
    Spring3d,
    NonlinearSpring1d,
    ContactGap1d,
    Beam1d,
    ThermalBeam1d,
    Frame2d,
    Frame3d,
    ThermalFrame2d,
    ThermalFrame3d,
    ModalFrame2d,
    ModalFrame3d,
    Truss2d,
    TrussFrame3d,
    ThermalTruss2d,
    ThermalTruss3d,
    PlaneTriangle2d,
    PlaneQuad2d,
    ThermalPlaneTriangle2d,
    ThermalPlaneQuad2d,
    HeatPlaneTriangle2d,
    HeatPlaneQuad2d,
    ElectrostaticPlaneTriangle2d,
    ElectrostaticPlaneQuad2d,
    MagnetostaticPlaneTriangle2d,
    MagnetostaticPlaneQuad2d,
    StokesFlowPlaneQuad2d,
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
    let mut resolved = matrix
        .template_stems
        .iter()
        .map(|stem| {
            spec.templates
                .iter()
                .find(|template| &template.stem == stem)
                .unwrap_or_else(|| {
                    panic!(
                        "benchmark matrix '{}' references missing template '{}'",
                        matrix.name, stem
                    )
                })
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
        BenchmarkFamily::ThermalBar1d => BenchmarkCase {
            id,
            family: "thermal_bar_1d",
            workload: BenchmarkWorkload::ThermalBar1d(generate_thermal_bar_case(
                profile.axial_elements,
            )),
        },
        BenchmarkFamily::AcousticBar1d => BenchmarkCase {
            id,
            family: "acoustic_bar_1d",
            workload: BenchmarkWorkload::AcousticBar1d(generate_acoustic_bar_case(
                profile.axial_elements,
            )),
        },
        BenchmarkFamily::HeatBar1d => BenchmarkCase {
            id,
            family: "heat_bar_1d",
            workload: BenchmarkWorkload::HeatBar1d(generate_heat_bar_case(profile.axial_elements)),
        },
        BenchmarkFamily::ElectrostaticBar1d => BenchmarkCase {
            id,
            family: "electrostatic_bar_1d",
            workload: BenchmarkWorkload::ElectrostaticBar1d(generate_electrostatic_bar_case(
                profile.axial_elements,
            )),
        },
        BenchmarkFamily::MagnetostaticBar1d => BenchmarkCase {
            id,
            family: "magnetostatic_bar_1d",
            workload: BenchmarkWorkload::MagnetostaticBar1d(generate_magnetostatic_bar_case(
                profile.axial_elements,
            )),
        },
        BenchmarkFamily::Torsion1d => BenchmarkCase {
            id,
            family: "torsion_1d",
            workload: BenchmarkWorkload::Torsion1d(generate_torsion_case(profile.axial_elements)),
        },
        BenchmarkFamily::Spring1d => BenchmarkCase {
            id,
            family: "spring_1d",
            workload: BenchmarkWorkload::Spring1d(generate_spring_1d_case(profile.axial_elements)),
        },
        BenchmarkFamily::Spring2d => BenchmarkCase {
            id,
            family: "spring_2d",
            workload: BenchmarkWorkload::Spring2d(generate_spring_2d_case()),
        },
        BenchmarkFamily::Spring3d => BenchmarkCase {
            id,
            family: "spring_3d",
            workload: BenchmarkWorkload::Spring3d(generate_spring_3d_case()),
        },
        BenchmarkFamily::NonlinearSpring1d => BenchmarkCase {
            id,
            family: "nonlinear_spring_1d",
            workload: BenchmarkWorkload::NonlinearSpring1d(generate_nonlinear_spring_1d_case(
                profile.axial_elements.min(120),
            )),
        },
        BenchmarkFamily::ContactGap1d => BenchmarkCase {
            id,
            family: "contact_gap_1d",
            workload: BenchmarkWorkload::ContactGap1d(generate_contact_gap_1d_case(
                profile.axial_elements.min(120),
            )),
        },
        BenchmarkFamily::Beam1d => BenchmarkCase {
            id,
            family: "beam_1d",
            workload: BenchmarkWorkload::Beam1d(generate_beam_1d_case(profile.axial_elements)),
        },
        BenchmarkFamily::ThermalBeam1d => BenchmarkCase {
            id,
            family: "thermal_beam_1d",
            workload: BenchmarkWorkload::ThermalBeam1d(generate_thermal_beam_1d_case(
                profile.axial_elements,
            )),
        },
        BenchmarkFamily::Frame2d => BenchmarkCase {
            id,
            family: "frame_2d",
            workload: BenchmarkWorkload::Frame2d(generate_frame_2d_case()),
        },
        BenchmarkFamily::Frame3d => BenchmarkCase {
            id,
            family: "frame_3d",
            workload: BenchmarkWorkload::Frame3d(generate_frame_3d_case()),
        },
        BenchmarkFamily::ThermalFrame2d => BenchmarkCase {
            id,
            family: "thermal_frame_2d",
            workload: BenchmarkWorkload::ThermalFrame2d(generate_thermal_frame_2d_case()),
        },
        BenchmarkFamily::ThermalFrame3d => BenchmarkCase {
            id,
            family: "thermal_frame_3d",
            workload: BenchmarkWorkload::ThermalFrame3d(generate_thermal_frame_3d_case()),
        },
        BenchmarkFamily::ModalFrame2d => BenchmarkCase {
            id,
            family: "modal_frame_2d",
            workload: BenchmarkWorkload::ModalFrame2d(generate_modal_frame_2d_case()),
        },
        BenchmarkFamily::ModalFrame3d => BenchmarkCase {
            id,
            family: "modal_frame_3d",
            workload: BenchmarkWorkload::ModalFrame3d(generate_modal_frame_3d_case()),
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
        BenchmarkFamily::ThermalTruss2d => BenchmarkCase {
            id,
            family: "thermal_truss_2d",
            workload: BenchmarkWorkload::ThermalTruss2d(generate_thermal_truss_2d_case()),
        },
        BenchmarkFamily::ThermalTruss3d => BenchmarkCase {
            id,
            family: "thermal_truss_3d",
            workload: BenchmarkWorkload::ThermalTruss3d(generate_thermal_truss_3d_case()),
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
        BenchmarkFamily::ThermalPlaneTriangle2d => BenchmarkCase {
            id,
            family: "thermal_plane_triangle_2d",
            workload: BenchmarkWorkload::ThermalPlaneTriangle2d(generate_thermal_triangle_panel(
                profile.plane_triangle.nx,
                profile.plane_triangle.ny,
                profile.plane_triangle.width,
                profile.plane_triangle.height,
            )),
        },
        BenchmarkFamily::ThermalPlaneQuad2d => BenchmarkCase {
            id,
            family: "thermal_plane_quad_2d",
            workload: BenchmarkWorkload::ThermalPlaneQuad2d(generate_thermal_quad_panel(
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
        BenchmarkFamily::HeatPlaneTriangle2d => BenchmarkCase {
            id,
            family: "heat_plane_triangle_2d",
            workload: BenchmarkWorkload::HeatPlaneTriangle2d(generate_heat_triangle_panel(
                profile.plane_triangle.nx,
                profile.plane_triangle.ny,
                profile.plane_triangle.width,
                profile.plane_triangle.height,
            )),
        },
        BenchmarkFamily::ElectrostaticPlaneTriangle2d => BenchmarkCase {
            id,
            family: "electrostatic_plane_triangle_2d",
            workload: BenchmarkWorkload::ElectrostaticPlaneTriangle2d(
                generate_electrostatic_triangle_panel(
                    profile.plane_triangle.nx,
                    profile.plane_triangle.ny,
                    profile.plane_triangle.width,
                    profile.plane_triangle.height,
                ),
            ),
        },
        BenchmarkFamily::ElectrostaticPlaneQuad2d => BenchmarkCase {
            id,
            family: "electrostatic_plane_quad_2d",
            workload: BenchmarkWorkload::ElectrostaticPlaneQuad2d(
                generate_electrostatic_quad_panel(
                    profile.plane_quad.nx,
                    profile.plane_quad.ny,
                    profile.plane_quad.width,
                    profile.plane_quad.height,
                ),
            ),
        },
        BenchmarkFamily::MagnetostaticPlaneTriangle2d => BenchmarkCase {
            id,
            family: "magnetostatic_plane_triangle_2d",
            workload: BenchmarkWorkload::MagnetostaticPlaneTriangle2d(
                generate_magnetostatic_triangle_panel(
                    profile.plane_triangle.nx,
                    profile.plane_triangle.ny,
                    profile.plane_triangle.width,
                    profile.plane_triangle.height,
                ),
            ),
        },
        BenchmarkFamily::MagnetostaticPlaneQuad2d => BenchmarkCase {
            id,
            family: "magnetostatic_plane_quad_2d",
            workload: BenchmarkWorkload::MagnetostaticPlaneQuad2d(
                generate_magnetostatic_quad_panel(
                    profile.plane_quad.nx,
                    profile.plane_quad.ny,
                    profile.plane_quad.width,
                    profile.plane_quad.height,
                ),
            ),
        },
        BenchmarkFamily::StokesFlowPlaneQuad2d => BenchmarkCase {
            id,
            family: "stokes_flow_plane_quad_2d",
            workload: BenchmarkWorkload::StokesFlowPlaneQuad2d(generate_stokes_quad_panel(
                profile.plane_quad.nx,
                profile.plane_quad.ny,
                profile.plane_quad.width,
                profile.plane_quad.height,
            )),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BenchmarkCatalogSpec, BenchmarkFamily, BenchmarkMatrixSpec, CaseTemplateSpec,
        resolve_matrix_templates,
    };

    #[test]
    fn matrix_template_resolution_preserves_declared_order() {
        let spec = catalog_spec(vec![
            template("a", BenchmarkFamily::AxialBar),
            template("b", BenchmarkFamily::HeatBar1d),
            template("c", BenchmarkFamily::Frame2d),
        ]);
        let matrix = BenchmarkMatrixSpec {
            name: "ordered".to_string(),
            template_stems: vec!["c".to_string(), "a".to_string()],
            owned_templates: vec![],
        };

        let stems = resolve_matrix_templates(&spec, &matrix)
            .into_iter()
            .map(|template| template.stem.as_str())
            .collect::<Vec<_>>();

        assert_eq!(stems, vec!["c", "a"]);
    }

    #[test]
    #[should_panic(expected = "benchmark matrix 'broken' references missing template 'missing'")]
    fn matrix_template_resolution_rejects_missing_stems() {
        let spec = catalog_spec(vec![template("a", BenchmarkFamily::AxialBar)]);
        let matrix = BenchmarkMatrixSpec {
            name: "broken".to_string(),
            template_stems: vec!["missing".to_string()],
            owned_templates: vec![],
        };

        let _ = resolve_matrix_templates(&spec, &matrix);
    }

    fn catalog_spec(templates: Vec<CaseTemplateSpec>) -> BenchmarkCatalogSpec {
        BenchmarkCatalogSpec {
            templates,
            matrices: vec![],
            profiles: vec![],
        }
    }

    fn template(stem: &str, family: BenchmarkFamily) -> CaseTemplateSpec {
        CaseTemplateSpec {
            stem: stem.to_string(),
            family,
        }
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
