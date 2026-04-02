use std::time::Instant;

use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_protocol::{
    AnalysisResult,
    PlaneNodeInput, PlaneTriangleElementInput, SolveBarRequest, SolvePlaneTriangle2dRequest,
    SolveTruss2dRequest, TrussElementInput, TrussNodeInput,
};
use serde::Serialize;

fn main() {
    let config = BenchmarkConfig::from_env();
    let cases = benchmark_cases(config.profile);
    let selected = select_cases(&cases, config.case_filter.as_deref());

    let results = selected
        .iter()
        .map(|case| run_case(case, config.repeat))
        .collect::<Vec<_>>();

    match config.format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(&BenchmarkReport {
                    repeat: config.repeat,
                    profile: config.profile,
                    cases: results,
                })
                .expect("report should serialize")
            );
        }
        OutputFormat::Table => print_table(&results, config.repeat, config.profile),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchmarkConfig {
    repeat: usize,
    case_filter: Option<String>,
    format: OutputFormat,
    profile: BenchmarkProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum OutputFormat {
    Table,
    Json,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum BenchmarkProfile {
    Medium,
    Large,
    V2,
}

impl BenchmarkProfile {
    fn as_str(self) -> &'static str {
        match self {
            Self::Medium => "medium",
            Self::Large => "large",
            Self::V2 => "v2",
        }
    }
}

impl BenchmarkConfig {
    fn from_env() -> Self {
        let mut config = Self {
            repeat: 10,
            case_filter: None,
            format: OutputFormat::Table,
            profile: BenchmarkProfile::Medium,
        };

        let args = std::env::args().skip(1).collect::<Vec<_>>();
        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--repeat" => {
                    if let Some(value) = args.next() {
                        config.repeat = value.parse().unwrap_or(config.repeat).max(1);
                    }
                }
                "--case" => {
                    if let Some(value) = args.next() {
                        config.case_filter = Some(value.clone());
                    }
                }
                "--format" => {
                    if let Some(value) = args.next() {
                        config.format = match value.as_str() {
                            "json" => OutputFormat::Json,
                            _ => OutputFormat::Table,
                        };
                    }
                }
                "--profile" => {
                    if let Some(value) = args.next() {
                        config.profile = match value.as_str() {
                            "large" => BenchmarkProfile::Large,
                            "v2" => BenchmarkProfile::V2,
                            _ => BenchmarkProfile::Medium,
                        };
                    }
                }
                _ => {}
            }
        }

        config
    }
}

#[derive(Debug, Clone)]
struct BenchmarkCase {
    id: &'static str,
    family: &'static str,
    workload: BenchmarkWorkload,
}

#[derive(Debug, Clone)]
enum BenchmarkWorkload {
    AxialBar(SolveBarRequest),
    Truss2d(SolveTruss2dRequest),
    PlaneTriangle2d(SolvePlaneTriangle2dRequest),
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkReport {
    repeat: usize,
    profile: BenchmarkProfile,
    cases: Vec<BenchmarkResult>,
}

#[derive(Debug, Clone, Serialize)]
struct BenchmarkResult {
    id: String,
    family: String,
    ok: bool,
    error: Option<String>,
    repeat: usize,
    min_ms: f64,
    mean_ms: f64,
    max_ms: f64,
    dof_count: usize,
    node_count: usize,
    element_count: usize,
    max_displacement: f64,
    max_stress: f64,
}

fn benchmark_cases(profile: BenchmarkProfile) -> Vec<BenchmarkCase> {
    match profile {
        BenchmarkProfile::Medium => vec![
            BenchmarkCase {
                id: "axial-bar-medium",
                family: "axial_bar_1d",
                workload: BenchmarkWorkload::AxialBar(generate_bar_case(120)),
            },
            BenchmarkCase {
                id: "truss-roof-medium",
                family: "truss_2d",
                workload: BenchmarkWorkload::Truss2d(generate_pratt_truss(12, 24.0, 5.0)),
            },
            BenchmarkCase {
                id: "plane-panel-medium",
                family: "plane_triangle_2d",
                workload: BenchmarkWorkload::PlaneTriangle2d(generate_panel_mesh(6, 4, 6.0, 4.0)),
            },
        ],
        BenchmarkProfile::Large => vec![
            BenchmarkCase {
                id: "axial-bar-large",
                family: "axial_bar_1d",
                workload: BenchmarkWorkload::AxialBar(generate_bar_case(2500)),
            },
            BenchmarkCase {
                id: "truss-roof-large",
                family: "truss_2d",
                workload: BenchmarkWorkload::Truss2d(generate_pratt_truss(127, 64.0, 12.0)),
            },
            BenchmarkCase {
                id: "plane-panel-large",
                family: "plane_triangle_2d",
                workload: BenchmarkWorkload::PlaneTriangle2d(generate_panel_mesh(21, 21, 21.0, 21.0)),
            },
        ],
        BenchmarkProfile::V2 => vec![
            BenchmarkCase {
                id: "axial-bar-v2",
                family: "axial_bar_1d",
                workload: BenchmarkWorkload::AxialBar(generate_bar_case(5000)),
            },
            BenchmarkCase {
                id: "truss-roof-v2",
                family: "truss_2d",
                workload: BenchmarkWorkload::Truss2d(generate_pratt_truss(2500, 1250.0, 80.0)),
            },
            BenchmarkCase {
                id: "plane-panel-v2",
                family: "plane_triangle_2d",
                workload: BenchmarkWorkload::PlaneTriangle2d(generate_panel_mesh(70, 70, 70.0, 70.0)),
            },
        ],
    }
}

fn select_cases<'a>(cases: &'a [BenchmarkCase], filter: Option<&str>) -> Vec<&'a BenchmarkCase> {
    match filter {
        Some(filter) => cases
            .iter()
            .filter(|case| case.id.contains(filter))
            .collect(),
        None => cases.iter().collect(),
    }
}

