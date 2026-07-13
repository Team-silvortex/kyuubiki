use serde_json::{Map, Value, json};

const DEFAULT_DOMAIN_ORDER: &[&str] = &[
    "structural",
    "thermal",
    "thermo",
    "electrostatic",
    "magnetostatic",
    "cfd",
    "transport",
    "acoustic",
    "modal",
    "dynamic",
];

pub fn evaluate_coupled_readiness(payload: Value, config: Value) -> Result<Value, String> {
    let domains = domain_object(&payload)?;
    let required_domains = configured_domains(&config, "required_domains");
    let optional_domains = configured_domains(&config, "optional_domains");
    let mut domain_names = ordered_domain_names(domains, &required_domains, &optional_domains);
    if domain_names.is_empty() {
        domain_names = DEFAULT_DOMAIN_ORDER
            .iter()
            .filter(|domain| domains.contains_key(**domain))
            .map(|domain| (*domain).to_string())
            .collect();
    }
    if domain_names.is_empty() && required_domains.is_empty() {
        return Err(
            "transform.evaluate_coupled_readiness requires at least one domain".to_string(),
        );
    }

    let mut summaries = Vec::new();
    let mut ready_count = 0_u64;
    let mut present_count = 0_u64;
    let mut blocking_domains = Vec::new();
    let mut warning_domains = Vec::new();
    let mut total_score = 0.0_f64;
    let mut scored_count = 0_u64;

    for domain in domain_names {
        let required = required_domains.iter().any(|value| value == &domain);
        let summary = evaluate_domain(domains.get(&domain), &domain, required, &config)?;
        if summary.present {
            present_count += 1;
        }
        if summary.ready {
            ready_count += 1;
        }
        if let Some(score) = summary.score {
            total_score += score;
            scored_count += 1;
        }
        if summary.state == "block" {
            blocking_domains.push(Value::String(domain.clone()));
        } else if summary.state == "warn" {
            warning_domains.push(Value::String(domain.clone()));
        }
        summaries.push(summary.into_value());
    }

    let required_missing = required_domains
        .iter()
        .filter(|domain| !domains.contains_key(domain.as_str()))
        .cloned()
        .map(Value::String)
        .collect::<Vec<_>>();
    let required_ready = required_domains.iter().all(|domain| {
        summaries.iter().any(|summary| {
            summary.get("domain").and_then(Value::as_str) == Some(domain.as_str())
                && summary.get("ready").and_then(Value::as_bool) == Some(true)
        })
    });
    let ready = required_ready && blocking_domains.is_empty() && required_missing.is_empty();
    let state = if !ready {
        "block"
    } else if warning_domains.is_empty() {
        "pass"
    } else {
        "warn"
    };
    let recommendation = match state {
        "pass" => "continue_to_next_round",
        "warn" => "review_before_next_round",
        _ => "hold_and_repair_inputs",
    };
    let average_score = (scored_count > 0).then_some(total_score / scored_count as f64);

    Ok(json!({
        "coupled_readiness_contract": "kyuubiki.coupled_readiness/v1",
        "coupled_readiness_ready": ready,
        "coupled_readiness_state": state,
        "coupled_readiness_domain_count": summaries.len(),
        "coupled_readiness_present_count": present_count,
        "coupled_readiness_ready_count": ready_count,
        "coupled_readiness_scored_count": scored_count,
        "coupled_readiness_average_score": average_score,
        "coupled_readiness_required_domains": required_domains,
        "coupled_readiness_required_missing": required_missing,
        "coupled_readiness_blocking_domains": blocking_domains,
        "coupled_readiness_warning_domains": warning_domains,
        "coupled_readiness_domains": summaries,
        "coupled_readiness_recommendation": recommendation,
        "coupled_readiness_summary": format!(
            "Coupled readiness {state}: ready={ready_count}/{present_count} present domains, required_ready={required_ready}, scored={scored_count}, recommendation={recommendation}."
        ),
    }))
}

struct DomainSummary {
    domain: String,
    present: bool,
    required: bool,
    ready: bool,
    state: &'static str,
    score: Option<f64>,
    grade: Option<String>,
    guard_state: Option<String>,
    reasons: Vec<Value>,
}

impl DomainSummary {
    fn into_value(self) -> Value {
        json!({
            "domain": self.domain,
            "present": self.present,
            "required": self.required,
            "ready": self.ready,
            "state": self.state,
            "score": self.score,
            "grade": self.grade,
            "guard_state": self.guard_state,
            "reasons": self.reasons,
        })
    }
}

