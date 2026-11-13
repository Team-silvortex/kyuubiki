use super::*;
use kyuubiki_protocol::{OperatorDeploymentRequest, OperatorDeploymentStatus};
#[cfg(test)]
#[path = "direct_mesh_package_gateway_tests.rs"]
mod tests;
impl DirectMeshGateway {
    pub(super) fn package_deployment(&self, request: &HttpRequest) -> HttpResponse {
        let body = match parse_json_object(request) {
            Ok(v) => v,
            Err(e) => return e,
        };
        if body.len() != 2
            || !body.contains_key("request")
            || request.body.len() > 524288
            || body
                .get("endpoints")
                .and_then(Value::as_array)
                .is_none_or(|a| a.len() != 1)
        {
            return json_response(400, json!({"error":"invalid_package_deployment"}));
        }
        let endpoints = match self.resolve_endpoints(body.get("endpoints")) {
            Ok(e) if e.len() == 1 => e,
            _ => return json_response(400, json!({"error":"single_package_target_required"})),
        };
        let params: OperatorDeploymentRequest =
            match body.get("request").cloned().map(serde_json::from_value) {
                Some(Ok(p)) => p,
                _ => return json_response(400, json!({"error":"invalid_package_deployment"})),
            };
        if params.package.validate().is_err() {
            return json_response(400, json!({"error":"invalid_package_pin"}));
        }
        let security = match api_security::security_for(request, &endpoints) {
            Ok(s) => s,
            Err(e) => return e,
        };
        let local = parse_endpoint(&endpoints[0]).ok().is_some_and(|e| {
            e.host
                .parse::<std::net::IpAddr>()
                .is_ok_and(|ip| ip.is_loopback())
        });
        if security.is_none() && !local {
            return json_response(403, json!({"error":"package_deployment_requires_tls"}));
        }
        match crate::direct_mesh_rpc::request_secured(
            &endpoints[0],
            "manage_operator_package",
            serde_json::to_value(&params).unwrap(),
            security.as_deref(),
        ) {
            Ok((value, progress)) => {
                let status = serde_json::from_value::<OperatorDeploymentStatus>(value);
                match status {
                    Ok(s)
                        if progress.is_empty()
                            && s.validate().is_ok()
                            && s.package == params.package
                            && s.process_instance_id
                                .as_ref()
                                .is_some_and(|p| !p.is_empty() && p.len() <= 256)
                            && params
                                .expected_process_instance_id
                                .as_ref()
                                .is_none_or(|p| s.process_instance_id.as_ref() == Some(p)) =>
                    {
                        json_response(
                            200,
                            json!({"endpoint":endpoints[0],"gateway_runtime":"rust-native","deployment":s}),
                        )
                    }
                    _ => json_response(502, json!({"error":"invalid_package_deployment_receipt"})),
                }
            }
            Err(error) => json_response(
                502,
                json!({"error":"package_deployment_unconfirmed","agent_error_code":error.agent_code}),
            ),
        }
    }
}
