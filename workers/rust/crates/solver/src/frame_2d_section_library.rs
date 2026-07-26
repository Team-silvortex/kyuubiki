use kyuubiki_protocol::{
    Frame2dBilinearKinematicMaterialInput, Frame2dSectionFiberInput, Frame2dSectionLibraryInput,
};

use crate::frame_2d_section_polygon::polygon_fibers;

struct FiberRegion {
    y_min: f64,
    y_max: f64,
    width: f64,
    fiber_count: usize,
}

pub(crate) fn resolve_section_fibers(
    material: &Frame2dBilinearKinematicMaterialInput,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    let Some(section) = &material.section_library else {
        return Ok(material.section_fibers.clone());
    };
    if !material.section_fibers.is_empty() {
        return Err(format!(
            "frame 2d material '{}' cannot combine section_library with explicit section_fibers",
            material.element_id
        ));
    }
    match section {
        Frame2dSectionLibraryInput::Rectangle {
            width,
            depth,
            fiber_count,
        } => rectangle_fibers(&material.element_id, *width, *depth, *fiber_count),
        Frame2dSectionLibraryInput::ISection {
            depth,
            flange_width,
            flange_thickness,
            web_thickness,
            fibers_per_flange,
            web_fiber_count,
        } => i_section_fibers(
            &material.element_id,
            *depth,
            *flange_width,
            *flange_thickness,
            *web_thickness,
            *fibers_per_flange,
            *web_fiber_count,
        ),
        Frame2dSectionLibraryInput::Circular {
            radius,
            fiber_count,
        } => circular_fibers(&material.element_id, *radius, *fiber_count),
        Frame2dSectionLibraryInput::HollowBox {
            width,
            depth,
            wall_thickness,
            fibers_per_flange,
            web_fiber_count,
        } => hollow_box_fibers(
            &material.element_id,
            *width,
            *depth,
            *wall_thickness,
            *fibers_per_flange,
            *web_fiber_count,
        ),
        Frame2dSectionLibraryInput::TSection {
            depth,
            flange_width,
            flange_thickness,
            web_thickness,
            flange_fiber_count,
            web_fiber_count,
        } => t_section_fibers(
            &material.element_id,
            *depth,
            *flange_width,
            *flange_thickness,
            *web_thickness,
            *flange_fiber_count,
            *web_fiber_count,
        ),
        Frame2dSectionLibraryInput::Layered { layers } => {
            layered_fibers(&material.element_id, layers)
        }
        Frame2dSectionLibraryInput::Polygon {
            vertices,
            fiber_count,
        } => polygon_fibers(&material.element_id, vertices, *fiber_count),
    }
}

fn rectangle_fibers(
    element_id: &str,
    width: f64,
    depth: f64,
    fiber_count: usize,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    require_positive(element_id, "width", width)?;
    require_positive(element_id, "depth", depth)?;
    require_total_fiber_count(element_id, fiber_count)?;
    corrected_region_fibers(
        &[FiberRegion {
            y_min: -0.5 * depth,
            y_max: 0.5 * depth,
            width,
            fiber_count,
        }],
        width * depth,
        width * depth.powi(3) / 12.0,
    )
}

