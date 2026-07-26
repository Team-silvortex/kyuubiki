use crate::frame_2d_section_library::corrected_fibers;
use kyuubiki_protocol::{Frame2dSectionFiberInput, Frame2dSectionVertexInput};

const MAX_VERTICES: usize = 64;

pub(crate) fn polygon_fibers(
    element_id: &str,
    vertices: &[Frame2dSectionVertexInput],
    fiber_count: usize,
) -> Result<Vec<Frame2dSectionFiberInput>, String> {
    if !(3..=MAX_VERTICES).contains(&vertices.len()) {
        return Err(format!(
            "frame 2d material '{element_id}' polygon section must contain between 3 and {MAX_VERTICES} vertices"
        ));
    }
    if !(2..=32).contains(&fiber_count) {
        return Err(format!(
            "frame 2d material '{element_id}' polygon section fiber_count must be between 2 and 32"
        ));
    }
    if vertices
        .iter()
        .any(|vertex| !(vertex.y.is_finite() && vertex.z.is_finite()))
    {
        return Err(format!(
            "frame 2d material '{element_id}' polygon section vertices must be finite"
        ));
    }

    let scale = polygon_scale(vertices);
    if !(scale.is_finite() && scale > 0.0) {
        return Err(format!(
            "frame 2d material '{element_id}' polygon section must span a positive y or z range"
        ));
    }
    let tolerance = scale.max(1.0) * 1.0e-12;
    validate_edges(element_id, vertices, tolerance)?;
    validate_simple_polygon(element_id, vertices, tolerance)?;
    let (area, centroid_y, inertia) = polygon_properties(element_id, vertices, tolerance)?;

    let min_y = vertices
        .iter()
        .map(|vertex| vertex.y)
        .fold(f64::INFINITY, f64::min);
    let max_y = vertices
        .iter()
        .map(|vertex| vertex.y)
        .fold(f64::NEG_INFINITY, f64::max);
    let layer_depth = (max_y - min_y) / fiber_count as f64;
    let mut fibers = Vec::with_capacity(fiber_count);
    for index in 0..fiber_count {
        let y = min_y + (index as f64 + 0.5) * layer_depth;
        let width = horizontal_width(vertices, y, tolerance)?;
        if width > tolerance {
            fibers.push(Frame2dSectionFiberInput {
                y: y - centroid_y,
                area: width * layer_depth,
                initial_axial_stress: 0.0,
                material_id: None,
            });
        }
    }
    if fibers.len() < 2 {
        return Err(format!(
            "frame 2d material '{element_id}' polygon section generated fewer than two active fibers"
        ));
    }
    corrected_fibers(fibers, area, inertia)
}

fn polygon_scale(vertices: &[Frame2dSectionVertexInput]) -> f64 {
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_z, mut max_z) = (f64::INFINITY, f64::NEG_INFINITY);
    for vertex in vertices {
        min_y = min_y.min(vertex.y);
        max_y = max_y.max(vertex.y);
        min_z = min_z.min(vertex.z);
        max_z = max_z.max(vertex.z);
    }
    (max_y - min_y).max(max_z - min_z)
}

fn validate_edges(
    element_id: &str,
    vertices: &[Frame2dSectionVertexInput],
    tolerance: f64,
) -> Result<(), String> {
    for index in 0..vertices.len() {
        let next = (index + 1) % vertices.len();
        let dy = vertices[next].y - vertices[index].y;
        let dz = vertices[next].z - vertices[index].z;
        if dy.hypot(dz) <= tolerance {
            return Err(format!(
                "frame 2d material '{element_id}' polygon section has a duplicate or zero-length edge at vertex {index}"
            ));
        }
    }
    Ok(())
}

fn validate_simple_polygon(
    element_id: &str,
    vertices: &[Frame2dSectionVertexInput],
    tolerance: f64,
) -> Result<(), String> {
    let count = vertices.len();
    for first in 0..count {
        let first_next = (first + 1) % count;
        for second in (first + 1)..count {
            let second_next = (second + 1) % count;
            if first == second
                || first == second_next
                || first_next == second
                || first_next == second_next
            {
                continue;
            }
            if segments_intersect(
                &vertices[first],
                &vertices[first_next],
                &vertices[second],
                &vertices[second_next],
                tolerance,
            ) {
                return Err(format!(
                    "frame 2d material '{element_id}' polygon section edges must not self-intersect"
                ));
            }
        }
    }
    Ok(())
}

fn segments_intersect(
    a: &Frame2dSectionVertexInput,
    b: &Frame2dSectionVertexInput,
    c: &Frame2dSectionVertexInput,
    d: &Frame2dSectionVertexInput,
    tolerance: f64,
) -> bool {
    let ab_c = orientation(a, b, c);
    let ab_d = orientation(a, b, d);
    let cd_a = orientation(c, d, a);
    let cd_b = orientation(c, d, b);
    if ab_c.abs() <= tolerance && on_segment(a, b, c, tolerance)
        || ab_d.abs() <= tolerance && on_segment(a, b, d, tolerance)
        || cd_a.abs() <= tolerance && on_segment(c, d, a, tolerance)
        || cd_b.abs() <= tolerance && on_segment(c, d, b, tolerance)
    {
        return true;
    }
    (ab_c > tolerance && ab_d < -tolerance || ab_c < -tolerance && ab_d > tolerance)
        && (cd_a > tolerance && cd_b < -tolerance || cd_a < -tolerance && cd_b > tolerance)
}

fn orientation(
    a: &Frame2dSectionVertexInput,
    b: &Frame2dSectionVertexInput,
    c: &Frame2dSectionVertexInput,
) -> f64 {
    (b.z - a.z) * (c.y - a.y) - (b.y - a.y) * (c.z - a.z)
}