fn evaluate_domain(
    value: Option<&Value>,
    domain: &str,
    required: bool,
    config: &Value,
) -> Result<DomainSummary, String> {
    let Some(value) = value else {
        return Ok(DomainSummary {
            domain: domain.to_string(),
            present: false,
            required,
            ready: !required,
            state: if required { "block" } else { "pass" },
            score: None,
            grade: None,
            guard_state: None,
            reasons: if required {
                vec![json!({"reason": "required_domain_missing"})]
            } else {
                Vec::new()
            },
        });
    };
    let object = value
        .as_object()
        .ok_or_else(|| format!("domain {domain} readiness payload must be an object"))?;
    let score = domain_number(object, domain, "quality_score")
        .or_else(|| number_field(object, "score"))
        .or_else(|| number_field(object, "quality_score"));
    let grade = domain_string(object, domain, "quality_grade")
        .or_else(|| string_field(object, "grade"))
        .or_else(|| string_field(object, "quality_grade"));
    let guard_state = domain_string(object, domain, "guard_state")
        .or_else(|| string_field(object, "guard_state").or_else(|| string_field(object, "state")));
    let ready_field = domain_bool(object, domain, "quality_ready")
        .or_else(|| bool_field(object, "ready"))
        .or_else(|| bool_field(object, "quality_ready"));
    let max_score = domain_config(config, domain)
        .and_then(|value| value.get("max_score"))
        .and_then(Value::as_f64);
    let min_score = domain_config(config, domain)
        .and_then(|value| value.get("min_score"))
        .and_then(Value::as_f64);

    let mut reasons = Vec::new();
    if ready_field == Some(false) {
        reasons.push(json!({"reason": "ready_field_false"}));
    }
    if grade.as_deref() == Some("block") {
        reasons.push(json!({"reason": "quality_grade_block"}));
    }
    if guard_state.as_deref() == Some("block") {
        reasons.push(json!({"reason": "guard_state_block"}));
    }
    if let (Some(score), Some(max_score)) = (score, max_score) {
        if score > max_score {
            reasons
                .push(json!({"reason": "score_above_max", "score": score, "max_score": max_score}));
        }
    }
    if let (Some(score), Some(min_score)) = (score, min_score) {
        if score < min_score {
            reasons
                .push(json!({"reason": "score_below_min", "score": score, "min_score": min_score}));
        }
    }

    let warn = grade.as_deref() == Some("watch") || guard_state.as_deref() == Some("warn");
    let ready = reasons.is_empty() && ready_field.unwrap_or(true);
    Ok(DomainSummary {
        domain: domain.to_string(),
        present: true,
        required,
        ready,
        state: if ready {
            if warn { "warn" } else { "pass" }
        } else {
            "block"
        },
        score,
        grade,
        guard_state,
        reasons,
    })
}

fn domain_object(payload: &Value) -> Result<&Map<String, Value>, String> {
    let object = payload.as_object().ok_or_else(|| {
        "transform.evaluate_coupled_readiness expects an object payload".to_string()
    })?;
    object
        .get("domains")
        .and_then(Value::as_object)
        .or(Some(object))
        .ok_or_else(|| "transform.evaluate_coupled_readiness domains must be an object".to_string())
}

fn ordered_domain_names(
    domains: &Map<String, Value>,
    required: &[String],
    optional: &[String],
) -> Vec<String> {
    let mut names = Vec::new();
    for domain in required.iter().chain(optional.iter()) {
        if !names.contains(domain) {
            names.push(domain.clone());
        }
    }
    for domain in DEFAULT_DOMAIN_ORDER {
        if domains.contains_key(*domain) && !names.iter().any(|value| value == domain) {
            names.push((*domain).to_string());
        }
    }
    for domain in domains.keys() {
        if !names.contains(domain) {
            names.push(domain.clone());
        }
    }
    names
}

fn configured_domains(config: &Value, field: &str) -> Vec<String> {
    config
        .get(field)
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn domain_config<'a>(config: &'a Value, domain: &str) -> Option<&'a Value> {
    config.get("domains").and_then(|value| value.get(domain))
}

fn domain_bool(object: &Map<String, Value>, domain: &str, suffix: &str) -> Option<bool> {
    bool_field(object, &format!("{domain}_{suffix}"))
}

fn domain_number(object: &Map<String, Value>, domain: &str, suffix: &str) -> Option<f64> {
    number_field(object, &format!("{domain}_{suffix}"))
}

fn domain_string(object: &Map<String, Value>, domain: &str, suffix: &str) -> Option<String> {
    string_field(object, &format!("{domain}_{suffix}"))
}

fn bool_field(object: &Map<String, Value>, field: &str) -> Option<bool> {
    object.get(field).and_then(Value::as_bool)
}

fn number_field(object: &Map<String, Value>, field: &str) -> Option<f64> {
    object.get(field).and_then(Value::as_f64)
}

fn string_field(object: &Map<String, Value>, field: &str) -> Option<String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}
