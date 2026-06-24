#[derive(Debug, Default)]
pub(crate) struct Flags {
    pub(crate) positional: Vec<String>,
    pub(crate) json: bool,
    pub(crate) execute: bool,
    pub(crate) executor: Option<String>,
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
