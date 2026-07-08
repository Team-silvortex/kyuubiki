#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Flags {
    pub(crate) study: String,
    pub(crate) out: Option<String>,
    pub(crate) json: bool,
    pub(crate) catalog: bool,
    pub(crate) describe_study: Option<String>,
    pub(crate) plan_study: Option<String>,
    pub(crate) plan_next: Option<String>,
    pub(crate) run_next: Option<String>,
    pub(crate) review_template: Option<String>,
    pub(crate) approve_review_template: Option<String>,
    pub(crate) materialize_reviewed: Option<String>,
    pub(crate) review_decision: Option<String>,
    pub(crate) reviewer_id: Option<String>,
    pub(crate) reviewer_name: Option<String>,
    pub(crate) reason: Option<String>,
    pub(crate) decided_at: Option<String>,
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
        let mut catalog = false;
        let mut describe_study = None;
        let mut plan_study = None;
        let mut plan_next = None;
        let mut run_next = None;
        let mut review_template = None;
        let mut approve_review_template = None;
        let mut materialize_reviewed = None;
        let mut review_decision = None;
        let mut reviewer_id = None;
        let mut reviewer_name = None;
        let mut reason = None;
        let mut decided_at = None;
        let mut run_materialized = None;
        let mut chain_next = None;
        let mut rounds = 2;
        let mut index = 1;
        if matches!(
            study.as_str(),
            "--plan-next"
                | "--run-next"
                | "--review-template"
                | "--approve-review-template"
                | "--materialize-reviewed"
                | "--run-materialized"
                | "--chain-next"
                | "--describe-study"
                | "--plan-study"
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
            } else if option == "--review-template" {
                review_template = Some(path.clone());
            } else if option == "--approve-review-template" {
                approve_review_template = Some(path.clone());
            } else if option == "--materialize-reviewed" {
                materialize_reviewed = Some(path.clone());
            } else if option == "--run-materialized" {
                run_materialized = Some(path.clone());
            } else if option == "--describe-study" {
                describe_study = Some(path.clone());
            } else if option == "--plan-study" {
                plan_study = Some(path.clone());
            } else {
                chain_next = Some(path.clone());
            }
            study = "from-previous-exploration".to_string();
            index = 2;
        } else if study == "--catalog" {
            catalog = true;
            study = "catalog".to_string();
            index = 1;
        }
        while index < args.len() {
            match args[index].as_str() {
                "--json" => json = true,
                "--out" => out = Some(take_value(&args, &mut index, "--out")?),
                "--catalog" => catalog = true,
                "--describe-study" => {
                    describe_study = Some(take_value(&args, &mut index, "--describe-study")?)
                }
                "--plan-study" => plan_study = Some(take_value(&args, &mut index, "--plan-study")?),
                "--plan-next" => plan_next = Some(take_value(&args, &mut index, "--plan-next")?),
                "--run-next" => run_next = Some(take_value(&args, &mut index, "--run-next")?),
                "--review-template" => {
                    review_template = Some(take_value(&args, &mut index, "--review-template")?)
                }
                "--approve-review-template" => {
                    approve_review_template =
                        Some(take_value(&args, &mut index, "--approve-review-template")?)
                }
                "--materialize-reviewed" => {
                    materialize_reviewed =
                        Some(take_value(&args, &mut index, "--materialize-reviewed")?)
                }
                "--review-decision" => {
                    review_decision = Some(take_value(&args, &mut index, "--review-decision")?)
                }
                "--reviewer-id" => {
                    reviewer_id = Some(take_value(&args, &mut index, "--reviewer-id")?)
                }
                "--reviewer-name" => {
                    reviewer_name = Some(take_value(&args, &mut index, "--reviewer-name")?)
                }
                "--reason" => reason = Some(take_value(&args, &mut index, "--reason")?),
                "--decided-at" => decided_at = Some(take_value(&args, &mut index, "--decided-at")?),
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
            catalog,
            describe_study,
            plan_study,
            plan_next,
            run_next,
            review_template,
            approve_review_template,
            materialize_reviewed,
            review_decision,
            reviewer_id,
            reviewer_name,
            reason,
            decided_at,
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
    "kyuubiki-material-explore <heat-spreader|dielectric-screening|thermo-shield|structural-panel|composite-thermo-electric-panel> [--out exploration.json] [--json]\nkyuubiki-material-explore --catalog [--out catalog.json] [--json]\nkyuubiki-material-explore --describe-study <study> [--out study.json] [--json]\nkyuubiki-material-explore --plan-study <study> [--out plan.json] [--json]\nkyuubiki-material-explore --plan-next previous-exploration.json [--out next-round.json] [--json]\nkyuubiki-material-explore --review-template next-round.json [--out decision-template.json] [--json]\nkyuubiki-material-explore --approve-review-template decision-template.json --reviewer-id id --reason text --decided-at timestamp [--out decision.json] [--json]\nkyuubiki-material-explore --materialize-reviewed next-round.json --review-decision decision.json [--out materialization-plan.json] [--json]\nkyuubiki-material-explore --run-next previous-exploration.json [--out next-exploration.json] [--json]\nkyuubiki-material-explore --run-materialized materialization-plan.json [--out rerun.json] [--json]\nkyuubiki-material-explore --chain-next previous-exploration.json [--rounds 2] [--out chain.json] [--json]\n\nRuns candidate material studies locally through real solver kernels and builds a ranked material report.".to_string()
}

#[cfg(test)]
mod tests {
    use super::Flags;

    #[test]
    fn parses_review_template_short_form() {
        let flags = parse(["--review-template", "next-round.json", "--json"]);

        assert_eq!(flags.study, "from-previous-exploration");
        assert_eq!(flags.review_template.as_deref(), Some("next-round.json"));
        assert!(flags.json);
    }

    #[test]
    fn parses_catalog_and_describe_study() {
        let catalog = parse(["--catalog", "--json"]);
        let describe = parse(["--describe-study", "heat-spreader", "--out", "study.json"]);
        let plan = parse(["--plan-study", "composite-thermo-electric-panel"]);

        assert!(catalog.catalog);
        assert!(catalog.json);
        assert_eq!(describe.study, "from-previous-exploration");
        assert_eq!(describe.describe_study.as_deref(), Some("heat-spreader"));
        assert_eq!(describe.out.as_deref(), Some("study.json"));
        assert_eq!(
            plan.plan_study.as_deref(),
            Some("composite-thermo-electric-panel")
        );
    }

    #[test]
    fn parses_approve_review_template_arguments() {
        let flags = parse([
            "--approve-review-template",
            "decision-template.json",
            "--reviewer-id",
            "reviewer-1",
            "--reviewer-name",
            "Reviewer One",
            "--reason",
            "prototype approved",
            "--decided-at",
            "2026-07-07T00:00:00Z",
            "--out",
            "decision.json",
        ]);

        assert_eq!(
            flags.approve_review_template.as_deref(),
            Some("decision-template.json")
        );
        assert_eq!(flags.reviewer_id.as_deref(), Some("reviewer-1"));
        assert_eq!(flags.reviewer_name.as_deref(), Some("Reviewer One"));
        assert_eq!(flags.reason.as_deref(), Some("prototype approved"));
        assert_eq!(flags.decided_at.as_deref(), Some("2026-07-07T00:00:00Z"));
        assert_eq!(flags.out.as_deref(), Some("decision.json"));
    }

    #[test]
    fn parses_materialize_and_run_materialized_paths() {
        let materialize = parse([
            "--materialize-reviewed",
            "next-round.json",
            "--review-decision",
            "decision.json",
        ]);
        let rerun = parse(["--run-materialized", "materialization-plan.json"]);

        assert_eq!(
            materialize.materialize_reviewed.as_deref(),
            Some("next-round.json")
        );
        assert_eq!(
            materialize.review_decision.as_deref(),
            Some("decision.json")
        );
        assert_eq!(
            rerun.run_materialized.as_deref(),
            Some("materialization-plan.json")
        );
    }

    #[test]
    fn rejects_missing_review_template_value() {
        let error = Flags::parse(strings(["--review-template", "--json"])).unwrap_err();

        assert!(error.contains("--review-template requires a value"));
    }

    fn parse<const N: usize>(args: [&str; N]) -> Flags {
        Flags::parse(strings(args)).expect("flags")
    }

    fn strings<const N: usize>(args: [&str; N]) -> Vec<String> {
        args.into_iter().map(ToString::to_string).collect()
    }
}
