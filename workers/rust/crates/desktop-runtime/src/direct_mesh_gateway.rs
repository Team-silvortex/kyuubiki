use std::collections::{HashSet, VecDeque};
use std::env;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use chrono::Utc;
use serde_json::{Map, Value, json};

use crate::direct_mesh_rpc::{describe_agents, parse_endpoint, solve};
use crate::frontend_http::{HttpRequest, HttpResponse, json_response, query_parameter};

const MAX_ENDPOINTS: usize = 32;
const MAX_RESULTS: usize = 24;
const RESULT_TTL: Duration = Duration::from_secs(10 * 60);
const MAX_CHUNK_LIMIT: usize = 1000;
static NEXT_JOB_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct DirectMeshGateway {
    config: Arc<GatewayConfig>,
    results: Arc<Mutex<VecDeque<StoredResult>>>,
}

#[derive(Clone, Debug)]
struct GatewayConfig {
    enabled: bool,
    token: Option<String>,
    configured_endpoints: Vec<String>,
    allow_request_endpoints: bool,
}

#[derive(Clone, Debug)]
struct StoredResult {
    job_id: String,
    study_kind: String,
    result: Value,
    endpoint: String,
    stored_at: String,
    stored: Instant,
}

impl DirectMeshGateway {
    pub(crate) fn from_env() -> Result<Self, String> {
        let deployment = env::var("KYUUBIKI_DEPLOYMENT_MODE").unwrap_or_else(|_| "local".into());
        let enabled =
            bool_env("KYUUBIKI_DIRECT_MESH_ENABLED")?.unwrap_or_else(|| deployment == "local");
        let allow_request_endpoints =
            bool_env("KYUUBIKI_DIRECT_MESH_ALLOW_REQUEST_ENDPOINTS")?.unwrap_or(false);
        let configured = env::var("KYUUBIKI_DIRECT_MESH_ENDPOINTS")
            .or_else(|_| env::var("KYUUBIKI_AGENT_ENDPOINTS"))
            .unwrap_or_default();
        let configured_endpoints = validate_endpoints(split_endpoints(&configured))?;
        Ok(Self {
            config: Arc::new(GatewayConfig {
                enabled,
                token: env::var("KYUUBIKI_DIRECT_MESH_TOKEN")
                    .ok()
                    .filter(|value| !value.is_empty()),
                configured_endpoints,
                allow_request_endpoints,
            }),
            results: Arc::new(Mutex::new(VecDeque::new())),
        })
    }

    pub(crate) fn handle(&self, request: &HttpRequest) -> HttpResponse {
        if let Some(response) = self.authorize(request) {
            return response;
        }
        match (request.method.as_str(), request.path.as_str()) {
            ("POST", "/api/direct-mesh/agents") => self.inspect_agents(request),
            ("POST", "/api/direct-mesh/solve") => self.run_study(request),
            ("GET" | "HEAD", path) if path.starts_with("/api/direct-mesh/results/") => {
                self.fetch_result(request)
            }
            _ => json_response(404, json!({"error": "direct_mesh_route_not_found"})),
        }
    }

    fn authorize(&self, request: &HttpRequest) -> Option<HttpResponse> {
        if !self.config.enabled {
            return Some(json_response(
                403,
                json!({
                    "error": "direct_mesh_disabled",
                    "message": "direct mesh runtime is disabled for this deployment"
                }),
            ));
        }
        let expected = self.config.token.as_deref()?;
        let supplied = request
            .headers
            .get("authorization")
            .and_then(|value| value.strip_prefix("Bearer "))
            .or_else(|| request.headers.get("x-kyuubiki-token").map(String::as_str))
            .unwrap_or_default();
        if constant_time_equal(expected.as_bytes(), supplied.as_bytes()) {
            None
        } else {
            Some(json_response(
                401,
                json!({
                    "error": "unauthorized",
                    "message": "missing or invalid direct mesh token"
                }),
            ))
        }
    }

