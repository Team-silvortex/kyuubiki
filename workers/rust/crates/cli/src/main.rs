use kyuubiki_protocol::{Job, ProgressEvent};
use kyuubiki_solver::MockSolver;

fn main() {
    let config = Config::from_env();
    let job = Job::new(config.job_id, config.project_id, config.case_id);
    let solver = MockSolver::new(config.steps);

    for event in solver.solve(&job) {
        println!("{}", format_event(&event));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Config {
    job_id: String,
    project_id: String,
    case_id: String,
    steps: u64,
}

impl Config {
    fn from_env() -> Self {
        let mut config = Self {
            job_id: "job-local-1".to_string(),
            project_id: "project-local-1".to_string(),
            case_id: "case-local-1".to_string(),
            steps: 5,
        };

        let mut args = std::env::args().skip(1);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--job-id" => {
                    if let Some(value) = args.next() {
                        config.job_id = value;
                    }
                }
                "--project-id" => {
                    if let Some(value) = args.next() {
                        config.project_id = value;
                    }
                }
                "--case-id" => {
                    if let Some(value) = args.next() {
                        config.case_id = value;
                    }
                }
                "--steps" => {
                    if let Some(value) = args.next() {
                        config.steps = value.parse().unwrap_or(config.steps);
                    }
                }
                _ => {}
            }
        }

        config
    }
}

fn format_event(event: &ProgressEvent) -> String {
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

#[cfg(test)]
mod tests {
    use super::{Config, format_event};
    use kyuubiki_protocol::{JobStatus, ProgressEvent};

    #[test]
    fn formats_events_for_machine_consumption() {
        let mut event = ProgressEvent::new("job-1", JobStatus::Solving, 0.5);
        event.iteration = Some(2);
        event.residual = Some(0.125);
        event.peak_memory = Some(576);
        event.message = Some("mock solve step 2/4".to_string());

        assert_eq!(
            format_event(&event),
            "event|job-1|solving|0.50|2|0.125|576|mock solve step 2/4"
        );
    }

    #[test]
    fn preserves_defaults_when_no_args_are_given() {
        let config = Config {
            job_id: "job-local-1".to_string(),
            project_id: "project-local-1".to_string(),
            case_id: "case-local-1".to_string(),
            steps: 5,
        };

        assert_eq!(config.steps, 5);
        assert_eq!(config.job_id, "job-local-1");
    }
}