#[allow(clippy::too_many_arguments)]
fn i_section_fibers(
    element_id: &str,
    depth: f64,
    flange_width: f64,
    flange_thickness: f64,
    web_thickness: f64,
    fibers_per_flange: usize,
    web_fiber_count: usize,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    require_positive(element_id, "depth", depth)?;
    require_positive(element_id, "flange_width", flange_width)?;
    require_positive(element_id, "flange_thickness", flange_thickness)?;
    require_positive(element_id, "web_thickness", web_thickness)?;
    if flange_thickness >= 0.5 * depth {
        return Err(format!(
            "frame 2d material '{element_id}' i_section flange_thickness must be less than half the depth"
        ));
    }
    if web_thickness > flange_width {
        return Err(format!(
            "frame 2d material '{element_id}' i_section web_thickness must not exceed flange_width"
        ));
    }
    require_region_fiber_count(element_id, fibers_per_flange)?;
    require_region_fiber_count(element_id, web_fiber_count)?;
    let total_fibers = 2 * fibers_per_flange + web_fiber_count;
    require_total_fiber_count(element_id, total_fibers)?;
    let web_depth = depth - 2.0 * flange_thickness;
    let regions = [
        FiberRegion {
            y_min: -0.5 * depth,
            y_max: -0.5 * web_depth,
            width: flange_width,
            fiber_count: fibers_per_flange,
        },
        FiberRegion {
            y_min: -0.5 * web_depth,
            y_max: 0.5 * web_depth,
            width: web_thickness,
            fiber_count: web_fiber_count,
        },
        FiberRegion {
            y_min: 0.5 * web_depth,
            y_max: 0.5 * depth,
            width: flange_width,
            fiber_count: fibers_per_flange,
        },
    ];
    let area = 2.0 * flange_width * flange_thickness + web_thickness * web_depth;
    let inertia =
        (flange_width * depth.powi(3) - (flange_width - web_thickness) * web_depth.powi(3)) / 12.0;
    corrected_region_fibers(&regions, area, inertia)
}

fn circular_fibers(
    element_id: &str,
    radius: f64,
    fiber_count: usize,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    require_positive(element_id, "radius", radius)?;
    require_total_fiber_count(element_id, fiber_count)?;
    let layer_depth = 2.0 * radius / fiber_count as f64;
    let mut fibers = Vec::with_capacity(fiber_count);
    for index in 0..fiber_count {
        let y_min = -radius + index as f64 * layer_depth;
        let y_max = y_min + layer_depth;
        let area = circle_area_primitive(radius, y_max) - circle_area_primitive(radius, y_min);
        let first_moment = circle_first_moment_primitive(radius, y_max)
            - circle_first_moment_primitive(radius, y_min);
        fibers.push(Frame2dSectionFiberInput {
            y: first_moment / area,
            area,
            initial_axial_stress: 0.0,
        });
    }
    corrected_fibers(
        fibers,
        std::f64::consts::PI * radius.powi(2),
        std::f64::consts::PI * radius.powi(4) / 4.0,
    )
}

#[allow(clippy::too_many_arguments)]
fn hollow_box_fibers(
    element_id: &str,
    width: f64,
    depth: f64,
    wall_thickness: f64,
    fibers_per_flange: usize,
    web_fiber_count: usize,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    require_positive(element_id, "width", width)?;
    require_positive(element_id, "depth", depth)?;
    require_positive(element_id, "wall_thickness", wall_thickness)?;
    if 2.0 * wall_thickness >= width.min(depth) {
        return Err(format!(
            "frame 2d material '{element_id}' hollow_box wall_thickness must leave positive inner width and depth"
        ));
    }
    require_region_fiber_count(element_id, fibers_per_flange)?;
    require_region_fiber_count(element_id, web_fiber_count)?;
    require_total_fiber_count(element_id, 2 * fibers_per_flange + web_fiber_count)?;
    let inner_width = width - 2.0 * wall_thickness;
    let inner_depth = depth - 2.0 * wall_thickness;
    let regions = [
        FiberRegion {
            y_min: -0.5 * depth,
            y_max: -0.5 * inner_depth,
            width,
            fiber_count: fibers_per_flange,
        },
        FiberRegion {
            y_min: -0.5 * inner_depth,
            y_max: 0.5 * inner_depth,
            width: 2.0 * wall_thickness,
            fiber_count: web_fiber_count,
        },
        FiberRegion {
            y_min: 0.5 * inner_depth,
            y_max: 0.5 * depth,
            width,
            fiber_count: fibers_per_flange,
        },
    ];
    corrected_region_fibers(
        &regions,
        width * depth - inner_width * inner_depth,
        (width * depth.powi(3) - inner_width * inner_depth.powi(3)) / 12.0,
    )
}

