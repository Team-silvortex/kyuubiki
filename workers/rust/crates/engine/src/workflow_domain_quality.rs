use crate::workflow_metric_resolver::checked_metric_value;
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
pub(crate) struct QualityTerm {
    pub field: &'static str,
    pub label: &'static str,
    pub target: f64,
    pub weight: f64,
    pub goal: QualityGoal,
}

#[derive(Clone, Copy)]
pub(crate) enum QualityGoal {
    Min,
    Max,
}

pub(crate) struct QualityScore {
    pub score_terms: Vec<Value>,
    pub score: f64,
    pub missing_count: usize,
    pub watch_count: usize,
    pub max_ready_score: f64,
    pub grade: &'static str,
    pub dominant_term: Value,
    pub blocking_terms: Vec<Value>,
}

struct ScoredTerm {
    term: QualityTerm,
    value: Option<f64>,
    penalty: f64,
    status: &'static str,
}

type TermLookup = fn(&str) -> Option<QualityTerm>;

pub(crate) fn score_quality_terms(
    object: &Map<String, Value>,
    config: &Value,
    defaults: &[QualityTerm],
    lookup: TermLookup,
    default_ready_score: f64,
) -> Result<QualityScore, String> {
    if !config.is_null() && !config.is_object() {
        return Err("config must be an object or null".into());
    }
    let terms = selected_terms(config, defaults, lookup)?;
    let targets = configured_terms(config, "targets", lookup, true)?;
    let weights = configured_terms(config, "weights", lookup, false)?;
    let max_ready_score = match config.get("max_ready_score") {
        Some(value) => configured_number(value, "config.max_ready_score", false)?,
        None => default_ready_score,
    };
    let mut score = 0.0;
    let mut missing_count = 0;
    let mut watch_count = 0;
    let mut scored = Vec::with_capacity(terms.len());
    for mut term in terms {
        if let Some(target) = targets.get(term.field) {
            term.target = *target;
        }
        if let Some(weight) = weights.get(term.field) {
            term.weight = *weight;
        }
        let evaluated = score_term(object, term)?;
        score += evaluated.penalty;
        if !score.is_finite() {
            return Err(format!(
                "quality_score became non-finite after {}",
                term.field
            ));
        }
        missing_count += usize::from(evaluated.status == "missing");
        watch_count += usize::from(evaluated.status == "watch");
        scored.push(evaluated);
    }
    let grade = quality_grade(score, missing_count, max_ready_score);
    let dominant_term = scored
        .iter()
        .max_by(|left, right| left.penalty.total_cmp(&right.penalty))
        .map(ScoredTerm::compact)
        .unwrap_or(Value::Null);
    let blocking_terms = if grade == "block" {
        scored
            .iter()
            .filter(|term| matches!(term.status, "missing" | "watch"))
            .map(ScoredTerm::compact)
            .collect()
    } else {
        Vec::new()
    };
    Ok(QualityScore {
        score_terms: scored.iter().map(ScoredTerm::to_value).collect(),
        score,
        missing_count,
        watch_count,
        max_ready_score,
        grade,
        dominant_term,
        blocking_terms,
    })
}

fn selected_terms(
    config: &Value,
    defaults: &[QualityTerm],
    lookup: TermLookup,
) -> Result<Vec<QualityTerm>, String> {
    let Some(requested) = config.get("enabled_terms") else {
        return Ok(defaults.to_vec());
    };
    let requested = requested
        .as_array()
        .filter(|terms| !terms.is_empty())
        .ok_or_else(|| "config.enabled_terms must be a non-empty array".to_string())?;
    let mut selected = Vec::with_capacity(defaults.len());
    let mut seen = BTreeSet::new();
    for (index, value) in requested.iter().enumerate() {
        let path = format!("config.enabled_terms[{index}]");
        let field = value
            .as_str()
            .map(str::trim)
            .filter(|field| !field.is_empty())
            .ok_or_else(|| format!("{path} must be a non-empty metric name"))?;
        let term = lookup(field).ok_or_else(|| format!("{path} names unknown metric {field}"))?;
        if !seen.insert(term.field) {
            return Err(format!("{path} repeats metric {field}"));
        }
        selected.push(term);
    }
    Ok(selected)
}

fn configured_terms<'a>(
    config: &'a Value,
    group: &str,
    lookup: TermLookup,
    positive: bool,
) -> Result<BTreeMap<&'a str, f64>, String> {
    let Some(overrides) = config.get(group) else {
        return Ok(BTreeMap::new());
    };
    let overrides = overrides
        .as_object()
        .ok_or_else(|| format!("config.{group} must be an object"))?;
    overrides
        .iter()
        .map(|(field, value)| {
            let path = format!("config.{group}.{field}");
            if lookup(field).is_none() {
                return Err(format!("{path} names an unknown metric"));
            }
            Ok((field.as_str(), configured_number(value, &path, positive)?))
        })
        .collect()
}

fn configured_number(value: &Value, path: &str, positive: bool) -> Result<f64, String> {
    let constraint = if positive { "positive" } else { "nonnegative" };
    value
        .as_f64()
        .filter(|number| {
            number.is_finite()
                && if positive {
                    *number > 0.0
                } else {
                    *number >= 0.0
                }
        })
        .ok_or_else(|| format!("{path} must be a finite {constraint} number"))
}

fn score_term(object: &Map<String, Value>, term: QualityTerm) -> Result<ScoredTerm, String> {
    let value = quality_metric(object, term.field)?;
    let Some(value) = value else {
        return Ok(ScoredTerm {
            term,
            value: None,
            penalty: 0.0,
            status: "missing",
        });
    };
    // Validate the metric even at zero weight, but do not evaluate an unused ratio.
    let penalty = if term.weight == 0.0 {
        0.0
    } else {
        let ratio = match term.goal {
            QualityGoal::Min => value.abs() / term.target,
            QualityGoal::Max => term.target / value.abs().max(1e-12),
        };
        if !ratio.is_finite() {
            return Err(format!("{}.ratio is non-finite", term.field));
        }
        let penalty = ratio * term.weight;
        if !penalty.is_finite() {
            return Err(format!("{}.penalty is non-finite", term.field));
        }
        penalty
    };
    let meets_target = match term.goal {
        QualityGoal::Min => value.abs() <= term.target,
        QualityGoal::Max => value >= term.target,
    };
    Ok(ScoredTerm {
        term,
        value: Some(value),
        penalty,
        status: if meets_target { "ok" } else { "watch" },
    })
}

fn quality_metric(object: &Map<String, Value>, field: &str) -> Result<Option<f64>, String> {
    checked_metric_value(object, field)
        .map_err(|error| format!("payload.{error} (requested metric payload.{field})"))
}

impl ScoredTerm {
    fn compact(&self) -> Value {
        json!({
            "field":self.term.field, "label":self.term.label,
            "status":self.status, "penalty":self.penalty
        })
    }

    fn to_value(&self) -> Value {
        let mut value = self.compact();
        value["target"] = json!(self.term.target);
        value["weight"] = json!(self.term.weight);
        if let Some(metric) = self.value {
            value["value"] = json!(metric);
            value["goal"] = json!(match self.term.goal {
                QualityGoal::Min => "min",
                QualityGoal::Max => "max",
            });
        }
        value
    }
}

fn quality_grade(score: f64, missing_count: usize, max_ready_score: f64) -> &'static str {
    if !score.is_finite() || missing_count > 0 || score > max_ready_score {
        "block"
    } else if score > max_ready_score * 0.7 {
        "review"
    } else if score > max_ready_score * 0.35 {
        "good"
    } else {
        "excellent"
    }
}
