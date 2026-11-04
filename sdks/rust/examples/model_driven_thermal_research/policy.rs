use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Design {
    pub left: f64,
    pub right: f64,
    pub refinement: usize,
}

pub const GRID: [f64; 4] = [2.0, 10.0, 100.0, 200.0];
pub const TIP_LIMIT: f64 = 110e-6;
pub const TIP_TOLERANCE: f64 = 1e-9;

pub fn tip_feasibility(tip: f64) -> Result<&'static str, String> {
    if !tip.is_finite() {
        return Err("nonfinite design metric".into());
    }
    Ok(if tip < TIP_LIMIT - TIP_TOLERANCE {
        "feasible"
    } else if tip > TIP_LIMIT + TIP_TOLERANCE {
        "infeasible"
    } else {
        "boundary"
    })
}
pub const MAX_CALLS: usize = 16;
pub const MAX_JOBS: usize = 40;
pub const MAX_INPUT_BYTES: usize = 128 * 1024;
pub const MAX_OUTPUT_TOKENS: u64 = 8192;
pub const BUDGET_USD: f64 = 8.0;

pub fn designs(arguments: &str) -> Result<Vec<Design>, String> {
    #[derive(Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Batch {
        designs: Vec<Design>,
    }
    if arguments.len() > 4096 {
        return Err("tool arguments too large".into());
    }
    let batch: Batch = serde_json::from_str(arguments).map_err(|_| "invalid tool arguments")?;
    if !(1..=4).contains(&batch.designs.len()) {
        return Err("batch must contain 1..4 designs".into());
    }
    for d in &batch.designs {
        if !GRID.contains(&d.left)
            || !GRID.contains(&d.right)
            || ![1, 2, 4, 8].contains(&d.refinement)
        {
            return Err("design outside approved conductivity/refinement grid".into());
        }
    }
    Ok(batch.designs)
}

pub fn estimated_cost(model: &str, input: u64, output: u64) -> Result<f64, String> {
    let (i, o) = match model {
        "gpt-6.1-sol" => (2.0, 10.0),
        "gpt-6-astra" => (10.0, 50.0),
        _ => return Err("unapproved model".into()),
    };
    Ok((input as f64 * i + output as f64 * o) / 1_000_000.0)
}

pub fn completed_output(response: &Value) -> Result<Vec<Value>, String> {
    if response["status"] != "completed" {
        return Err("model response not completed".into());
    }
    let output = response["output"]
        .as_array()
        .ok_or("missing model output")?;
    if output.is_empty() {
        return Err("empty model output".into());
    }
    for item in output {
        if let Some(content) = item["content"].as_array() {
            if content.iter().any(|part| part["type"] == "refusal") {
                return Err("model declined the request".into());
            }
        }
    }
    Ok(output.clone())
}

pub fn output_text(output: &[Value]) -> Result<String, String> {
    let text: String = output
        .iter()
        .filter(|v| v["type"] == "message")
        .filter_map(|v| v["content"].as_array())
        .flatten()
        .filter(|v| v["type"] == "output_text")
        .filter_map(|v| v["text"].as_str())
        .collect();
    if text.is_empty() {
        return Err("missing final model text".into());
    }
    Ok(text)
}

pub struct Budget {
    pub charged: f64,
    pub calls: usize,
}

impl Budget {
    pub fn reserve(&mut self, model: &str, bytes: usize) -> Result<f64, String> {
        if bytes > MAX_INPUT_BYTES || self.calls >= MAX_CALLS {
            return Err("model input/call limit reached".into());
        }
        // Overestimate tokens using UTF-8 bytes plus framing/tool overhead.
        let cost = estimated_cost(model, bytes as u64 + 8192, MAX_OUTPUT_TOKENS)?;
        if self.charged + cost > BUDGET_USD {
            return Err("research cost ceiling reached".into());
        }
        self.charged += cost;
        self.calls += 1;
        Ok(cost)
    }

