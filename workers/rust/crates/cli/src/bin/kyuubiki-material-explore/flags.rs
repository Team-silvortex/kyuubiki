#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Flags {
    pub(crate) study: String,
    pub(crate) out: Option<String>,
    pub(crate) json: bool,
    pub(crate) plan_next: Option<String>,
    pub(crate) run_next: Option<String>,
    pub(crate) run_materialized: Option<String>,
    pub(crate) chain_next: Option<String>,
    pub(crate) rounds: usize,
}

impl Flags {
    pub(crate) fn parse(args: Vec<String>) -> Result<Self, String> {
        if args.is_empty() || args.iter().any(|arg| arg == "--help" || arg == "help") {
            return Err(usage());
        }
        let mut study = args[0].clone();
        let mut out = None;
        let mut json = false;
        let mut plan_next = None;
        let mut run_next = None;
        let mut run_materialized = None;
        let mut chain_next = None;
        let mut rounds = 2;
        let mut index = 1;
        if matches!(
            study.as_str(),
            "--plan-next" | "--run-next" | "--run-materialized" | "--chain-next"
        ) {
            let option = study.clone();
            let Some(path) = args.get(1) else {
                return Err(format!("{option} requires a value"));
            };
            if path.starts_with("--") {
                return Err(format!("{option} requires a value"));
            }
            if option == "--plan-next" {
                plan_next = Some(path.clone());
            } else if option == "--run-next" {
                run_next = Some(path.clone());
            } else if option == "--run-materialized" {
                run_materialized = Some(path.clone());
            } else {
                chain_next = Some(path.clone());
            }
            study = "from-previous-exploration".to_string();
            index = 2;
        }
        while index < args.len() {
            match args[index].as_str() {
                "--json" => json = true,
                "--out" => out = Some(take_value(&args, &mut index, "--out")?),
                "--plan-next" => plan_next = Some(take_value(&args, &mut index, "--plan-next")?),
                "--run-next" => run_next = Some(take_value(&args, &mut index, "--run-next")?),
                "--run-materialized" => {
                    run_materialized = Some(take_value(&args, &mut index, "--run-materialized")?)
                }
                "--chain-next" => chain_next = Some(take_value(&args, &mut index, "--chain-next")?),
                "--rounds" => rounds = parse_rounds(take_value(&args, &mut index, "--rounds")?)?,
                other => return Err(format!("unsupported flag: {other}\n\n{}", usage())),
            }
            index += 1;
        }
        Ok(Self {
            study,
            out,
            json,
            plan_next,
            run_next,
            run_materialized,
            chain_next,
            rounds,
        })
    }
}

fn take_value(args: &[String], index: &mut usize, option: &str) -> Result<String, String> {
    *index += 1;
    let Some(value) = args.get(*index) else {
        return Err(format!("{option} requires a value"));
    };
    if value.starts_with("--") {
        return Err(format!("{option} requires a value"));
    }
    Ok(value.clone())
}

fn parse_rounds(value: String) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| "--rounds must be a positive integer".to_string())
        .and_then(|rounds| {
            if rounds == 0 {
                Err("--rounds must be at least 1".to_string())
            } else {
                Ok(rounds)
            }
        })
}

fn usage() -> String {
    "kyuubiki-material-explore <heat-spreader|dielectric-screening|thermo-shield|structural-panel|composite-thermo-electric-panel> [--out exploration.json] [--json]\nkyuubiki-material-explore --plan-next previous-exploration.json [--out next-round.json] [--json]\nkyuubiki-material-explore --run-next previous-exploration.json [--out next-exploration.json] [--json]\nkyuubiki-material-explore --run-materialized materialization-plan.json [--out rerun.json] [--json]\nkyuubiki-material-explore --chain-next previous-exploration.json [--rounds 2] [--out chain.json] [--json]\n\nRuns candidate material studies locally through real solver kernels and builds a ranked material report.".to_string()
}
