pub(super) const MAX_JOB_WAIT_TIMEOUT_MS: u64 = 86_400_000;

#[derive(Debug, Default)]
pub(crate) struct Flags {
    pub(crate) positional: Vec<String>,
    pub(crate) json: bool,
    pub(crate) execute: bool,
    pub(crate) executor: Option<String>,
    pub(crate) execution_posture: Option<String>,
    pub(crate) allow_sensitive: bool,
    pub(crate) allow_destructive: bool,
    pub(crate) api_base_url: Option<String>,
    pub(crate) api_token: Option<String>,
    pub(crate) runtime: Option<String>,
    pub(crate) category: Option<String>,
    pub(crate) tag: Option<String>,
    pub(crate) query: Option<String>,
    pub(crate) template: Option<String>,
    pub(crate) workflow_id: Option<String>,
    pub(crate) out: Option<String>,
    pub(crate) report_out: Option<String>,
    pub(crate) material_report: Option<String>,
    pub(crate) material_report_out: Option<String>,
    pub(crate) parameter_patch: Option<String>,
    pub(crate) parameter_patch_receipt_out: Option<String>,
    pub(crate) research_round_spec: Option<String>,
    pub(crate) previous_round_evidence: Option<String>,
    pub(crate) research_round_out: Option<String>,
    pub(crate) job_wait_timeout_ms: Option<u64>,
}

impl Flags {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        let mut flags = Self::default();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--json" => flags.json = true,
                "--execute" => flags.execute = true,
                "--executor" => {
                    flags.executor = Some(take_value(args, &mut index, "--executor")?);
                }
                "--execution-posture" => {
                    flags.execution_posture =
                        Some(take_value(args, &mut index, "--execution-posture")?);
                }
                "--allow-sensitive" => flags.allow_sensitive = true,
                "--allow-destructive" => flags.allow_destructive = true,
                "--api-base-url" => {
                    flags.api_base_url = Some(take_value(args, &mut index, "--api-base-url")?);
                }
                "--api-token" => {
                    flags.api_token = Some(take_value(args, &mut index, "--api-token")?);
                }
                "--runtime" | "--runtime-style" => {
                    flags.runtime = Some(take_value(args, &mut index, "--runtime")?);
                }
                "--category" => {
                    flags.category = Some(take_value(args, &mut index, "--category")?);
                }
                "--tag" => {
                    flags.tag = Some(take_value(args, &mut index, "--tag")?);
                }
                "--query" | "--search" => {
                    flags.query = Some(take_value(args, &mut index, "--query")?);
                }
                "--template" => {
                    flags.template = Some(take_value(args, &mut index, "--template")?);
                }
                "--workflow-id" => {
                    flags.workflow_id = Some(take_value(args, &mut index, "--workflow-id")?);
                }
                "--out" => {
                    flags.out = Some(take_value(args, &mut index, "--out")?);
                }
                "--report-out" => {
                    flags.report_out = Some(take_value(args, &mut index, "--report-out")?);
                }
                "--material-report" => {
                    flags.material_report =
                        Some(take_value(args, &mut index, "--material-report")?);
                }
                "--material-report-out" => {
                    flags.material_report_out =
                        Some(take_value(args, &mut index, "--material-report-out")?);
                }
                "--parameter-patch" => {
                    flags.parameter_patch =
                        Some(take_value(args, &mut index, "--parameter-patch")?);
                }
                "--parameter-patch-receipt-out" => {
                    flags.parameter_patch_receipt_out = Some(take_value(
                        args,
                        &mut index,
                        "--parameter-patch-receipt-out",
                    )?);
                }
                "--research-round-spec" => {
                    flags.research_round_spec =
                        Some(take_value(args, &mut index, "--research-round-spec")?);
                }
                "--previous-round-evidence" => {
                    flags.previous_round_evidence =
                        Some(take_value(args, &mut index, "--previous-round-evidence")?);
                }
                "--research-round-out" => {
                    flags.research_round_out =
                        Some(take_value(args, &mut index, "--research-round-out")?);
                }
                "--job-wait-timeout-ms" => {
                    let value = take_value(args, &mut index, "--job-wait-timeout-ms")?;
                    flags.job_wait_timeout_ms = Some(parse_job_wait_timeout_ms(&value)?);
                }
                value if value.starts_with("--") => {
                    return Err(format!("unknown option: {value}"));
                }
                value => flags.positional.push(value.to_string()),
            }
            index += 1;
        }
        Ok(flags)
    }

    pub(crate) fn input_path(&self) -> Result<String, String> {
        self.positional
            .first()
            .cloned()
            .ok_or_else(|| "command requires an input path".to_string())
    }

    pub(crate) fn selected_executor(&self) -> Result<Option<&str>, String> {
        if !self.execute {
            if self.executor.is_some() || self.execution_posture.is_some() {
                return Err("--executor and --execution-posture require --execute".to_string());
            }
            return Ok(None);
        }
        let executor = self.executor.as_deref().ok_or_else(|| {
            "--execute requires an explicit --executor; use mock only for previews or service for real execution"
                .to_string()
        })?;
        if !matches!(executor, "mock" | "service" | "hybrid") {
            return Err(if executor == "direct-mesh" {
                "unsupported executor \"direct-mesh\"; direct_mesh_solve is a service action, so use --executor service (or hybrid for workflows that also contain browser actions)"
                    .to_string()
            } else {
                format!(
                    "unsupported executor \"{executor}\"; available executors: mock, service, hybrid"
                )
            });
        }
        let posture = self.execution_posture.as_deref().unwrap_or("preview");
        match posture {
            "preview" => {}
            "research" if executor == "service" => {}
            "research" => {
                return Err(format!(
                    "research execution requires --executor service; {executor} cannot provide a no-mock execution guarantee"
                ));
            }
            other => {
                return Err(format!(
                    "unsupported execution posture \"{other}\"; available: preview, research"
                ));
            }
        }
        Ok(Some(executor))
    }
}

