use std::time::{Instant, SystemTime, UNIX_EPOCH};

use kyuubiki_engine::{EngineSolveRequest, solve};
use kyuubiki_headless_sdk::{action_capability_manifest, direct_fem_capability_manifest};
use kyuubiki_protocol::AnalysisResult;
use kyuubiki_solver::profile_heat_plane_quad_2d;

use crate::models::{
    BenchmarkCase, BenchmarkMemoryStage, BenchmarkReport, BenchmarkResult, BenchmarkWorkload,
};

pub(crate) fn build_report(
    selected: &[&BenchmarkCase],
    repeat: usize,
    profile: crate::config::BenchmarkProfile,
    matrix: &str,
) -> BenchmarkReport {
    let cases = selected
        .iter()
        .map(|case| run_case(case, repeat))
        .collect::<Vec<_>>();

    BenchmarkReport {
        repeat,
        profile,
        matrix: matrix.to_string(),
        generated_at_unix_s: unix_timestamp(),
        cases,
    }
}

pub(crate) fn run_case(case: &BenchmarkCase, repeat: usize) -> BenchmarkResult {
    let mut durations = Vec::with_capacity(repeat);
    let (mut node_count, mut element_count, mut dof_count) = workload_shape(&case.workload);
    let mut max_displacement = 0.0;
    let mut max_stress = 0.0;
    let mut peak_rss_kib = current_peak_rss_kib();
    let mut memory_stages = Vec::new();
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
            BenchmarkWorkload::PlaneQuad2d(request) => {
                solve(EngineSolveRequest::PlaneQuad2d(request.clone())).map(|result| {
                    let AnalysisResult::PlaneQuad2d(result) = result else {
                        unreachable!("quad plane solve should return quad plane result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len() * 2;
                    max_displacement = result.max_displacement;
                    max_stress = result.max_stress;
                })
            }
            BenchmarkWorkload::HeatPlaneQuad2d(request) => {
                profile_heat_plane_quad_2d(request).map(|profile| {
                    let result = profile.result;
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len();
                    max_displacement = result.max_temperature;
                    max_stress = result.max_heat_flux;
                    memory_stages = profile
                        .memory_stages
                        .into_iter()
                        .map(|stage| BenchmarkMemoryStage {
                            label: stage.label.to_string(),
                            rss_kib: stage.rss_kib,
                        })
                        .collect();
                })
            }
            BenchmarkWorkload::Truss3d(request) => {
                solve(EngineSolveRequest::Truss3d(request.clone())).map(|result| {
                    let AnalysisResult::Truss3d(result) = result else {
                        unreachable!("3d truss solve should return 3d truss result")
                    };
                    node_count = result.nodes.len();
                    element_count = result.elements.len();
                    dof_count = result.nodes.len() * 3;
                    max_displacement = result.max_displacement;
                    max_stress = result.max_stress;
                })
            }
            BenchmarkWorkload::HeadlessActionManifest => {
                let manifest = action_capability_manifest();
                let payload = serde_json::to_vec(&manifest)
                    .map_err(|error| format!("manifest serialization failed: {error}"));
                payload.map(|payload| {
                    node_count = manifest.len();
                    element_count = manifest
                        .iter()
                        .filter(|entry| entry.direct_fem_route.is_some())
                        .count();
                    dof_count = manifest
                        .iter()
                        .map(|entry| entry.required_payload_keys.len() + entry.output_keys.len())
                        .sum();
                    max_displacement = element_count as f64;
                    max_stress = payload.len() as f64;
                })
            }
            BenchmarkWorkload::DirectFemManifest => {
                let manifest = direct_fem_capability_manifest();
                let payload = serde_json::to_vec(&manifest)
                    .map_err(|error| format!("direct FEM manifest serialization failed: {error}"));
                payload.map(|payload| {
                    node_count = manifest.len();
                    element_count = manifest.len();
                    dof_count = manifest
                        .iter()
                        .map(|entry| entry.required_payload_keys.len() + entry.output_keys.len())
                        .sum();
                    max_displacement = manifest.len() as f64;
                    max_stress = payload.len() as f64;
                })
            }
        };

        durations.push(started.elapsed().as_secs_f64() * 1000.0);
        peak_rss_kib = peak_rss_kib.max(current_peak_rss_kib());

        if let Err(message) = outcome {
            error = Some(message);
            break;
        }
    }

    let ok = error.is_none();
    let mut sorted = durations.clone();
    sorted.sort_by(|lhs, rhs| lhs.total_cmp(rhs));
    let min_ms = sorted.iter().copied().fold(f64::INFINITY, f64::min);
    let max_ms = sorted.iter().copied().fold(0.0, f64::max);
    let mean_ms = if durations.is_empty() {
        0.0
    } else {
        durations.iter().copied().sum::<f64>() / durations.len() as f64
    };

    BenchmarkResult {
        id: case.id.clone(),
        family: case.family.to_string(),
        ok,
        error,
        repeat,
        min_ms: if min_ms.is_finite() { min_ms } else { 0.0 },
        median_ms: percentile(&sorted, 0.5),
        mean_ms,
        p95_ms: percentile(&sorted, 0.95),
        max_ms,
        dof_count,
        node_count,
        element_count,
        peak_rss_kib,
        memory_stages,
        max_displacement,
        max_stress,
    }
}

fn workload_shape(workload: &BenchmarkWorkload) -> (usize, usize, usize) {
    match workload {
        BenchmarkWorkload::AxialBar(request) => {
            (request.elements + 1, request.elements, request.elements)
        }
        BenchmarkWorkload::Truss2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::Truss3d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 3,
        ),
        BenchmarkWorkload::PlaneTriangle2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::PlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len() * 2,
        ),
        BenchmarkWorkload::HeatPlaneQuad2d(request) => (
            request.nodes.len(),
            request.elements.len(),
            request.nodes.len(),
        ),
        BenchmarkWorkload::HeadlessActionManifest => (0, 0, 0),
        BenchmarkWorkload::DirectFemManifest => (0, 0, 0),
    }
}

fn current_peak_rss_kib() -> u64 {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if status == 0 {
            let usage = unsafe { usage.assume_init() };
            #[cfg(target_os = "macos")]
            {
                return (usage.ru_maxrss as u64) / 1024;
            }
            #[cfg(target_os = "linux")]
            {
                return usage.ru_maxrss as u64;
            }
        }
    }

    0
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }

    let index = ((sorted.len() - 1) as f64 * fraction).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

fn unix_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