#[allow(clippy::too_many_arguments)]
fn t_section_fibers(
    element_id: &str,
    depth: f64,
    flange_width: f64,
    flange_thickness: f64,
    web_thickness: f64,
    flange_fiber_count: usize,
    web_fiber_count: usize,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    require_positive(element_id, "depth", depth)?;
    require_positive(element_id, "flange_width", flange_width)?;
    require_positive(element_id, "flange_thickness", flange_thickness)?;
    require_positive(element_id, "web_thickness", web_thickness)?;
    if flange_thickness >= depth {
        return Err(format!(
            "frame 2d material '{element_id}' t_section flange_thickness must be less than depth"
        ));
    }
    if web_thickness > flange_width {
        return Err(format!(
            "frame 2d material '{element_id}' t_section web_thickness must not exceed flange_width"
        ));
    }
    require_region_fiber_count(element_id, flange_fiber_count)?;
    require_region_fiber_count(element_id, web_fiber_count)?;
    require_total_fiber_count(element_id, flange_fiber_count + web_fiber_count)?;
    let web_depth = depth - flange_thickness;
    let web_area = web_thickness * web_depth;
    let flange_area = flange_width * flange_thickness;
    let web_y = -0.5 * flange_thickness;
    let flange_y = 0.5 * (depth - flange_thickness);
    let area = web_area + flange_area;
    let centroid = (web_area * web_y + flange_area * flange_y) / area;
    let inertia = web_thickness * web_depth.powi(3) / 12.0
        + web_area * (web_y - centroid).powi(2)
        + flange_width * flange_thickness.powi(3) / 12.0
        + flange_area * (flange_y - centroid).powi(2);
    corrected_region_fibers(
        &[
            FiberRegion {
                y_min: -0.5 * depth,
                y_max: 0.5 * depth - flange_thickness,
                width: web_thickness,
                fiber_count: web_fiber_count,
            },
            FiberRegion {
                y_min: 0.5 * depth - flange_thickness,
                y_max: 0.5 * depth,
                width: flange_width,
                fiber_count: flange_fiber_count,
            },
        ],
        area,
        inertia,
    )
}

fn layered_fibers(
    element_id: &str,
    layers: &[kyuubiki_protocol::Frame2dSectionLayerInput],
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    if layers.is_empty() || layers.len() > 16 {
        return Err(format!(
            "frame 2d material '{element_id}' layered section must contain between 1 and 16 layers"
        ));
    }
    let mut regions = Vec::with_capacity(layers.len());
    let mut total_fibers = 0_usize;
    for (index, layer) in layers.iter().enumerate() {
        require_positive(element_id, &format!("layers[{index}].width"), layer.width)?;
        if !(layer.y_min.is_finite() && layer.y_max.is_finite() && layer.y_min < layer.y_max) {
            return Err(format!(
                "frame 2d material '{element_id}' layered section layers[{index}] requires finite y_min < y_max"
            ));
        }
        require_region_fiber_count(element_id, layer.fiber_count)?;
        total_fibers = total_fibers.checked_add(layer.fiber_count).ok_or_else(|| {
            format!("frame 2d material '{element_id}' layered section fiber count overflow")
        })?;
        regions.push(FiberRegion {
            y_min: layer.y_min,
            y_max: layer.y_max,
            width: layer.width,
            fiber_count: layer.fiber_count,
        });
    }
    require_total_fiber_count(element_id, total_fibers)?;
    regions.sort_by(|left, right| left.y_min.total_cmp(&right.y_min));
    for pair in regions.windows(2) {
        if pair[1].y_min < pair[0].y_max {
            return Err(format!(
                "frame 2d material '{element_id}' layered section layers must not overlap"
            ));
        }
    }
    let area = regions
        .iter()
        .map(|region| region.width * (region.y_max - region.y_min))
        .sum::<f64>();
    let first_moment = regions
        .iter()
        .map(|region| {
            let layer_area = region.width * (region.y_max - region.y_min);
            layer_area * 0.5 * (region.y_min + region.y_max)
        })
        .sum::<f64>();
    let centroid = first_moment / area;
    let inertia = regions
        .iter()
        .map(|region| {
            let depth = region.y_max - region.y_min;
            let layer_area = region.width * depth;
            let layer_y = 0.5 * (region.y_min + region.y_max);
            region.width * depth.powi(3) / 12.0 + layer_area * (layer_y - centroid).powi(2)
        })
        .sum::<f64>();
    corrected_region_fibers(&regions, area, inertia)
}

