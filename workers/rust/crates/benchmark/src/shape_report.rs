use serde::Serialize;

use crate::config::BenchmarkProfile;
use crate::models::BenchmarkCase;
use crate::runner_shape::workload_shape;
use crate::runner_util::unix_timestamp;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BenchmarkShapeReport {
    pub(crate) profile: BenchmarkProfile,
    pub(crate) matrix: String,
    pub(crate) generated_at_unix_s: u64,
    pub(crate) cases: Vec<BenchmarkShapeCase>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct BenchmarkShapeCase {
    pub(crate) id: String,
    pub(crate) family: String,
    pub(crate) node_count: usize,
    pub(crate) element_count: usize,
    pub(crate) dof_count: usize,
}

pub(crate) fn build_shape_report(
    selected: &[&BenchmarkCase],
    profile: BenchmarkProfile,
    matrix: &str,
) -> BenchmarkShapeReport {
    let cases = selected
        .iter()
        .map(|case| {
            let (node_count, element_count, dof_count) = workload_shape(&case.workload);
            BenchmarkShapeCase {
                id: case.id.clone(),
                family: case.family.to_string(),
                node_count,
                element_count,
                dof_count,
            }
        })
        .collect();

    BenchmarkShapeReport {
        profile,
        matrix: matrix.to_string(),
        generated_at_unix_s: unix_timestamp(),
        cases,
    }
}

pub(crate) fn print_shape_table(report: &BenchmarkShapeReport) {
    println!(
        "benchmark shape report: profile={} matrix={}",
        report.profile.as_str(),
        report.matrix
    );
    println!(
        "{:<34} {:<34} {:>12} {:>12} {:>12}",
        "case", "family", "nodes", "elements", "dofs"
    );
    for case in &report.cases {
        println!(
            "{:<34} {:<34} {:>12} {:>12} {:>12}",
            case.id, case.family, case.node_count, case.element_count, case.dof_count
        );
    }
}