    fn inspect_agents(&self, request: &HttpRequest) -> HttpResponse {
        let body = match parse_json_object(request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let endpoints = match self.resolve_endpoints(body.get("endpoints")) {
            Ok(endpoints) => endpoints,
            Err(error) => return json_response(400, json!({"error": error})),
        };
        let agents = describe_agents(&endpoints);
        json_response(
            200,
            json!({
                "mode": "direct_mesh_gui",
                "gateway_contract": "kyuubiki.frontend-runtime-gateway/direct-mesh-v1",
                "gateway_runtime": "rust-native",
                "discovery": "manual",
                "endpoint_count": endpoints.len(),
                "agents": agents,
            }),
        )
    }

    fn run_study(&self, request: &HttpRequest) -> HttpResponse {
        let body = match parse_json_object(request) {
            Ok(body) => body,
            Err(response) => return response,
        };
        let study_kind = match body.get("study_kind").and_then(Value::as_str) {
            Some(value) => value,
            None => return json_response(400, json!({"error": "study_kind is required"})),
        };
        let method = match method_for_study(study_kind) {
            Some(method) => method,
            None => {
                return json_response(
                    400,
                    json!({"error": format!("unsupported direct mesh study: {study_kind}")}),
                );
            }
        };
        let input = match body.get("input") {
            Some(Value::Object(_)) => body.get("input").cloned().unwrap_or_default(),
            _ => return json_response(400, json!({"error": "input must be an object"})),
        };
        let endpoints = match self.resolve_endpoints(body.get("endpoints")) {
            Ok(endpoints) => endpoints,
            Err(error) => return json_response(400, json!({"error": error})),
        };
        let selection_mode = body
            .get("selection_mode")
            .and_then(Value::as_str)
            .unwrap_or("healthiest");
        if !matches!(selection_mode, "healthiest" | "first_reachable") {
            return json_response(400, json!({"error": "invalid direct mesh selection mode"}));
        }

        let started_at = Utc::now().to_rfc3339();
        let solved = match solve(method, input, &endpoints, selection_mode) {
            Ok(solved) => solved,
            Err(error) => return json_response(502, json!({"error": error})),
        };
        let completed_at = Utc::now().to_rfc3339();
        let job_id = format!(
            "direct-{:x}-{:x}",
            Utc::now().timestamp_millis().unsigned_abs(),
            NEXT_JOB_ID.fetch_add(1, Ordering::Relaxed)
        );
        self.store_result(StoredResult {
            job_id: job_id.clone(),
            study_kind: study_kind.to_string(),
            result: solved.result.clone(),
            endpoint: solved.endpoint.clone(),
            stored_at: completed_at.clone(),
            stored: Instant::now(),
        });
        json_response(
            200,
            json!({
                "job": {
                    "job_id": job_id,
                    "status": "completed",
                    "worker_id": format!("direct-mesh@{}", solved.endpoint),
                    "progress": 1,
                    "message": "completed through native direct mesh runtime gateway",
                    "created_at": started_at,
                    "updated_at": completed_at,
                    "has_result": true,
                },
                "result": solved.result,
                "direct_mesh": {
                    "endpoint": solved.endpoint,
                    "strategy": solved.strategy,
                    "progress_frames": solved.progress_frames,
                },
                "gateway_contract": "kyuubiki.frontend-runtime-gateway/direct-mesh-v1",
                "gateway_runtime": "rust-native",
            }),
        )
    }

    fn fetch_result(&self, request: &HttpRequest) -> HttpResponse {
        let tail = request.path.trim_start_matches("/api/direct-mesh/results/");
        let parts = tail.split('/').collect::<Vec<_>>();
        let Some(job_id) = parts.first().filter(|value| !value.is_empty()) else {
            return json_response(404, json!({"error": "direct_mesh_result_not_found"}));
        };
        let Some(result) = self.get_result(job_id) else {
            return json_response(
                404,
                json!({"error": format!("no cached direct mesh result for {job_id}")}),
            );
        };
        if parts.len() == 1 {
            return json_response(
                200,
                json!({
                    "job_id": result.job_id,
                    "study_kind": result.study_kind,
                    "result": result.result,
                    "endpoint": result.endpoint,
                    "stored_at": result.stored_at,
                }),
            );
        }
        if parts.len() != 3 || parts[1] != "chunks" || !matches!(parts[2], "nodes" | "elements") {
            return json_response(400, json!({"error": "unsupported direct mesh chunk kind"}));
        }
        let offset = parse_query_usize(&request.target, "offset", 0, usize::MAX);
        let limit = parse_query_usize(&request.target, "limit", 200, MAX_CHUNK_LIMIT).max(1);
        let collection = result
            .result
            .get(parts[2])
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let offset = offset.min(collection.len());
        let items = collection
            .iter()
            .skip(offset)
            .take(limit)
            .cloned()
            .collect::<Vec<_>>();
        json_response(
            200,
            json!({
                "job_id": result.job_id,
                "kind": parts[2],
                "offset": offset,
                "limit": limit,
                "returned": items.len(),
                "total": collection.len(),
                "items": items,
                "endpoint": result.endpoint,
                "stored_at": result.stored_at,
            }),
        )
    }

    fn resolve_endpoints(&self, requested: Option<&Value>) -> Result<Vec<String>, String> {
        let requested = requested
            .map(endpoint_array)
            .transpose()?
            .unwrap_or_default();
        let requested = validate_endpoints(requested)?;
        if requested.is_empty() {
            if self.config.configured_endpoints.is_empty() {
                return Err("no configured direct mesh endpoints".to_string());
            }
            return Ok(self.config.configured_endpoints.clone());
        }
        if self.config.allow_request_endpoints {
            return Ok(requested);
        }
        if self.config.configured_endpoints.is_empty() {
            return Err(
                "request-defined direct mesh endpoints are disabled for this deployment"
                    .to_string(),
            );
        }
        let allowed = self
            .config
            .configured_endpoints
            .iter()
            .collect::<HashSet<_>>();
        if let Some(endpoint) = requested
            .iter()
            .find(|endpoint| !allowed.contains(endpoint))
        {
            return Err(format!(
                "direct mesh endpoint is not allowlisted: {endpoint}"
            ));
        }
        Ok(requested)
    }

    fn store_result(&self, result: StoredResult) {
        let mut results = self.results.lock().unwrap_or_else(|lock| lock.into_inner());
        prune_results(&mut results);
        results.push_front(result);
        results.truncate(MAX_RESULTS);
    }

    fn get_result(&self, job_id: &str) -> Option<StoredResult> {
        let mut results = self.results.lock().unwrap_or_else(|lock| lock.into_inner());
        prune_results(&mut results);
        results
            .iter()
            .find(|result| result.job_id == job_id)
            .cloned()
    }
}

fn parse_json_object(request: &HttpRequest) -> Result<Map<String, Value>, HttpResponse> {
    if request.body.is_empty() {
        return Ok(Map::new());
    }
    match serde_json::from_slice::<Value>(&request.body) {
        Ok(Value::Object(body)) => Ok(body),
        Ok(_) => Err(json_response(
            400,
            json!({"error": "JSON body must be an object"}),
        )),
        Err(error) => Err(json_response(
            400,
            json!({"error": format!("invalid JSON: {error}")}),
        )),
    }
}

fn endpoint_array(value: &Value) -> Result<Vec<String>, String> {
    value
        .as_array()
        .ok_or_else(|| "endpoints must be an array".to_string())?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(ToString::to_string)
                .ok_or_else(|| "direct mesh endpoints must be strings".to_string())
        })
        .collect()
}

