use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};

use kyuubiki_protocol::{Job, ProgressEvent, RpcRequest, RpcResponse};
use kyuubiki_solver::{MockSolver, solve_bar_1d};

fn main() {
    match Command::from_env() {
        Command::Worker(config) => {
            let job = Job::new(config.job_id, config.project_id, config.case_id);
            let solver = MockSolver::new(config.steps);

            for event in solver.solve(&job) {
                println!("{}", format_event(&event));
            }
        }
        Command::Agent(config) => {
            if let Err(error) = run_agent(&config) {
                eprintln!("agent error: {error}");
                std::process::exit(1);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkerConfig {
    job_id: String,
    project_id: String,
    case_id: String,
    steps: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentConfig {
    host: String,
    port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Command {
    Worker(WorkerConfig),
    Agent(AgentConfig),
}

impl Command {
    fn from_env() -> Self {
        let args = std::env::args().skip(1).collect::<Vec<_>>();

        match args.first().map(String::as_str) {
            Some("agent") => Self::Agent(AgentConfig::from_args(&args[1..])),
            _ => Self::Worker(WorkerConfig::from_args(&args)),
        }
    }
}

impl WorkerConfig {
    fn from_args(args: &[String]) -> Self {
        let mut config = Self {
            job_id: "job-local-1".to_string(),
            project_id: "project-local-1".to_string(),
            case_id: "case-local-1".to_string(),
            steps: 5,
        };

        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--job-id" => {
                    if let Some(value) = args.next() {
                        config.job_id = value.clone();
                    }
                }
                "--project-id" => {
                    if let Some(value) = args.next() {
                        config.project_id = value.clone();
                    }
                }
                "--case-id" => {
                    if let Some(value) = args.next() {
                        config.case_id = value.clone();
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

impl AgentConfig {
    fn from_args(args: &[String]) -> Self {
        let mut config = Self {
            host: "127.0.0.1".to_string(),
            port: 5001,
        };

        let mut args = args.iter();

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--host" => {
                    if let Some(value) = args.next() {
                        config.host = value.clone();
                    }
                }
                "--port" => {
                    if let Some(value) = args.next() {
                        config.port = value.parse().unwrap_or(config.port);
                    }
                }
                _ => {}
            }
        }

        config
    }
}

fn run_agent(config: &AgentConfig) -> Result<(), String> {
    let listener = TcpListener::bind((config.host.as_str(), config.port))
        .map_err(|error| format!("failed to bind {}:{}: {error}", config.host, config.port))?;

    for stream in listener.incoming() {
        let stream = stream.map_err(|error| format!("failed to accept connection: {error}"))?;
        handle_connection(stream)?;
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> Result<(), String> {
    let reader_stream = stream
        .try_clone()
        .map_err(|error| format!("failed to clone stream: {error}"))?;
    let mut reader = BufReader::new(reader_stream);
    let mut line = String::new();

    loop {
        line.clear();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|error| format!("failed to read request: {error}"))?;

        if bytes == 0 {
            break;
        }

        let response = handle_request_line(line.trim_end());
        let encoded = serde_json::to_string(&response)
            .map_err(|error| format!("failed to serialize response: {error}"))?;
        stream
            .write_all(encoded.as_bytes())
            .and_then(|_| stream.write_all(b"\n"))
            .map_err(|error| format!("failed to write response: {error}"))?;
    }

    Ok(())
}

fn handle_request_line(line: &str) -> RpcResponse {
    let request = match serde_json::from_str::<RpcRequest>(line) {
        Ok(request) => request,
        Err(error) => return RpcResponse::error("invalid_json", error.to_string()),
    };

    if request.method != "solve_bar_1d" {
        return RpcResponse::error(
            "invalid_method",
            format!("unsupported method: {}", request.method),
        );
    }

    match solve_bar_1d(&request.params) {
        Ok(result) => RpcResponse::success(result),
        Err(error) => RpcResponse::error("solve_failed", error),
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
    use super::{AgentConfig, Command, WorkerConfig, format_event, handle_request_line};
    use kyuubiki_protocol::{JobStatus, ProgressEvent, RpcRequest, SolveBarRequest};

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
    fn preserves_worker_defaults_when_no_args_are_given() {
        let config = WorkerConfig {
            job_id: "job-local-1".to_string(),
            project_id: "project-local-1".to_string(),
            case_id: "case-local-1".to_string(),
            steps: 5,
        };

        assert_eq!(config.steps, 5);
        assert_eq!(config.job_id, "job-local-1");
    }

    #[test]
    fn parses_agent_command_defaults() {
        let command = Command::Agent(AgentConfig::from_args(&[]));

        assert_eq!(
            command,
            Command::Agent(AgentConfig {
                host: "127.0.0.1".to_string(),
                port: 5001,
            })
        );
    }

    #[test]
    fn handles_solver_rpc_requests() {
        let request = RpcRequest {
            method: "solve_bar_1d".to_string(),
            params: SolveBarRequest {
                length: 1.0,
                area: 0.01,
                youngs_modulus: 210.0e9,
                elements: 1,
                tip_force: 1000.0,
            },
        };

        let response = handle_request_line(
            &serde_json::to_string(&request).expect("request should serialize"),
        );

        assert!(response.ok);
        assert!(response.error.is_none());
        assert!(
            (response.result.expect("solver result").tip_displacement - 4.761904761904762e-7).abs()
                < 1.0e-12
        );
    }
}
