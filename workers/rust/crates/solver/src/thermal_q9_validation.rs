use crate::solver_control::{SolverStage, checkpoint};
use kyuubiki_protocol::{SolveThermalQ9RectRequest, THERMAL_Q9_RECT_SCHEMA};
use std::collections::BTreeSet;

#[derive(Clone, Copy)]
pub(super) struct Rect {
    pub x: f64,
    pub y: f64,
    pub right: f64,
    pub top: f64,
    pub dx: f64,
    pub dy: f64,
}

pub(super) fn validate(r: &SolveThermalQ9RectRequest) -> Result<Vec<Rect>, String> {
    if r.schema_version != THERMAL_Q9_RECT_SCHEMA {
        return Err("Q9 schema version is unsupported".into());
    }
    if !(9..=4096).contains(&r.nodes.len()) || !(1..=512).contains(&r.elements.len()) {
        return Err("Q9 budget requires 9..4096 nodes and 1..512 elements".into());
    }
    let mut names = BTreeSet::new();
    let mut coordinates = BTreeSet::new();
    let bits = |x: f64| if x == 0.0 { 0 } else { x.to_bits() };
    for (i, n) in r.nodes.iter().enumerate() {
        checkpoint(SolverStage::ElementPrecompute, i)?;
        if n.id.is_empty() || n.id.len() > 128 || !names.insert(&n.id) {
            return Err("Q9 node IDs must be unique, nonempty and <=128 bytes".into());
        }
        if [n.x, n.y, n.load_x, n.load_y, n.temperature_delta]
            .iter()
            .any(|x| !x.is_finite())
        {
            return Err("Q9 node coordinate, load or temperature is nonfinite".into());
        }
        if !coordinates.insert((bits(n.x), bits(n.y))) {
            return Err("Q9 coincident nodes must share one index".into());
        }
    }
    names.clear();
    let mut used = vec![false; r.nodes.len()];
    let mut rects = vec![];
    for (i, e) in r.elements.iter().enumerate() {
        checkpoint(SolverStage::ElementPrecompute, i)?;
        if e.id.is_empty() || e.id.len() > 128 || !names.insert(&e.id) {
            return Err("Q9 element IDs must be unique, nonempty and <=128 bytes".into());
        }
        if e.nodes.iter().any(|n| *n >= r.nodes.len())
            || e.nodes.iter().collect::<BTreeSet<_>>().len() != 9
        {
            return Err("Q9 element requires nine distinct in-range node indices".into());
        }
        if [e.thickness, e.youngs_modulus]
            .iter()
            .any(|v| !v.is_finite() || *v <= 0.0)
            || !e.poisson_ratio.is_finite()
            || e.poisson_ratio <= -1.0
            || e.poisson_ratio >= 0.5
            || !e.thermal_expansion.is_finite()
        {
            return Err("Q9 invalid plane-stress material".into());
        }
        let a = &r.nodes[e.nodes[0]];
        let z = &r.nodes[e.nodes[8]];
        let (dx, dy) = (z.x - a.x, z.y - a.y);
        if !dx.is_finite()
            || !dy.is_finite()
            || dx <= 0.0
            || dy <= 0.0
            || dx <= 64.0 * f64::EPSILON * a.x.abs().max(z.x.abs())
            || dy <= 64.0 * f64::EPSILON * a.y.abs().max(z.y.abs())
            || !(dx * dy).is_finite()
            || dx * dy <= 0.0
        {
            return Err("Q9 rectangle has nonpositive or unrepresentable Jacobian".into());
        }
        for (k, &id) in e.nodes.iter().enumerate() {
            let n = &r.nodes[id];
            if (n.x - (a.x + (k % 3) as f64 * dx / 2.0)).abs() > 1e-12 * dx
                || (n.y - (a.y + (k / 3) as f64 * dy / 2.0)).abs() > 1e-12 * dy
            {
                return Err(
                    "Q9 requires a row-major axis-aligned rectangular 3x3 node grid".into(),
                );
            }
            used[id] = true;
        }
        rects.push(Rect {
            x: a.x,
            y: a.y,
            right: z.x,
            top: z.y,
            dx,
            dy,
        });
    }
    if used.contains(&false) {
        return Err("Q9 model contains unused nodes".into());
    }
    let mut adjacency = vec![vec![]; rects.len()];
    for i in 0..rects.len() {
        checkpoint(SolverStage::ElementPrecompute, i)?;
        let a = rects[i];
        for j in i + 1..rects.len() {
            let b = rects[j];
            // Reconstructing bounds as origin + extent can create false overlap
            // at a shared edge through cancellation (e.g. -0.3 + (0.1 - -0.3)).
            let ox = a.right.min(b.right) - a.x.max(b.x);
            let oy = a.top.min(b.top) - a.y.max(b.y);
            if ox > 0.0 && oy > 0.0 {
                return Err("Q9 overlapping elements are unsupported".into());
            }
            let vertical = ox == 0.0 && oy > 0.0;
            let horizontal = oy == 0.0 && ox > 0.0;
            if vertical || horizontal {
                if (vertical && (a.y != b.y || a.top != b.top))
                    || (horizontal && (a.x != b.x || a.right != b.right))
                {
                    return Err("Q9 hanging or partially shared edges are unsupported".into());
                }
                let common = r.elements[i]
                    .nodes
                    .iter()
                    .filter(|n| r.elements[j].nodes.contains(n))
                    .count();
                if common != 3 {
                    return Err("Q9 shared edges must share both corners and midside index".into());
                }
                adjacency[i].push(j);
                adjacency[j].push(i);
            }
        }
    }
    let mut seen = vec![false; rects.len()];
    let mut pending = vec![0];
    seen[0] = true;
    while let Some(i) = pending.pop() {
        for &j in &adjacency[i] {
            if !seen[j] {
                seen[j] = true;
                pending.push(j);
            }
        }
    }
    if seen.contains(&false) {
        return Err("Q9 requires one edge-connected mesh".into());
    }
    validate_restraints(r)?;
    Ok(rects)
}

fn validate_restraints(r: &SolveThermalQ9RectRequest) -> Result<(), String> {
    let origin = &r.nodes[0];
    let scale = r
        .nodes
        .iter()
        .map(|n| (n.x - origin.x).hypot(n.y - origin.y))
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 {
        return Err("Q9 support geometry is not representable".into());
    }
    let mut basis: Vec<[f64; 3]> = vec![];
    for n in &r.nodes {
        for (fixed, mut v) in [
            (n.fix_x, [1.0, 0.0, -(n.y - origin.y) / scale]),
            (n.fix_y, [0.0, 1.0, (n.x - origin.x) / scale]),
        ] {
            if !fixed {
                continue;
            }
            for b in &basis {
                let dot: f64 = (0..3).map(|i| v[i] * b[i]).sum();
                for i in 0..3 {
                    v[i] -= dot * b[i];
                }
            }
            let norm = v[0].hypot(v[1]).hypot(v[2]);
            if norm > 1e-12 {
                basis.push(v.map(|x| x / norm));
            }
            if basis.len() == 3 {
                return Ok(());
            }
        }
    }
    Err("Q9 supports do not remove all three rigid-body modes".into())
}
