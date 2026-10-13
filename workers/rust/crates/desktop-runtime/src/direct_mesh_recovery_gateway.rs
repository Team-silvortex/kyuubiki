use super::*;

impl DirectMeshGateway {
    pub(super) fn durable_control(&self, request: &HttpRequest, cancel: bool) -> HttpResponse {
        let body = match parse_json_object(request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let endpoints = match self.resolve_endpoints(body.get("endpoints")) {
            Ok(endpoints) if endpoints.len() == 1 => endpoints,
            _ => {
                return json_response(
                    400,
                    json!({"error":"recovery requires one allowed pinned endpoint"}),
                );
            }
        };
        let params = if cancel {
            json!({"execution_id":body.get("execution_id"), "process_instance_id":body.get("process_instance_id"),
                "request_digest":body.get("request_digest")})
        } else {
            json!({"limit":body.get("limit"), "after":body.get("after")})
        };
        if let Err(error) = validate_query(&params, cancel) {
            return json_response(400, json!({"error":error}));
        }
        let method = if cancel {
            "cancel_durable_agent_execution"
        } else {
            "list_durable_agent_executions"
        };
        let result = crate::direct_mesh_rpc::request(&endpoints[0], method, params.clone())
            .and_then(|(report, _)| {
                validate_control_report(&report, &params, cancel)?;
                Ok(report)
            });
        match result {
            Ok(report) => json_response(
                200,
                json!({"recovery":report, "endpoint":endpoints[0],
                "gateway_runtime":"rust-native"}),
            ),
            Err(error) => json_response(
                502,
                json!({"error":error, "endpoint":endpoints[0],
                "automatic_replay_authorized":false, "required_action":"inspect_execution_on_original_agent"}),
            ),
        }
    }
}

fn hex(value: &Value, length: usize) -> bool {
    value.as_str().is_some_and(|s| {
        s.len() == length
            && s.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

fn text(value: &Value) -> bool {
    value
        .as_str()
        .is_some_and(|s| !s.is_empty() && s.len() <= 256)
}

fn validate_query(params: &Value, cancel: bool) -> Result<(), String> {
    let valid = if cancel {
        hex(&params["execution_id"], 32)
            && hex(&params["request_digest"], 64)
            && text(&params["process_instance_id"])
    } else {
        params["limit"]
            .as_u64()
            .is_some_and(|v| (1..=64).contains(&v))
            && (params["after"].is_null() || hex(&params["after"], 32))
    };
    if valid {
        Ok(())
    } else {
        Err("invalid durable recovery query".into())
    }
}

pub(super) fn validate_control_report(
    report: &Value,
    params: &Value,
    cancel: bool,
) -> Result<(), String> {
    let error = || "invalid durable recovery response".to_string();
    if report["automatic_replay_authorized"] != false {
        return Err(error());
    }
    if cancel {
        let status = report["status"].as_str().ok_or_else(error)?;
        if report["schema_version"] != "kyuubiki.agent-durable-cancellation/v1"
            || report["execution_id"] != params["execution_id"]
            || report["request_digest"] != params["request_digest"]
            || report["process_instance_id"] != params["process_instance_id"]
            || !matches!(status, "requested" | "not_running" | "owner_not_observed")
            || report["cancel_registered"] != (status == "requested")
            || report["execution_terminal_confirmed"] != false
        {
            return Err(error());
        }
    } else {
        if report["schema_version"] != "kyuubiki.agent-durable-execution-index/v1" {
            return Err(error());
        }
        let rows = report["executions"].as_array().ok_or_else(error)?;
        let limit = params["limit"].as_u64().ok_or_else(error)? as usize;
        if rows.len() > limit {
            return Err(error());
        }
        let mut previous = params["after"].as_str().unwrap_or("");
        for row in rows {
            if row["schema_version"] != "kyuubiki.agent-durable-execution/v1"
                || !hex(&row["execution_id"], 32)
                || !hex(&row["request_digest"], 64)
                || !text(&row["process_instance_id"])
                || !text(&row["request_id"])
                || !text(&row["method"])
                || row.as_object().is_none_or(|object| object.len() != 6)
            {
                return Err(error());
            }
            let id = row["execution_id"].as_str().ok_or_else(error)?;
            if id <= previous {
                return Err(error());
            }
            previous = id;
        }
        if report.get("next_cursor").is_none() {
            return Err(error());
        }
        if !report["next_cursor"].is_null()
            && (rows.len() != limit || rows.is_empty() || report["next_cursor"] != previous)
        {
            return Err(error());
        }
    }
    Ok(())
}