fn corrected_region_fibers(
    regions: &[FiberRegion],
    target_area: f64,
    target_inertia: f64,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    let mut fibers = Vec::new();
    for region in regions {
        let thickness = (region.y_max - region.y_min) / region.fiber_count as f64;
        for index in 0..region.fiber_count {
            fibers.push(Frame2dSectionFiberInput {
                y: region.y_min + (index as f64 + 0.5) * thickness,
                area: region.width * thickness,
                initial_axial_stress: 0.0,
            });
        }
    }
    corrected_fibers(fibers, target_area, target_inertia)
}

pub(crate) fn corrected_fibers(
    mut fibers: Vec<Frame2dSectionFiberInput>,
    target_area: f64,
    target_inertia: f64,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    let raw_area = fibers.iter().map(|fiber| fiber.area).sum::<f64>();
    let area_scale = target_area / raw_area;
    for fiber in &mut fibers {
        fiber.area *= area_scale;
    }
    let centroid = fibers.iter().map(|fiber| fiber.area * fiber.y).sum::<f64>() / target_area;
    let raw_inertia = fibers
        .iter()
        .map(|fiber| fiber.area * (fiber.y - centroid).powi(2))
        .sum::<f64>();
    if !(raw_inertia.is_finite() && raw_inertia > 0.0) {
        return Err("frame 2d section library generated zero or nonfinite inertia".into());
    }
    let y_scale = (target_inertia / raw_inertia).sqrt();
    for fiber in &mut fibers {
        fiber.y = (fiber.y - centroid) * y_scale;
    }
    Ok(fibers)
}

fn circle_area_primitive(radius: f64, y: f64) -> f64 {
    let ratio = (y / radius).clamp(-1.0, 1.0);
    y * (radius.powi(2) - y.powi(2)).max(0.0).sqrt() + radius.powi(2) * ratio.asin()
}

fn circle_first_moment_primitive(radius: f64, y: f64) -> f64 {
    -(2.0 / 3.0) * (radius.powi(2) - y.powi(2)).max(0.0).powf(1.5)
}

fn require_positive(element_id: &str, field: &str, value: f64) -> Result<(), String> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!(
            "frame 2d material '{element_id}' section_library {field} must be positive and finite"
        ))
    }
}

fn require_region_fiber_count(element_id: &str, count: usize) -> Result<(), String> {
    if (1..=32).contains(&count) {
        Ok(())
    } else {
        Err(format!(
            "frame 2d material '{element_id}' section_library fiber counts must be between 1 and 32"
        ))
    }
}