fn on_segment(
    a: &Frame2dSectionVertexInput,
    b: &Frame2dSectionVertexInput,
    point: &Frame2dSectionVertexInput,
    tolerance: f64,
) -> bool {
    point.z >= a.z.min(b.z) - tolerance
        && point.z <= a.z.max(b.z) + tolerance
        && point.y >= a.y.min(b.y) - tolerance
        && point.y <= a.y.max(b.y) + tolerance
}

fn polygon_properties(
    element_id: &str,
    vertices: &[Frame2dSectionVertexInput],
    tolerance: f64,
) -> Result<(f64, f64, f64), String> {
    let mut twice_area = 0.0;
    let mut first_y = 0.0;
    let mut second_y = 0.0;
    for index in 0..vertices.len() {
        let current = &vertices[index];
        let next = &vertices[(index + 1) % vertices.len()];
        let cross = current.z * next.y - next.z * current.y;
        twice_area += cross;
        first_y += (current.y + next.y) * cross;
        second_y += (current.y.powi(2) + current.y * next.y + next.y.powi(2)) * cross;
    }
    if twice_area.abs() <= tolerance.powi(2) {
        return Err(format!(
            "frame 2d material '{element_id}' polygon section area must be positive"
        ));
    }
    let orientation = twice_area.signum();
    let area = 0.5 * twice_area * orientation;
    let centroid_y = first_y * orientation / (6.0 * area);
    let origin_inertia = second_y * orientation / 12.0;
    let inertia = origin_inertia - area * centroid_y.powi(2);
    if !(area.is_finite()
        && area > tolerance.powi(2)
        && centroid_y.is_finite()
        && inertia.is_finite()
        && inertia > tolerance.powi(4))
    {
        return Err(format!(
            "frame 2d material '{element_id}' polygon section has degenerate area or bending inertia"
        ));
    }
    Ok((area, centroid_y, inertia))
}

fn horizontal_width(
    vertices: &[Frame2dSectionVertexInput],
    y: f64,
    tolerance: f64,
) -> Result<f64, String> {
    let mut intersections = Vec::new();
    for index in 0..vertices.len() {
        let current = &vertices[index];
        let next = &vertices[(index + 1) % vertices.len()];
        let (lower, upper) = if current.y < next.y {
            (current, next)
        } else {
            (next, current)
        };
        if y < lower.y || y >= upper.y || (upper.y - lower.y).abs() <= tolerance {
            continue;
        }
        let ratio = (y - lower.y) / (upper.y - lower.y);
        intersections.push(lower.z + ratio * (upper.z - lower.z));
    }
    intersections.sort_by(f64::total_cmp);
    if intersections.len() % 2 != 0 {
        return Err(
            "frame 2d polygon horizontal slicing produced an odd intersection count".into(),
        );
    }
    Ok(intersections
        .chunks_exact(2)
        .map(|pair| (pair[1] - pair[0]).max(0.0))
        .sum())
}

#[cfg(test)]
mod tests {
    use super::{polygon_fibers, polygon_properties};
    use kyuubiki_protocol::{Frame2dSectionFiberInput, Frame2dSectionVertexInput};

    #[test]
    fn concave_polygon_preserves_exact_area_centroid_and_inertia() {
        let vertices = l_section();
        let fibers = polygon_fibers("l-section", &vertices, 16).expect("polygon fibers");
        let web_inertia = 0.05 * 0.4_f64.powi(3) / 12.0;
        let flange_inertia = 0.15 * 0.1_f64.powi(3) / 12.0;
        let area = 0.05_f64 * 0.4 + 0.15 * 0.1;
        let centroid = 0.15_f64 * 0.1 * 0.15 / area;
        let inertia =
            web_inertia + flange_inertia + 0.15 * 0.1 * 0.15_f64.powi(2) - area * centroid.powi(2);

        assert_section_properties(&fibers, area, inertia);
        assert_eq!(fibers.len(), 16);
    }

    #[test]
    fn polygon_properties_are_orientation_independent() {
        let mut vertices = l_section();
        let forward = polygon_properties("forward", &vertices, 1.0e-12).unwrap();
        vertices.reverse();
        let reverse = polygon_properties("reverse", &vertices, 1.0e-12).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn polygon_rejects_self_intersections_and_degenerate_edges() {
        let bow_tie = vec![
            vertex(-0.2, -0.2),
            vertex(0.2, 0.2),
            vertex(-0.2, 0.2),
            vertex(0.2, -0.2),
        ];
        assert!(
            polygon_fibers("bow-tie", &bow_tie, 8)
                .unwrap_err()
                .contains("self-intersect")
        );
        let duplicate = vec![
            vertex(-0.2, 0.0),
            vertex(-0.2, 0.1),
            vertex(-0.2, 0.1),
            vertex(0.2, 0.0),
        ];
        assert!(
            polygon_fibers("duplicate", &duplicate, 8)
                .unwrap_err()
                .contains("zero-length edge")
        );
        assert!(
            polygon_fibers("budget", &l_section(), 1)
                .unwrap_err()
                .contains("between 2 and 32")
        );
    }

    fn l_section() -> Vec<Frame2dSectionVertexInput> {
        vec![
            vertex(-0.2, 0.0),
            vertex(-0.2, 0.05),
            vertex(0.1, 0.05),
            vertex(0.1, 0.2),
            vertex(0.2, 0.2),
            vertex(0.2, 0.0),
        ]
    }

    fn vertex(y: f64, z: f64) -> Frame2dSectionVertexInput {
        Frame2dSectionVertexInput { y, z }
    }

    fn assert_section_properties(
        fibers: &[Frame2dSectionFiberInput],
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
}
