use std::ffi::OsString;

type RunnerResult<T> = Result<T, String>;

#[derive(Default)]
pub(super) struct ReportOptions {
    pub(super) strict: bool,
    pub(super) strict_language: Option<String>,
    pub(super) language: Option<String>,
    pub(super) batch: Option<String>,
    pub(super) template_out: Option<String>,
    pub(super) apply_from: Option<String>,
    pub(super) help: bool,
}

impl ReportOptions {
    pub(super) fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--strict" => options.strict = true,
                "--strict-language" => {
                    options.strict_language = Some(next_value(&mut args, "--strict-language")?)
                }
                "--language" => options.language = Some(next_value(&mut args, "--language")?),
                "--batch" => options.batch = Some(next_value(&mut args, "--batch")?),
                "--template-out" => {
                    options.template_out = Some(next_value(&mut args, "--template-out")?)
                }
                "--apply-from" => options.apply_from = Some(next_value(&mut args, "--apply-from")?),
                "--help" | "-h" => options.help = true,
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(options)
    }

    pub(super) fn template_request(&self) -> RunnerResult<Option<(&str, &str, &str)>> {
        match (
            self.language.as_deref(),
            self.batch.as_deref(),
            self.template_out.as_deref(),
        ) {
            (None, None, None) => Ok(None),
            (Some(language), Some(batch), Some(output)) => Ok(Some((language, batch, output))),
            _ => Err("--batch, --language, and --template-out must be provided together".into()),
        }
    }
}

#[derive(Default)]
pub(super) struct PlanOptions {
    pub(super) language: Option<String>,
    pub(super) next: bool,
    pub(super) json: bool,
    pub(super) help: bool,
}

impl PlanOptions {
    pub(super) fn parse(args: Vec<OsString>) -> RunnerResult<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.to_string_lossy().as_ref() {
                "--language" => options.language = Some(next_value(&mut args, "--language")?),
                "--next" => options.next = true,
                "--json" => options.json = true,
                "--help" | "-h" => options.help = true,
                other => return Err(format!("unknown argument {other}")),
            }
        }
        Ok(options)
    }
}

fn next_value(args: &mut impl Iterator<Item = OsString>, option: &str) -> RunnerResult<String> {
    args.next()
        .and_then(|value| value.into_string().ok())
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("{option} requires a value"))
}