fn require_total_fiber_count(element_id: &str, count: usize) -> Result<(), String> {
    if (2..=32).contains(&count) {
        Ok(())
    } else {
        Err(format!(
            "frame 2d material '{element_id}' section_library must generate between 2 and 32 fibers"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_section_fibers;
    use kyuubiki_protocol::{
        Frame2dBilinearKinematicMaterialInput, Frame2dSectionLayerInput, Frame2dSectionLibraryInput,
    };

    #[test]
    fn rectangle_library_preserves_exact_area_centroid_and_inertia() {
        let fibers = resolve_section_fibers(&material(Frame2dSectionLibraryInput::Rectangle {
            width: 0.2,
            depth: 0.4,
            fiber_count: 8,
        }))
        .expect("rectangle fibers");

        assert_section_properties(&fibers, 0.08, 0.2 * 0.4_f64.powi(3) / 12.0);
        assert_eq!(fibers.len(), 8);
    }

    #[test]
    fn i_section_library_preserves_exact_area_centroid_and_inertia() {
        let section = Frame2dSectionLibraryInput::ISection {
            depth: 0.6,
            flange_width: 0.24,
            flange_thickness: 0.04,
            web_thickness: 0.02,
            fibers_per_flange: 4,
            web_fiber_count: 8,
        };
        let fibers = resolve_section_fibers(&material(section)).expect("i-section fibers");
        let web_depth = 0.52_f64;
        let area = 2.0 * 0.24 * 0.04 + 0.02 * web_depth;
        let inertia = (0.24 * 0.6_f64.powi(3) - (0.24 - 0.02) * web_depth.powi(3)) / 12.0;

        assert_section_properties(&fibers, area, inertia);
        assert_eq!(fibers.len(), 16);
    }

    #[test]
    fn circular_library_preserves_exact_area_centroid_and_inertia() {
        let radius = 0.15_f64;
        let fibers = resolve_section_fibers(&material(Frame2dSectionLibraryInput::Circular {
            radius,
            fiber_count: 16,
        }))
        .expect("circular fibers");

        assert_section_properties(
            &fibers,
            std::f64::consts::PI * radius.powi(2),
            std::f64::consts::PI * radius.powi(4) / 4.0,
        );
        assert_eq!(fibers.len(), 16);
    }

    #[test]
    fn hollow_box_library_preserves_exact_area_centroid_and_inertia() {
        let width = 0.24_f64;
        let depth = 0.4_f64;
        let wall = 0.02_f64;
        let inner_width = width - 2.0 * wall;
        let inner_depth = depth - 2.0 * wall;
        let fibers = resolve_section_fibers(&material(Frame2dSectionLibraryInput::HollowBox {
            width,
            depth,
            wall_thickness: wall,
            fibers_per_flange: 4,
            web_fiber_count: 8,
        }))
        .expect("hollow-box fibers");

        assert_section_properties(
            &fibers,
            width * depth - inner_width * inner_depth,
            (width * depth.powi(3) - inner_width * inner_depth.powi(3)) / 12.0,
        );
        assert_eq!(fibers.len(), 16);
    }

    #[test]
    fn t_section_library_recenters_the_asymmetric_geometry() {
        let depth = 0.4_f64;
        let flange_width = 0.24_f64;
        let flange_thickness = 0.04_f64;
        let web_thickness = 0.02_f64;
        let web_depth = depth - flange_thickness;
        let web_area = web_thickness * web_depth;
        let flange_area = flange_width * flange_thickness;
        let area = web_area + flange_area;
        let web_y = -0.5 * flange_thickness;
        let flange_y = 0.5 * (depth - flange_thickness);
        let centroid = (web_area * web_y + flange_area * flange_y) / area;
        let inertia = web_thickness * web_depth.powi(3) / 12.0
            + web_area * (web_y - centroid).powi(2)
            + flange_width * flange_thickness.powi(3) / 12.0
            + flange_area * (flange_y - centroid).powi(2);
        let fibers = resolve_section_fibers(&material(Frame2dSectionLibraryInput::TSection {
            depth,
            flange_width,
            flange_thickness,
            web_thickness,
            flange_fiber_count: 4,
            web_fiber_count: 8,
        }))
        .expect("t-section fibers");

        assert_section_properties(&fibers, area, inertia);
        assert_eq!(fibers.len(), 12);
        assert!(fibers.first().unwrap().y.abs() != fibers.last().unwrap().y.abs());
    }

    #[test]
    fn layered_library_sorts_and_recenters_an_asymmetric_profile() {
        let layers = vec![
            layer(0.15, 0.3, 0.1, 3),
            layer(-0.3, -0.1, 0.05, 4),
            layer(-0.1, 0.15, 0.02, 5),
        ];
        let (area, inertia) = layered_properties(&layers);
        let fibers =
            resolve_section_fibers(&material(Frame2dSectionLibraryInput::Layered { layers }))
                .expect("layered fibers");

        assert_section_properties(&fibers, area, inertia);
        assert_eq!(fibers.len(), 12);
        assert!(fibers.windows(2).all(|pair| pair[0].y < pair[1].y));
        assert!(fibers.first().unwrap().y.abs() != fibers.last().unwrap().y.abs());
    }

    #[test]
    fn section_library_rejects_invalid_geometry_and_fiber_budgets() {
        let invalid_geometry = material(Frame2dSectionLibraryInput::ISection {
            depth: 0.6,
            flange_width: 0.24,
            flange_thickness: 0.3,
            web_thickness: 0.02,
            fibers_per_flange: 4,
            web_fiber_count: 8,
        });
        let error = resolve_section_fibers(&invalid_geometry).unwrap_err();
        assert!(error.contains("less than half the depth"));

        let oversized = material(Frame2dSectionLibraryInput::ISection {
            depth: 0.6,
            flange_width: 0.24,
            flange_thickness: 0.04,
            web_thickness: 0.02,
            fibers_per_flange: 13,
            web_fiber_count: 8,
        });
        let error = resolve_section_fibers(&oversized).unwrap_err();
        assert!(error.contains("between 2 and 32 fibers"));

        let closed_box = material(Frame2dSectionLibraryInput::HollowBox {
            width: 0.2,
            depth: 0.3,
            wall_thickness: 0.1,
            fibers_per_flange: 4,
            web_fiber_count: 8,
        });
        let error = resolve_section_fibers(&closed_box).unwrap_err();
        assert!(error.contains("positive inner width and depth"));

        let overlapping = material(Frame2dSectionLibraryInput::Layered {
            layers: vec![layer(-0.2, 0.1, 0.05, 4), layer(0.0, 0.2, 0.05, 4)],
        });
        let error = resolve_section_fibers(&overlapping).unwrap_err();
        assert!(error.contains("must not overlap"));
    }

    fn material(
        section_library: Frame2dSectionLibraryInput,
    ) -> Frame2dBilinearKinematicMaterialInput {
        Frame2dBilinearKinematicMaterialInput {
            element_id: "section".into(),
            yield_strength: 250.0e6,
            hardening_ratio: 0.02,
            initial_axial_stress: 0.0,
            section_library: Some(section_library),
            section_fibers: Vec::new(),
            longitudinal_integration_points: 2,
            adaptive_longitudinal_integration: false,
            longitudinal_integration_tolerance: 1.0e-3,
        }
    }

    fn assert_section_properties(
        fibers: &[kyuubiki_protocol::Frame2dSectionFiberInput],
        expected_area: f64,
        expected_inertia: f64,
    ) {
        let area = fibers.iter().map(|fiber| fiber.area).sum::<f64>();
        let first_moment = fibers.iter().map(|fiber| fiber.area * fiber.y).sum::<f64>();
        let inertia = fibers
            .iter()
            .map(|fiber| fiber.area * fiber.y.powi(2))
            .sum::<f64>();
        assert!((area - expected_area).abs() < 1.0e-14);
        assert!(first_moment.abs() < 1.0e-14);
        assert!((inertia - expected_inertia).abs() < 1.0e-14);
    }

    fn layer(y_min: f64, y_max: f64, width: f64, fiber_count: usize) -> Frame2dSectionLayerInput {
        Frame2dSectionLayerInput {
            y_min,
            y_max,
            width,
            fiber_count,
        }
    }

    fn layered_properties(layers: &[Frame2dSectionLayerInput]) -> (f64, f64) {
        let area = layers
            .iter()
            .map(|layer| layer.width * (layer.y_max - layer.y_min))
            .sum::<f64>();
        let centroid = layers
            .iter()
            .map(|layer| {
                layer.width * (layer.y_max - layer.y_min) * 0.5 * (layer.y_min + layer.y_max)
            })
            .sum::<f64>()
            / area;
        let inertia = layers
            .iter()
            .map(|layer| {
                let depth = layer.y_max - layer.y_min;
                let layer_area = layer.width * depth;
                let layer_y = 0.5 * (layer.y_min + layer.y_max);
                layer.width * depth.powi(3) / 12.0 + layer_area * (layer_y - centroid).powi(2)
            })
            .sum();
        (area, inertia)
    }
}
