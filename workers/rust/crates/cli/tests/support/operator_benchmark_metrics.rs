use serde_json::{Value, json};

pub fn latency_summary(samples_ms: &[f64], wall_seconds: f64) -> Result<Value, String> {
    if samples_ms.is_empty()
        || !wall_seconds.is_finite()
        || wall_seconds <= 0.0
        || samples_ms.iter().any(|x| !x.is_finite() || *x < 0.0)
    {
        return Err("invalid_benchmark_samples".into());
    }
    let mut sorted = samples_ms.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = |percent: usize| (sorted.len() * percent).div_ceil(100) - 1;
    Ok(json!({"count": sorted.len(), "p50_ms":sorted[rank(50)],
        "p95_ms":sorted[rank(95)], "max_ms":sorted.last().unwrap(),
        "wall_seconds":wall_seconds, "throughput_per_second":sorted.len() as f64/wall_seconds}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentiles_use_nearest_rank_and_throughput_uses_parallel_wall_time() {
        let samples: Vec<_> = (1..=20).rev().map(f64::from).collect();
        let summary = latency_summary(&samples, 0.1).unwrap();
        assert_eq!(summary["p50_ms"], 10.0);
        assert_eq!(summary["p95_ms"], 19.0);
        assert_eq!(summary["throughput_per_second"], 200.0);
        assert_eq!(latency_summary(&[3.0], 1.0).unwrap()["p95_ms"], 3.0);
    }

    #[test]
    fn invalid_samples_never_publish_a_successful_benchmark() {
        for samples in [vec![], vec![f64::NAN], vec![f64::INFINITY], vec![-1.0]] {
            assert!(latency_summary(&samples, 1.0).is_err());
        }
        for wall in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            assert!(latency_summary(&[1.0], wall).is_err());
        }
    }
}