fn parse_job_wait_timeout_ms(value: &str) -> Result<u64, String> {
    let timeout = value
        .parse::<u64>()
        .map_err(|_| format!("--job-wait-timeout-ms must be a positive integer, got {value}"))?;
    if timeout == 0 || timeout > MAX_JOB_WAIT_TIMEOUT_MS {
        return Err(format!(
            "--job-wait-timeout-ms must be between 1 and {MAX_JOB_WAIT_TIMEOUT_MS}"
        ));
    }
    Ok(timeout)
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

#[cfg(test)]
mod tests {
    use super::Flags;

    fn flags(args: &[&str]) -> Flags {
        Flags::parse(
            &args
                .iter()
                .map(|value| (*value).to_string())
                .collect::<Vec<_>>(),
        )
        .expect("flags")
    }

    #[test]
    fn execute_requires_explicit_executor() {
        let error = flags(&["workflow.json", "--execute"])
            .selected_executor()
            .expect_err("executor should be required");

        assert!(error.contains("explicit --executor"));
    }

    #[test]
    fn research_posture_only_accepts_service_executor() {
        let service = flags(&[
            "workflow.json",
            "--execute",
            "--executor",
            "service",
            "--execution-posture",
            "research",
        ]);
        let mock = flags(&[
            "workflow.json",
            "--execute",
            "--executor",
            "mock",
            "--execution-posture",
            "research",
        ]);

        assert_eq!(
            service.selected_executor().expect("service"),
            Some("service")
        );
        assert!(
            mock.selected_executor()
                .expect_err("mock should fail")
                .contains("no-mock execution guarantee")
        );
    }

    #[test]
    fn preview_posture_keeps_explicit_mock_available() {
        let preview = flags(&[
            "workflow.json",
            "--execute",
            "--executor",
            "mock",
            "--execution-posture",
            "preview",
        ]);

        assert_eq!(preview.selected_executor().expect("preview"), Some("mock"));
    }

    #[test]
    fn direct_mesh_is_reported_as_a_service_route_not_an_executor() {
        let direct_mesh = flags(&["workflow.json", "--execute", "--executor", "direct-mesh"]);

        let error = direct_mesh
            .selected_executor()
            .expect_err("direct-mesh executor alias should fail");
        assert!(error.contains("direct_mesh_solve is a service action"));
        assert!(error.contains("--executor service"));
    }

    #[test]
    fn parses_bounded_job_wait_timeout_override() {
        let parsed = flags(&["workflow.json", "--job-wait-timeout-ms", "1200000"]);
        assert_eq!(parsed.job_wait_timeout_ms, Some(1_200_000));

        for invalid in ["0", "86400001", "later"] {
            let error = Flags::parse(
                &["workflow.json", "--job-wait-timeout-ms", invalid]
                    .into_iter()
                    .map(str::to_string)
                    .collect::<Vec<_>>(),
            )
            .expect_err("invalid timeout should fail");
            assert!(error.contains("--job-wait-timeout-ms"));
        }
    }

    #[test]
    fn parses_parameter_patch_path() {
        let parsed = flags(&[
            "workflow.json",
            "--parameter-patch",
            "round-2.json",
            "--parameter-patch-receipt-out",
            "round-2.receipt.json",
        ]);
        assert_eq!(parsed.parameter_patch.as_deref(), Some("round-2.json"));
        assert_eq!(
            parsed.parameter_patch_receipt_out.as_deref(),
            Some("round-2.receipt.json")
        );
    }

    #[test]
    fn parses_research_round_artifact_paths() {
        let parsed = flags(&[
            "workflow.json",
            "--research-round-spec",
            "round-2.spec.json",
            "--previous-round-evidence",
            "round-1.evidence.json",
            "--research-round-out",
            "round-2.evidence.json",
        ]);
        assert_eq!(
            parsed.research_round_spec.as_deref(),
            Some("round-2.spec.json")
        );
        assert_eq!(
            parsed.previous_round_evidence.as_deref(),
            Some("round-1.evidence.json")
        );
        assert_eq!(
            parsed.research_round_out.as_deref(),
            Some("round-2.evidence.json")
        );
    }
}
