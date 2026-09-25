use crate::workflow_diagnostic_samples::{Field, Samples, configured_name, insert_finite};
use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value, json};
use std::collections::BTreeSet;

struct FieldInput<'a> {
    samples: Samples<'a>,
    source: &'a str,
    field: Field<'a>,
    prefix: &'a str,
}

impl<'a> FieldInput<'a> {
    fn scan(
        &self,
        observe: impl FnMut(&'a Value, f64) -> Result<(), String>,
    ) -> Result<(), String> {
        self.samples.scan(
            self.field.label(),
            true,
            |row, source, index| self.field.read(row, source, index),
            observe,
        )?;
        Ok(())
    }

    fn metadata(&self, summary: &mut Map<String, Value>) {
        summary.insert("source_collection".into(), self.source.into());
        summary.insert("source_field".into(), self.field.label().into());
    }
}

fn extract_checked(
    payload: Value,
    config: Value,
    operator: &str,
    default_source: &str,
    extract: impl FnOnce(FieldInput<'_>, &Value) -> Result<Value, String>,
) -> Result<Value, String> {
    let run = || {
        let object = payload.as_object().ok_or("expects an object payload")?;
        require_converged_result(object, operator, "payload")?;
        if !config.is_null() && !config.is_object() {
            return Err("config must be an object or null".into());
        }
        let source = configured_name(&config, "source")?.unwrap_or(default_source);
        let field = configured_name(&config, "field")?.ok_or("requires config.field")?;
        let prefix = configured_name(&config, "output_prefix")?.unwrap_or(field);
        let input = FieldInput {
            samples: Samples::fixed(object, source)?,
            source,
            field: Field::aliases(&[field]),
            prefix,
        };
        extract(input, &config)
    };
    run().map_err(|error| format!("{operator}: {error}"))
}

pub fn extract_field_statistics(payload: Value, config: Value) -> Result<Value, String> {
    extract_checked(
        payload,
        config,
        "extract.field_statistics",
        "nodes",
        |input, config| {
            let percentiles = requested_percentiles(config)?;
            let mut values = Vec::new();
            let (mut sum, mut min, mut max) = (0.0_f64, f64::INFINITY, f64::NEG_INFINITY);
            input.scan(|_, value| {
                sum += value;
                if !sum.is_finite() {
                    return Err(format!("{}_sum is non-finite", input.prefix));
                }
                min = min.min(value);
                max = max.max(value);
                if !percentiles.is_empty() {
                    values.push(value);
                }
                Ok(())
            })?;
            let count = input.samples.len();
            let mean = sum / count as f64;
            let root_count = (count as f64).sqrt();
            let mut stddev = 0.0_f64;
            input.scan(|_, value| {
                let delta = value - mean;
                // Scale before subtraction only when the unscaled deviation overflows.
                let scaled = if delta.is_finite() {
                    delta / root_count
                } else {
                    value / root_count - mean / root_count
                };
                stddev = stddev.hypot(scaled);
                Ok(())
            })?;
            let mut summary = Map::new();
            summary.insert(format!("{}_count", input.prefix), count.into());
            for (suffix, value) in [
                ("min", min),
                ("max", max),
                ("sum", sum),
                ("mean", mean),
                ("stddev", stddev),
            ] {
                insert_finite(&mut summary, format!("{}_{suffix}", input.prefix), value)?;
            }
            if !percentiles.is_empty() {
                values.sort_unstable_by(f64::total_cmp);
                for percentile in percentiles {
                    insert_finite(
                        &mut summary,
                        format!("{}_{}", input.prefix, percentile_key(percentile)),
                        interpolate_percentile(&values, percentile),
                    )?;
                }
            }
            input.metadata(&mut summary);
            Ok(Value::Object(summary))
        },
    )
}

pub fn extract_field_hotspots(payload: Value, config: Value) -> Result<Value, String> {
    extract_checked(
        payload,
        config,
        "extract.field_hotspots",
        "elements",
        |input, config| {
            let threshold = optional_number(config, "threshold")?;
            let percentile = optional_number(config, "percentile")?.unwrap_or(90.0);
            validate_percentile(percentile, "config.percentile")?;
            let sample_limit = match config.get("sample_limit") {
                None => 8,
                Some(value) => value
                    .as_u64()
                    .ok_or("config.sample_limit must be a nonnegative integer")?
                    .min(32) as usize,
            };
            let sample_sort = configured_name(config, "sample_sort")?.unwrap_or("value_desc");
            if !["value_desc", "value_asc"].contains(&sample_sort) {
                return Err("config.sample_sort must be value_desc or value_asc".into());
            }
            let threshold = if let Some(value) = threshold {
                value
            } else {
                let mut values = Vec::new();
                input.scan(|_, value| {
                    values.push(value);
                    Ok(())
                })?;
                values.sort_unstable_by(f64::total_cmp);
                let value = interpolate_percentile(&values, percentile);
                if !value.is_finite() {
                    return Err("hotspot threshold is non-finite".into());
                }
                value
            };
            let mut hotspots = Vec::new();
            input.scan(|entry, value| {
                if value >= threshold {
                    hotspots.push((value, entry));
                }
                Ok(())
            })?;
            if hotspots.is_empty() {
                return Err(format!(
                    "did not find any values meeting the threshold for {}.{}",
                    input.source,
                    input.field.label()
                ));
            }
            hotspots.sort_by(|left, right| {
                if sample_sort == "value_asc" {
                    left.0.total_cmp(&right.0)
                } else {
                    right.0.total_cmp(&left.0)
                }
            });
            let mut mean = 0.0_f64;
            for (index, (value, _)) in hotspots.iter().enumerate() {
                if index == 0 {
                    mean = *value;
                    continue;
                }
                let count = (index + 1) as f64;
                let delta = *value - mean;
                mean = if delta.is_finite() {
                    mean + delta / count
                } else {
                    mean * ((count - 1.0) / count) + *value / count
                };
            }
            let prefix = input.prefix;
            let mut summary = Map::new();
            for (suffix, value) in [
                ("threshold", threshold),
                ("hotspot_mean", mean),
                (
                    "hotspot_max",
                    hotspots
                        .iter()
                        .map(|(v, _)| *v)
                        .fold(f64::NEG_INFINITY, f64::max),
                ),
                (
                    "hotspot_fraction",
                    hotspots.len() as f64 / input.samples.len() as f64,
                ),
            ] {
                insert_finite(&mut summary, format!("{prefix}_{suffix}"), value)?;
            }
            summary.insert(format!("{prefix}_hotspot_count"), hotspots.len().into());
            summary.insert(format!("{prefix}_sample_sort"), sample_sort.into());
            summary.insert(
                format!("{prefix}_hotspot_ids"),
                json!(
                    hotspots
                        .iter()
                        .filter_map(|(_, item)| item.get("id"))
                        .collect::<Vec<_>>()
                ),
            );
            // Only the bounded published samples clone full records; sorting uses borrowed records.
            summary.insert(
                format!("{prefix}_hotspot_samples"),
                json!(
                    hotspots
                        .iter()
                        .take(sample_limit)
                        .map(|(_, item)| *item)
                        .collect::<Vec<_>>()
                ),
            );
            input.metadata(&mut summary);
            Ok(Value::Object(summary))
        },
    )
}

fn optional_number(config: &Value, key: &str) -> Result<Option<f64>, String> {
    config
        .get(key)
        .map(|value| {
            value
                .as_f64()
                .filter(|v| v.is_finite())
                .ok_or_else(|| format!("config.{key} must be a finite number"))
        })
        .transpose()
}

fn validate_percentile(value: f64, path: &str) -> Result<(), String> {
    if !value.is_finite() || !(0.0..=100.0).contains(&value) {
        return Err(format!("{path} must be a finite number between 0 and 100"));
    }
    Ok(())
}

fn requested_percentiles(config: &Value) -> Result<Vec<f64>, String> {
    let Some(entries) = config.get("percentiles") else {
        return Ok(Vec::new());
    };
    let entries = entries
        .as_array()
        .ok_or("config.percentiles must be an array")?;
    let mut seen = BTreeSet::new();
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| {
            let path = format!("config.percentiles[{index}]");
            let value = entry
                .as_f64()
                .ok_or_else(|| format!("{path} must be a finite number"))?;
            validate_percentile(value, &path)?;
            let value = if value == 0.0 { 0.0 } else { value };
            if !seen.insert(percentile_key(value)) {
                return Err(format!("{path} duplicates a percentile output"));
            }
            Ok(value)
        })
        .collect()
}

fn percentile_key(percentile: f64) -> String {
    format!("p{}", percentile.to_string().replace('.', "_"))
}

fn interpolate_percentile(sorted: &[f64], percentile: f64) -> f64 {
    let position = (percentile / 100.0) * (sorted.len() - 1) as f64;
    let low = sorted[position.floor() as usize];
    let high = sorted[position.ceil() as usize];
    let weight = position.fract();
    if low == high {
        low
    } else if low.is_sign_negative() == high.is_sign_negative() {
        low + (high - low) * weight
    } else {
        low * (1.0 - weight) + high * weight
    }
}
