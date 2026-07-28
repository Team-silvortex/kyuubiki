use serde_json::Value;

type RunnerResult<T> = Result<T, String>;

pub(crate) fn validate_execution_authority_trace(bundle: &Value) -> RunnerResult<()> {
    if bundle
        .pointer("/execution_trace/authority/schema_version")
        .and_then(Value::as_str)
        != Some("kyuubiki.research-execution-authority-trace/v1")
    {
        return Err("execution_trace.authority.schema_version is invalid".to_string());
    }
    for assertion in ["all_real_solver", "no_mock_execution", "no_fallback"] {
        if bundle
            .pointer(&format!(
                "/execution_trace/authority/assertions/{assertion}"
            ))
            .and_then(Value::as_bool)
            != Some(true)
        {
            return Err(format!(
                "execution_trace.authority.assertions.{assertion} must be true"
            ));
        }
    }
    for pointer in [
        "/execution_trace/authority/initial",
        "/execution_trace/authority/next",
    ] {
        validate_real_solver_authority(
            bundle
                .pointer(pointer)
                .ok_or_else(|| format!("{pointer}: missing execution authority"))?,
            pointer,
        )?;
    }
    let chain = bundle
        .pointer("/execution_trace/authority/chain")
        .and_then(Value::as_array)
        .filter(|items| !items.is_empty())
        .ok_or_else(|| "execution_trace.authority.chain must be non-empty".to_string())?;
    for (index, authority) in chain.iter().enumerate() {
        validate_real_solver_authority(
            authority,
            &format!("/execution_trace/authority/chain/{index}"),
        )?;
    }
    Ok(())
}

fn validate_real_solver_authority(authority: &Value, field: &str) -> RunnerResult<()> {
    if authority.get("execution_class").and_then(Value::as_str) != Some("real_solver") {
        return Err(format!("{field}/execution_class must be real_solver"));
    }
    if authority.get("mock_execution").and_then(Value::as_bool) != Some(false) {
        return Err(format!("{field}/mock_execution must be false"));
    }
    if authority.get("fallback_used").and_then(Value::as_bool) != Some(false) {
        return Err(format!("{field}/fallback_used must be false"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_execution_authority_trace;
    use serde_json::json;

    #[test]
    fn rejects_mock_or_missing_evidence() {
        let missing = json!({});
        let mock = json!({
            "execution_trace": {
                "authority": {
                    "schema_version": "kyuubiki.research-execution-authority-trace/v1",
                    "initial": real_authority(),
                    "next": real_authority(),
                    "chain": [real_authority()],
                    "assertions": {
                        "all_real_solver": true,
                        "no_mock_execution": false,
                        "no_fallback": true
                    }
                }
            }
        });

        assert!(validate_execution_authority_trace(&missing).is_err());
        assert!(
            validate_execution_authority_trace(&mock)
                .expect_err("mock assertion should fail")
                .contains("no_mock_execution")
        );
    }

    fn real_authority() -> serde_json::Value {
        json!({
            "execution_class": "real_solver",
            "mock_execution": false,
            "fallback_used": false
        })
    }
}