fn split_endpoints(value: &str) -> Vec<String> {
    value.split(',').map(ToString::to_string).collect()
}

fn validate_endpoints(values: Vec<String>) -> Result<Vec<String>, String> {
    let mut seen = HashSet::new();
    let mut endpoints = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        let endpoint = parse_endpoint(value)?.endpoint;
        if seen.insert(endpoint.clone()) {
            endpoints.push(endpoint);
        }
    }
    if endpoints.len() > MAX_ENDPOINTS {
        return Err(format!(
            "too many direct mesh endpoints; max {MAX_ENDPOINTS}"
        ));
    }
    Ok(endpoints)
}

fn bool_env(name: &str) -> Result<Option<bool>, String> {
    match env::var(name).ok().as_deref().map(str::trim) {
        None | Some("") => Ok(None),
        Some("true" | "1") => Ok(Some(true)),
        Some("false" | "0") => Ok(Some(false)),
        Some(_) => Err(format!("{name} must be true, false, 1, or 0")),
    }
}

fn method_for_study(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "axial_bar_1d" => "solve_bar_1d",
        "thermal_bar_1d" => "solve_thermal_bar_1d",
        "heat_bar_1d" => "solve_heat_bar_1d",
        "electrostatic_plane_triangle_2d" => "solve_electrostatic_plane_triangle_2d",
        "electrostatic_plane_quad_2d" => "solve_electrostatic_plane_quad_2d",
        "heat_plane_triangle_2d" => "solve_heat_plane_triangle_2d",
        "heat_plane_quad_2d" => "solve_heat_plane_quad_2d",
        "thermal_truss_2d" => "solve_thermal_truss_2d",
        "thermal_truss_3d" => "solve_thermal_truss_3d",
        "spring_1d" => "solve_spring_1d",
        "spring_2d" => "solve_spring_2d",
        "spring_3d" => "solve_spring_3d",
        "beam_1d" => "solve_beam_1d",
        "thermal_beam_1d" => "solve_thermal_beam_1d",
        "thermal_frame_2d" => "solve_thermal_frame_2d",
        "torsion_1d" => "solve_torsion_1d",
        "truss_2d" => "solve_truss_2d",
        "truss_3d" => "solve_truss_3d",
        "plane_triangle_2d" => "solve_plane_triangle_2d",
        "thermal_plane_triangle_2d" => "solve_thermal_plane_triangle_2d",
        "plane_quad_2d" => "solve_plane_quad_2d",
        "thermal_plane_quad_2d" => "solve_thermal_plane_quad_2d",
        "frame_2d" => "solve_frame_2d",
        _ => return None,
    })
}

