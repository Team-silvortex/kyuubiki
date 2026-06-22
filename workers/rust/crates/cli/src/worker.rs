use crate::config::WorkerConfig;
use kyuubiki_protocol::{Job, ProgressEvent};
use kyuubiki_solver::MockSolver;

pub(crate) fn run_worker(config: WorkerConfig) {
    let job = Job::new(config.job_id, config.project_id, config.case_id);
    let solver = MockSolver::new(config.steps);

    for event in solver.solve(&job) {
        println!("{}", format_event(&event));
    }
}

pub(crate) fn format_event(event: &ProgressEvent) -> String {
    format!(
        "event|{}|{}|{:.2}|{}|{}|{}|{}",
        event.job_id,
        event.stage.as_str(),
        event.progress,
        optional_u64(event.iteration),
        optional_f64(event.residual),
        optional_u64(event.peak_memory),
        optional_string(event.message.as_deref())
    )
}

fn optional_u64(value: Option<u64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn optional_f64(value: Option<f64>) -> String {
    value.map(|number| number.to_string()).unwrap_or_default()
}

fn optional_string(value: Option<&str>) -> String {
    value.unwrap_or_default().replace('|', "/")
}