    pub fn settle(
        &mut self,
        reserved: f64,
        model: &str,
        input: u64,
        output: u64,
    ) -> Result<f64, String> {
        let actual = estimated_cost(model, input, output)?;
        if actual > reserved || output > MAX_OUTPUT_TOKENS {
            return Err("usage exceeded reservation; stop without retry".into());
        }
        self.charged += actual - reserved;
        Ok(actual)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn feasibility_near_the_limit_is_not_a_rounding_coin_flip() {
        assert_eq!(tip_feasibility(100e-6).unwrap(), "feasible");
        assert_eq!(tip_feasibility(120e-6).unwrap(), "infeasible");
        for tip in [TIP_LIMIT, TIP_LIMIT + 2e-17, TIP_LIMIT - 2e-17] {
            assert_eq!(tip_feasibility(tip).unwrap(), "boundary");
        }
        assert!(tip_feasibility(f64::NAN).is_err());
        assert!(tip_feasibility(f64::INFINITY).is_err());
    }

    #[test]
    fn accepts_only_explicit_finite_grid_designs() {
        let value = r#"{"designs":[{"left":100,"right":200,"refinement":2}]}"#;
        assert_eq!(designs(value).unwrap()[0].left, 100.0);
        for invalid in [
            r#"{"designs":[]}"#,
            r#"{"designs":[{"left":0,"right":200,"refinement":2}]}"#,
            r#"{"designs":[{"left":100,"right":200,"refinement":10000}]}"#,
            r#"{"designs":[{"left":100,"right":200,"refinement":2,"command":"exec"}]}"#,
            r#"{"designs":[{"left":1e999,"right":200,"refinement":2}]}"#,
            r#"{"designs":[],"approve":true}"#,
        ] {
            assert!(designs(invalid).is_err(), "accepted {invalid}");
        }
    }

    #[test]
    fn rejects_entire_invalid_or_oversized_batch() {
        let valid = json!({"left":2,"right":10,"refinement":1});
        assert!(designs(&json!({"designs": vec![valid.clone(); 5]}).to_string()).is_err());
        assert!(
            designs(
                &json!({"designs": [valid, {"left":-1,"right":10,"refinement":1}]}).to_string()
            )
            .is_err()
        );
    }

    #[test]
    fn prices_are_bounded_to_explicit_models() {
        assert!((estimated_cost("gpt-6.1-sol", 1000, 1000).unwrap() - 0.012).abs() < 1e-12);
        assert!((estimated_cost("gpt-6-astra", 1000, 1000).unwrap() - 0.060).abs() < 1e-12);
        assert!(estimated_cost("automatic-model", 1000, 1000).is_err());
    }

    #[test]
    fn refuses_partial_or_refused_tool_responses() {
        for response in [
            json!({"status":"incomplete","output":[]}),
            json!({"status":"completed","output":[{"type":"message","content":[{"type":"refusal","refusal":"no"}]}]}),
            json!({"status":"completed"}),
        ] {
            assert!(completed_output(&response).is_err());
        }
        assert!(completed_output(&json!({"status":"completed","output":[{"type":"message","content":[{"type":"output_text","text":"ok"}]}]})).is_ok());
    }

    #[test]
    fn reserves_before_send_and_keeps_unknown_costs() {
        let mut b = Budget {
            charged: 0.0,
            calls: 0,
        };
        let reserved = b.reserve("gpt-6-astra", 1000).unwrap();
        assert_eq!(b.charged, reserved);
        assert!(b.settle(reserved, "gpt-6-astra", 1000000, 1).is_err());
        assert_eq!(b.charged, reserved);
        let actual = b.settle(reserved, "gpt-6-astra", 1000, 1000).unwrap();
        assert!((b.charged - actual).abs() < 1e-12);
    }

    #[test]
    fn all_budget_limits_fail_before_reservation() {
        for (charged, calls, bytes) in [
            (BUDGET_USD, 0, 1),
            (0.0, MAX_CALLS, 1),
            (0.0, 0, MAX_INPUT_BYTES + 1),
        ] {
            let mut b = Budget { charged, calls };
            assert!(b.reserve("gpt-6.1-sol", bytes).is_err());
            assert_eq!((b.charged, b.calls), (charged, calls));
        }
    }
}
