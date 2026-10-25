use serde_json::{Value, json};

#[path = "result_contract.rs"]
mod result_contract;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
type AssembledReference = (Vec<Vec<f64>>, Vec<f64>, Vec<usize>);

/// Independently assemble the *input* geometry and original diagonal masses.
/// No production solver, stiffness, residual or normalization helper is imported.
fn assemble(input: &Value) -> Result<AssembledReference> {
    if input != &super::model::model(false) && input != &super::model::model(true) {
        return Err("reference only supports the two explicit synthetic model orderings".into());
    }
    let nodes = input["nodes"].as_array().ok_or("nodes missing")?;
    let mut k = vec![vec![0.0; 9]; 9];
    let mut mass = vec![0.0; 9];
    for element in input["elements"].as_array().ok_or("elements missing")? {
        let i = element["node_i"].as_u64().ok_or("node i missing")? as usize;
        let j = element["node_j"].as_u64().ok_or("node j missing")? as usize;
        let length = number(&nodes[j], "x")? - number(&nodes[i], "x")?;
        let rigidity = number(element, "youngs_modulus")? * number(element, "moment_of_inertia")?;
        let a = 12.0 * rigidity / length.powi(3);
        let b = 6.0 * rigidity / length.powi(2);
        let c = 4.0 * rigidity / length;
        let d = 2.0 * rigidity / length;
        let local = [[a, b, -a, b], [b, c, -b, d], [-a, -b, a, -b], [b, d, -b, c]];
        let dofs = [3 * i + 1, 3 * i + 2, 3 * j + 1, 3 * j + 2];
        for row in 0..4 {
            for column in 0..4 {
                k[dofs[row]][dofs[column]] += local[row][column];
            }
        }
        let total = number(element, "density")? * number(element, "area")? * length;
        for node in [i, j] {
            mass[3 * node] += total / 2.0;
            mass[3 * node + 1] += total / 2.0;
            mass[3 * node + 2] += total * length.powi(2) / 24.0;
        }
    }
    let free = nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node["fix_y"] == false)
        .flat_map(|(i, _)| [3 * i + 1, 3 * i + 2])
        .collect();
    Ok((k, mass, free))
}

// Four-DOF reference only: symmetric LDL inertia and bracketed roots, not the
// production modal eigensolver or a solver implementation offered by this SDK.
fn spectrum(k: &[Vec<f64>], mass: &[f64], free: &[usize]) -> Vec<f64> {
    let n = free.len();
    let a: Vec<Vec<_>> = free
        .iter()
        .map(|&i| {
            free.iter()
                .map(|&j| k[i][j] / (mass[i] * mass[j]).sqrt())
                .collect()
        })
        .collect();
    let upper = a
        .iter()
        .map(|row| row.iter().map(|v| v.abs()).sum::<f64>())
        .fold(0.0, f64::max)
        * 1.01;
    (0..n)
        .map(|index| {
            let (mut low, mut high) = (0.0, upper);
            for _ in 0..100 {
                let mid = low + (high - low) / 2.0;
                let mut l = vec![vec![0.0; n]; n];
                let mut diagonal = vec![0.0; n];
                for i in 0..n {
                    let mut pivot = a[i][i] - mid;
                    for (j, &value) in diagonal.iter().enumerate().take(i) {
                        pivot -= l[i][j] * l[i][j] * value;
                    }
                    if pivot.abs() < 1e-30 {
                        pivot = -1e-30;
                    }
                    diagonal[i] = pivot;
                    for j in i + 1..n {
                        let mut entry = a[j][i];
                        for (p, &value) in diagonal.iter().enumerate().take(i) {
                            entry -= l[j][p] * l[i][p] * value;
                        }
                        l[j][i] = entry / pivot;
                    }
                }
                if diagonal.iter().filter(|&&v| v < 0.0).count() <= index {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            (low + high) / 2.0
        })
        .collect()
}

pub fn verify(input: &Value, output: &Value) -> Result<Value> {
    let (k, mass, free) = assemble(input)?;
    if output["free_dofs"] != json!(free) {
        return Err("wrong free DOFs".into());
    }
    let modes = output["modes"].as_array().ok_or("missing modes")?;
    if modes.len() != 4 {
        return Err("four modes required, no partial publication".into());
    }
    result_contract::validate(input, output)?;
    let roots = spectrum(&k, &mass, &free);
    let mut checked_shapes = Vec::<Vec<f64>>::new();
    let mut residuals = vec![];
    for (index, mode) in modes.iter().enumerate() {
        let lambda = number(mode, "eigenvalue_rad_s_squared")?;
        if mode["index"] != index || lambda <= 0.0 || (lambda / roots[index] - 1.0).abs() > 1e-8 {
            return Err("wrong eigenvalue or spectral index".into());
        }
        let shape: Vec<f64> = mode["shape"]
            .as_array()
            .ok_or("shape missing")?
            .iter()
            .map(|v| {
                v.as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or("nonfinite shape")
            })
            .collect::<std::result::Result<_, _>>()?;
        if shape.len() != 9
            || shape
                .iter()
                .enumerate()
                .any(|(i, &v)| !free.contains(&i) && v != 0.0)
        {
            return Err("invalid shape size or restrained displacement".into());
        }
        let norm = shape.iter().map(|v| v * v).sum::<f64>();
        if (norm - 1.0).abs() > 1e-10 {
            return Err("nonunit physical shape".into());
        }
        let (mut residual, mut applied, mut inertia) = (0.0_f64, 0.0_f64, 0.0_f64);
        for &i in &free {
            let force = k[i].iter().zip(&shape).map(|(a, v)| a * v).sum::<f64>() / mass[i].sqrt();
            let target = lambda * mass[i].sqrt() * shape[i];
            residual = residual.hypot(force - target);
            applied = applied.hypot(force);
            inertia = inertia.hypot(target);
        }
        let relative = residual / applied.max(inertia);
        if !relative.is_finite() || relative > 1e-8 {
            return Err("physical residual rejected".into());
        }
        let weighted: Vec<_> = shape.iter().zip(&mass).map(|(v, m)| v * m.sqrt()).collect();
        let mass_norm = weighted.iter().map(|v| v * v).sum::<f64>().sqrt();
        let normalized: Vec<_> = weighted.iter().map(|v| v / mass_norm).collect();
        for previous in &checked_shapes {
            if previous
                .iter()
                .zip(&normalized)
                .map(|(a, b)| a * b)
                .sum::<f64>()
                .abs()
                > 1e-8
            {
                return Err("duplicate or nonorthogonal mode".into());
            }
        }
        checked_shapes.push(normalized);
        let frequency = number(mode, "natural_frequency_hz")?;
        if frequency <= 0.0
            || (frequency * std::f64::consts::TAU / lambda.sqrt() - 1.0).abs() > 1e-12
            || (frequency * number(mode, "period_s")? - 1.0).abs() > 1e-12
        {
            return Err("inconsistent frequency or period".into());
        }
        residuals.push(relative);
    }
    if (number(output, "total_mass")? - 21.0).abs() > 1e-12 {
        return Err("wrong total mass".into());
    }
    Ok(
        json!({"scope":"synthetic_two_material_four_mode_reference", "passed":true,
        "independent_roots":roots, "physical_residuals":residuals,
        "mass_orthogonality_checked":true, "material_certification":false}),
    )
}

fn number(object: &Value, key: &str) -> Result<f64> {
    object[key]
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("missing finite {key}").into())
}