fn run_case(case: &BenchmarkCase, repeat: usize) -> BenchmarkResult {
    let mut durations = Vec::with_capacity(repeat);
    let mut node_count = 0;
    let mut element_count = 0;
    let mut dof_count = 0;
    let mut max_displacement = 0.0;
    let mut max_stress = 0.0;
    let mut error = None;

    for _ in 0..repeat {
        let started = Instant::now();

        let outcome = match &case.workload {
            BenchmarkWorkload::AxialBar(request) => {
                solve(EngineSolveRequest::Bar1d(request.clone())).map(|result| {
                    let AnalysisResult::Bar1d(result) = result else {
                        unreachable!("bar solve should return bar result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len().saturating_sub(1);
                    max_displacement = result.max_displacement;
                    max_stress = result.max_stress;
                })
            }
            BenchmarkWorkload::Truss2d(request) => {
                solve(EngineSolveRequest::Truss2d(request.clone())).map(|result| {
                    let AnalysisResult::Truss2d(result) = result else {
                        unreachable!("truss solve should return truss result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len() * 2;
                    max_displacement = result.max_displacement;
                    max_stress = result.max_stress;
                })
            }
            BenchmarkWorkload::PlaneTriangle2d(request) => {
                solve(EngineSolveRequest::PlaneTriangle2d(request.clone())).map(|result| {
                    let AnalysisResult::PlaneTriangle2d(result) = result else {
                        unreachable!("plane solve should return plane result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len() * 2;
                    max_displacement = result.max_displacement;
                    max_stress = result.max_stress;
                })
            }
        };

        durations.push(started.elapsed().as_secs_f64() * 1000.0);

        if let Err(message) = outcome {
            error = Some(message);
            break;
        }
    }

    let ok = error.is_none();
    let min_ms = durations.iter().copied().fold(f64::INFINITY, f64::min);
    let max_ms = durations.iter().copied().fold(0.0, f64::max);
    let mean_ms = if durations.is_empty() {
        0.0
    } else {
        durations.iter().copied().sum::<f64>() / durations.len() as f64
    };

    BenchmarkResult {
        id: case.id.to_string(),
        family: case.family.to_string(),
        ok,
        error,
        repeat,
        min_ms: if min_ms.is_finite() { min_ms } else { 0.0 },
        mean_ms,
        max_ms,
        dof_count,
        node_count,
        element_count,
        max_displacement,
        max_stress,
    }
}

fn print_table(results: &[BenchmarkResult], repeat: usize, profile: BenchmarkProfile) {
    println!("kyuubiki benchmark suite");
    println!("profile: {}", profile.as_str());
    println!("repeat count: {repeat}");
    println!();
    println!(
        "{:<22} {:<20} {:<6} {:>6} {:>6} {:>7} {:>10} {:>10} {:>10}",
        "case", "family", "status", "nodes", "elems", "dofs", "min ms", "mean ms", "max ms"
    );

    for result in results {
        println!(
            "{:<22} {:<20} {:<6} {:>6} {:>6} {:>7} {:>10.4} {:>10.4} {:>10.4}",
            result.id,
            result.family,
            if result.ok { "ok" } else { "fail" },
            result.node_count,
            result.element_count,
            result.dof_count,
            result.min_ms,
            result.mean_ms,
            result.max_ms
        );
        if let Some(error) = &result.error {
            println!("  error: {error}");
        }
    }
}

fn generate_bar_case(elements: usize) -> SolveBarRequest {
    SolveBarRequest {
        length: 20.0,
        area: 0.01,
        youngs_modulus: 70.0e9,
        elements,
        tip_force: 1800.0,
    }
}

fn generate_pratt_truss(bays: usize, span: f64, height: f64) -> SolveTruss2dRequest {
    let bay_width = span / bays as f64;
    let mut nodes = Vec::new();
    let mut elements = Vec::new();

    for index in 0..=bays {
        nodes.push(TrussNodeInput {
            id: format!("b{index}"),
            x: index as f64 * bay_width,
            y: 0.0,
            fix_x: index == 0,
            fix_y: index == 0 || index == bays,
            load_x: 0.0,
            load_y: 0.0,
        });
    }

    for index in 0..bays {
        nodes.push(TrussNodeInput {
            id: format!("t{index}"),
            x: index as f64 * bay_width + bay_width * 0.5,
            y: height,
            fix_x: false,
            fix_y: false,
            load_x: 0.0,
            load_y: if index == bays / 2 { -40.0 } else { 0.0 },
        });
    }

    let top_offset = bays + 1;
    for index in 0..bays {
        elements.push(TrussElementInput {
            id: format!("bb{index}"),
            node_i: index,
            node_j: index + 1,
            area: 0.03,
            youngs_modulus: 210.0e9,
        });

        elements.push(TrussElementInput {
            id: format!("s{index}_l"),
            node_i: index,
            node_j: top_offset + index,
            area: 0.03,
            youngs_modulus: 210.0e9,
        });

        elements.push(TrussElementInput {
            id: format!("s{index}_r"),
            node_i: top_offset + index,
            node_j: index + 1,
            area: 0.03,
            youngs_modulus: 210.0e9,
        });

        if index + 1 < bays {
            elements.push(TrussElementInput {
                id: format!("tt{index}"),
                node_i: top_offset + index,
                node_j: top_offset + index + 1,
                area: 0.03,
                youngs_modulus: 210.0e9,
            });
        }
    }

    SolveTruss2dRequest { nodes, elements }
}

fn generate_panel_mesh(
    nx: usize,
    ny: usize,
    width: f64,
    height: f64,
) -> SolvePlaneTriangle2dRequest {
    let dx = width / nx as f64;
    let dy = height / ny as f64;
    let mut nodes = Vec::new();
    let mut elements = Vec::new();

    for j in 0..=ny {
        for i in 0..=nx {
            let index = j * (nx + 1) + i;
            nodes.push(PlaneNodeInput {
                id: format!("n{index}"),
                x: i as f64 * dx,
                y: j as f64 * dy,
                fix_x: i == 0,
                fix_y: i == 0 || (j == 0 && i == 0),
                load_x: if i == nx { 15.0 } else { 0.0 },
                load_y: if i == nx { -40.0 } else { 0.0 },
            });
        }
    }

    for j in 0..ny {
        for i in 0..nx {
            let n0 = j * (nx + 1) + i;
            let n1 = n0 + 1;
            let n2 = n0 + (nx + 1);
            let n3 = n2 + 1;

            elements.push(PlaneTriangleElementInput {
                id: format!("p{}_a", j * nx + i),
                node_i: n0,
                node_j: n1,
                node_k: n3,
                thickness: 0.015,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            });
            elements.push(PlaneTriangleElementInput {
                id: format!("p{}_b", j * nx + i),
                node_i: n0,
                node_j: n3,
                node_k: n2,
                thickness: 0.015,
                youngs_modulus: 70.0e9,
                poisson_ratio: 0.33,
            });
        }
    }

    SolvePlaneTriangle2dRequest { nodes, elements }
}

#[cfg(test)]
mod tests {
    use super::{BenchmarkConfig, BenchmarkProfile, OutputFormat, benchmark_cases, run_case};

    #[test]
    fn exposes_default_benchmark_config() {
        let config = BenchmarkConfig {
            repeat: 10,
            case_filter: None,
            format: OutputFormat::Table,
            profile: BenchmarkProfile::Medium,
        };

        assert_eq!(config.repeat, 10);
        assert!(matches!(config.format, OutputFormat::Table));
    }

    #[test]
    fn runs_benchmark_cases() {
        let cases = benchmark_cases(BenchmarkProfile::Medium);
        let result = run_case(&cases[0], 2);

        assert_eq!(result.repeat, 2);
        assert!(result.mean_ms >= 0.0);
        assert!(result.node_count > 0);
    }
}
