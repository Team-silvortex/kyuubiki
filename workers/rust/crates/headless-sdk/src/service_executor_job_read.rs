use crate::HeadlessExecutorError;
use crate::service_executor::validate_path_segment;
use serde_json::{Map, Value};

pub(crate) const INVALID_READ: &str = "job_read_invalid:";
const CONTEXT: &[(&str, &str)] = &[
    ("project_id", "projectId"),
    ("model_version_id", "modelVersionId"),
    ("simulation_case_id", "simulationCaseId"),
];

pub(crate) struct JobReadRequest<'a> {
    pub job_id: &'a str,
    expected: Map<String, Value>,
}

impl<'a> JobReadRequest<'a> {
    pub fn parse(payload: &'a Value) -> Result<Self, HeadlessExecutorError> {
        let job = identity(payload, "job_id", "jobId", false)?
            .ok_or_else(|| invalid("missing task identity"))?;
        let mut expected = Map::new();
        for (key, alias) in CONTEXT {
            if let Some(value) = identity(payload, key, alias, *key == "model_version_id")? {
                expected.insert((*key).into(), value.clone());
            }
        }
        Ok(Self {
            job_id: job.as_str().expect("validated nonnull task identity"),
            expected,
        })
    }

    pub fn validate_context(&self, envelope: &Value) -> Result<(), HeadlessExecutorError> {
        let job = envelope
            .get("job")
            .and_then(Value::as_object)
            .ok_or_else(|| invalid("missing job context"))?;
        for (key, _) in CONTEXT {
            if self
                .expected
                .get(*key)
                .is_some_and(|expected| job.get(*key) != Some(expected))
            {
                return Err(invalid(
                    "job context does not match the requested association",
                ));
            }
            if envelope
                .get(*key)
                .is_some_and(|value| job.get(*key) != Some(value))
            {
                return Err(invalid("repeated job context is contradictory"));
            }
        }
        Ok(())
    }

    pub fn validate_result_context(
        &self,
        verified_job: &Value,
        result: &Value,
    ) -> Result<(), HeadlessExecutorError> {
        if result.get("job").is_some() {
            self.validate_context(result)?;
        }
        for (key, _) in CONTEXT {
            let expected = verified_job.get(*key);
            if let Some(job) = result.get("job") {
                if expected.is_some() && job.get(*key) != expected {
                    return Err(invalid("job and result reads disagree on association"));
                }
            }
            if let Some(value) = result.get(*key) {
                if expected != Some(value) {
                    return Err(invalid("result context contradicts the verified job"));
                }
            }
        }
        Ok(())
    }
}

pub(crate) fn prefer_job_result(payload: &Value) -> Result<bool, HeadlessExecutorError> {
    let mut selected = None;
    for key in ["prefer_job_result", "preferJobResult"] {
        if let Some(value) = payload.get(key) {
            let value = value
                .as_bool()
                .ok_or_else(|| invalid("result preference must be boolean"))?;
            if selected.is_some_and(|previous| previous != value) {
                return Err(invalid("contradictory result preference aliases"));
            }
            selected = Some(value);
        }
    }
    Ok(selected.unwrap_or(true))
}

fn identity<'a>(
    payload: &'a Value,
    key: &str,
    alias: &str,
    nullable: bool,
) -> Result<Option<&'a Value>, HeadlessExecutorError> {
    let mut selected = None;
    for name in [key, alias] {
        if let Some(value) = payload.get(name) {
            if !(nullable && value.is_null()) {
                let id = value
                    .as_str()
                    .ok_or_else(|| invalid("read identity must be a string"))?;
                validate_path_segment(id, key).map_err(|_| invalid("read identity is unusable"))?;
            }
            if selected.is_some_and(|previous| previous != value) {
                return Err(invalid("contradictory read identity aliases"));
            }
            selected = Some(value);
        }
    }
    Ok(selected)
}

fn invalid(detail: &str) -> HeadlessExecutorError {
    HeadlessExecutorError {
        message: format!("{INVALID_READ} {detail}"),
    }
}
