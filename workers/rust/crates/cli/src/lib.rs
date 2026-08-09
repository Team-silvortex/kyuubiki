// The binary uses the full watchdog surface; library consumers only need its probe.
#[allow(dead_code)]
pub mod agent_watchdog;
mod composite_runtime;
mod composite_runtime_feedback;

pub use composite_runtime::solve_composite_thermo_electric_panel;