fn prune_results(results: &mut VecDeque<StoredResult>) {
    results.retain(|result| result.stored.elapsed() <= RESULT_TTL);
}

fn parse_query_usize(target: &str, name: &str, default: usize, max: usize) -> usize {
    query_parameter(target, name)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(default)
        .min(max)
}

fn constant_time_equal(expected: &[u8], supplied: &[u8]) -> bool {
    let mut difference = expected.len() ^ supplied.len();
    let length = expected.len().max(supplied.len());
    for index in 0..length {
        difference |= usize::from(
            expected.get(index).copied().unwrap_or(0) ^ supplied.get(index).copied().unwrap_or(0),
        );
    }
    difference == 0
}

#[cfg(test)]
mod tests {
    use super::{constant_time_equal, method_for_study, validate_endpoints};

    #[test]
    fn endpoint_lists_are_normalized_and_bounded() {
        let endpoints = validate_endpoints(vec![
            "127.0.0.1:5001".to_string(),
            " 127.0.0.1:5001 ".to_string(),
        ])
        .unwrap();
        assert_eq!(endpoints, ["127.0.0.1:5001"]);
    }

    #[test]
    fn study_mapping_is_an_explicit_allowlist() {
        assert_eq!(method_for_study("truss_2d"), Some("solve_truss_2d"));
        assert_eq!(method_for_study("shell"), None);
    }

    #[test]
    fn token_comparison_checks_length_and_content() {
        assert!(constant_time_equal(b"token", b"token"));
        assert!(!constant_time_equal(b"token", b"tokens"));
        assert!(!constant_time_equal(b"token", b"toker"));
    }
}
