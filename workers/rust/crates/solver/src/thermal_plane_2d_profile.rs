use kyuubiki_protocol::{SolveThermalPlaneQuad2dResult, SolveThermalPlaneTriangle2dResult};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ThermalPlaneProfileStage {
    pub label: &'static str,
    pub rss_kib: u64,
    pub elapsed_ms: f64,
}

#[derive(Debug, Clone)]
pub struct ThermalPlaneTriangleProfile {
    pub result: SolveThermalPlaneTriangle2dResult,
    pub stages: Vec<ThermalPlaneProfileStage>,
    pub solver_iterations: usize,
    pub solver_residual_norm: f64,
}

#[derive(Debug, Clone)]
pub struct ThermalPlaneQuadProfile {
    pub result: SolveThermalPlaneQuad2dResult,
    pub stages: Vec<ThermalPlaneProfileStage>,
    pub solver_iterations: usize,
    pub solver_residual_norm: f64,
}

pub(crate) fn push_thermal_plane_stage(
    stages: &mut Vec<ThermalPlaneProfileStage>,
    enabled: bool,
    label: &'static str,
    elapsed: Duration,
) {
    if !enabled {
        return;
    }

    stages.push(ThermalPlaneProfileStage {
        label,
        rss_kib: current_rss_kib(),
        elapsed_ms: elapsed.as_secs_f64() * 1000.0,
    });
}

fn current_rss_kib() -> u64 {
    #[cfg(target_os = "linux")]
    {
        if let Ok(statm) = std::fs::read_to_string("/proc/self/statm") {
            if let Some(resident_pages) = statm.split_whitespace().nth(1) {
                if let Ok(resident_pages) = resident_pages.parse::<u64>() {
                    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
                    if page_size > 0 {
                        return resident_pages * page_size as u64 / 1024;
                    }
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
        let status = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
        if status == 0 {
            let usage = unsafe { usage.assume_init() };
            return (usage.ru_maxrss as u64) / 1024;
        }
    }

    0
}
