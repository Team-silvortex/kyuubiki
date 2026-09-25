use crate::workflow_result_admission::require_converged_result;
use serde_json::{Map, Value, json};

pub(crate) struct Field<'a> {
    names: Vec<&'a str>,
    required: bool,
}

impl<'a> Field<'a> {
    pub(crate) fn aliases(names: &[&'a str]) -> Self {
        Self {
            names: names.to_vec(),
            required: false,
        }
    }

    pub(crate) fn configured(
        config: &'a Value,
        key: &str,
        defaults: &[&'a str],
    ) -> Result<Self, String> {
        let selected = configured_name(config, key)?;
        Ok(Self {
            names: selected.map_or_else(|| defaults.to_vec(), |name| vec![name]),
            required: selected.is_some(),
        })
    }

    pub(crate) fn label(&self) -> &str {
        self.names.first().copied().unwrap_or("metric")
    }

    pub(crate) fn read(
        &self,
        row: &Map<String, Value>,
        source: &str,
        index: usize,
    ) -> Result<Option<f64>, String> {
        for name in &self.names {
            if let Some(value) = row.get(*name) {
                // A present invalid canonical value must not fall through to another alias.
                return value
                    .as_f64()
                    .filter(|value| value.is_finite())
                    .map(Some)
                    .ok_or_else(|| {
                        format!("payload.{source}[{index}].{name} must be a finite number")
                    });
            }
        }
        if self.required {
            return Err(missing_sample(source, index, self.label()));
        }
        Ok(None)
    }
}

pub(crate) struct VectorFields<'a> {
    pub(crate) x: Field<'a>,
    pub(crate) y: Field<'a>,
    pub(crate) z: Field<'a>,
    pub(crate) magnitude: Field<'a>,
}

struct VectorSample {
    magnitude: f64,
    components: [Option<f64>; 3],
}

impl VectorFields<'_> {
    fn read(
        &self,
        row: &Map<String, Value>,
        source: &str,
        index: usize,
    ) -> Result<Option<VectorSample>, String> {
        let magnitude = self.magnitude.read(row, source, index)?;
        let components = [
            self.x.read(row, source, index)?,
            self.y.read(row, source, index)?,
            self.z.read(row, source, index)?,
        ];
        if magnitude.is_none() && components.iter().all(Option::is_none) {
            return Ok(None);
        }
        let magnitude = if let Some(magnitude) = magnitude {
            if magnitude < 0.0 {
                return Err(format!(
                    "payload.{source}[{index}].{} must be nonnegative",
                    self.magnitude
                        .names
                        .iter()
                        .find(|name| row.contains_key(**name))
                        .copied()
                        .unwrap_or(self.magnitude.label())
                ));
            }
            magnitude
        } else {
            let x = components[0].ok_or_else(|| missing_sample(source, index, self.x.label()))?;
            let y = components[1].ok_or_else(|| missing_sample(source, index, self.y.label()))?;
            let norm = x.hypot(y);
            let norm = components[2].map_or(norm, |z| norm.hypot(z));
            if !norm.is_finite() {
                return Err(format!(
                    "payload.{source}[{index}] vector magnitude is non-finite"
                ));
            }
            norm
        };
        Ok(Some(VectorSample {
            magnitude,
            components,
        }))
    }

    fn required(&self) -> bool {
        self.x.required || self.y.required || self.z.required || self.magnitude.required
    }
}

pub(crate) struct Samples<'a> {
    source: &'a str,
    entries: &'a [Value],
}

impl<'a> Samples<'a> {
    fn new(
        object: &'a Map<String, Value>,
        config: &'a Value,
        key: &str,
        default: &'a str,
    ) -> Result<Self, String> {
        let source = configured_name(config, key)?.unwrap_or(default);
        Self::fixed(object, source)
    }

    pub(crate) fn fixed(object: &'a Map<String, Value>, source: &'a str) -> Result<Self, String> {
        let entries = object
            .get(source)
            .and_then(Value::as_array)
            .ok_or_else(|| format!("expected array payload.{source}"))?;
        Ok(Self { source, entries })
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn scan<T>(
        &self,
        label: &str,
        required: bool,
        mut read: impl FnMut(&Map<String, Value>, &str, usize) -> Result<Option<T>, String>,
        mut observe: impl FnMut(&'a Value, T) -> Result<(), String>,
    ) -> Result<bool, String> {
        let mut count = 0_usize;
        let mut first_missing = None;
        for (index, entry) in self.entries.iter().enumerate() {
            let row = entry
                .as_object()
                .ok_or_else(|| format!("payload.{}[{index}] must be an object", self.source))?;
            if let Some(value) = read(row, self.source, index)? {
                observe(entry, value)?;
                count += 1;
            } else {
                first_missing.get_or_insert(index);
            }
        }
        // Entirely absent optional groups stay absent, but partial groups cannot claim completeness.
        if count > 0 || required {
            if let Some(index) = first_missing {
                return Err(missing_sample(self.source, index, label));
            }
            if count == 0 {
                return Err(format!(
                    "payload.{} has no samples for {label}",
                    self.source
                ));
            }
        }
        Ok(count > 0)
    }
}

pub(crate) struct Diagnostics<'a> {
    object: &'a Map<String, Value>,
    nodes: Samples<'a>,
    elements: Samples<'a>,
    prefix: String,
    summary: Map<String, Value>,
    measured_groups: usize,
    missing_scalar_id: Value,
}

pub(crate) fn extract_checked(
    payload: Value,
    config: Value,
    domain: &str,
    groups: &[&str],
    populate: impl FnOnce(&mut Diagnostics<'_>, &Value) -> Result<(), String>,
) -> Result<Value, String> {
    let operator = format!("extract.{domain}_result_diagnostics");
    let extract = || {
        let object = payload.as_object().ok_or("expects an object payload")?;
        require_converged_result(object, &operator, "payload")?;
        if !config.is_null() && !config.is_object() {
            return Err("config must be an object or null".to_string());
        }
        let prefix = configured_name(&config, "output_prefix")?
            .unwrap_or(domain)
            .chars()
            .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
            .collect::<String>()
            .trim_matches('_')
            .to_string();
        if prefix.is_empty() {
            return Err("config.output_prefix must contain an ASCII letter or digit".to_string());
        }
        let nodes = Samples::new(object, &config, "node_source", "nodes")?;
        let elements = Samples::new(object, &config, "element_source", "elements")?;
        let mut summary = Map::new();
        summary.insert(
            "diagnostic_contract".into(),
            json!("kyuubiki.workflow_diagnostics/v1"),
        );
        summary.insert("diagnostic_domain".into(), json!(domain));
        summary.insert(
            "diagnostic_subject".into(),
            json!(format!("{domain}_result")),
        );
        summary.insert("diagnostic_prefix".into(), json!(prefix));
        summary.insert("diagnostic_metric_groups".into(), json!(groups));
        for (label, count) in [
            ("node", nodes.entries.len()),
            ("element", elements.entries.len()),
        ] {
            summary.insert(format!("{prefix}_{label}_count"), count.into());
            summary.insert(format!("diagnostic_{label}_count"), count.into());
        }
        let mut diagnostics = Diagnostics {
            object,
            nodes,
            elements,
            prefix,
            summary,
            measured_groups: 0,
            missing_scalar_id: Value::Null,
        };
        populate(&mut diagnostics, &config)?;
        if diagnostics.measured_groups == 0 {
            return Err("did not find any diagnostic fields".to_string());
        }
        Ok(Value::Object(diagnostics.summary))
    };
    extract().map_err(|error| format!("{operator}: {error}"))
}

impl Diagnostics<'_> {
    pub(crate) fn scalar_id_fallback(&mut self, fallback: Value) {
        self.missing_scalar_id = fallback;
    }

    pub(crate) fn node_distribution(
        &mut self,
        label: &str,
        field: Field<'_>,
    ) -> Result<(), String> {
        let name = format!("{}_{label}", self.prefix);
        let mut count = 0_usize;
        let mut sum: f64 = 0.0;
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let present = self.nodes.scan(
            field.label(),
            field.required,
            |row, source, index| field.read(row, source, index),
            |_, value| {
                count += 1;
                sum += value;
                if !sum.is_finite() {
                    return Err(format!("{name}_sum became non-finite"));
                }
                min = min.min(value);
                max = max.max(value);
                Ok(())
            },
        )?;
        if present {
            self.summary.insert(format!("{name}_count"), count.into());
            for (suffix, value) in [
                ("min", min),
                ("max", max),
                ("sum", sum),
                ("mean", sum / count as f64),
                ("span", max - min),
            ] {
                insert_finite(&mut self.summary, format!("{name}_{suffix}"), value)?;
            }
            self.measured_groups += 1;
        }
        Ok(())
    }

    pub(crate) fn element_scalar_peak(
        &mut self,
        label: &str,
        field: Field<'_>,
    ) -> Result<bool, String> {
        let mut peak: Option<(&Value, f64)> = None;
        self.elements.scan(
            field.label(),
            field.required,
            |row, source, index| field.read(row, source, index),
            |entry, value| {
                if peak.is_none_or(|(_, best)| !value.total_cmp(&best).is_lt()) {
                    peak = Some((entry, value));
                }
                Ok(())
            },
        )?;
        if let Some((entry, value)) = peak {
            self.scalar_peak(label, entry, value)?;
        }
        Ok(peak.is_some())
    }

    pub(crate) fn element_component_peak(
        &mut self,
        label: &str,
        names: &[&str],
    ) -> Result<(), String> {
        let mut peak: Option<(&Value, f64)> = None;
        // Components are signed extrema, not Euclidean vector magnitudes.
        for name in names {
            let field = Field::aliases(&[name]);
            self.elements.scan(
                name,
                false,
                |row, source, index| field.read(row, source, index),
                |entry, value| {
                    if peak.is_none_or(|(_, best)| !value.abs().total_cmp(&best.abs()).is_lt()) {
                        peak = Some((entry, value));
                    }
                    Ok(())
                },
            )?;
        }
        if let Some((entry, value)) = peak {
            self.scalar_peak(label, entry, value)?;
        }
        Ok(())
    }

    pub(crate) fn payload_scalar_peak(&mut self, label: &str, field: &str) -> Result<bool, String> {
        let Some(value) = self.object.get(field) else {
            return Ok(false);
        };
        let value = value
            .as_f64()
            .filter(|value| value.is_finite())
            .ok_or_else(|| format!("payload.{field} must be a finite number"))?;
        self.scalar_peak(label, &json!({"id":field}), value)?;
        Ok(true)
    }

    fn scalar_peak(&mut self, label: &str, entry: &Value, value: f64) -> Result<(), String> {
        insert_finite(
            &mut self.summary,
            format!("{}_{label}_peak", self.prefix),
            value,
        )?;
        self.peak_identity(label, entry, value)?;
        if entry.get("id").is_none() {
            self.summary.insert(
                format!("{}_peak_{label}_id", self.prefix),
                self.missing_scalar_id.clone(),
            );
        }
        self.measured_groups += 1;
        Ok(())
    }

    pub(crate) fn element_vector_peak(
        &mut self,
        label: &str,
        fields: VectorFields<'_>,
    ) -> Result<(), String> {
        self.vector_peak(label, fields, false)
    }

    pub(crate) fn node_vector_peak(
        &mut self,
        label: &str,
        fields: VectorFields<'_>,
    ) -> Result<(), String> {
        self.vector_peak(label, fields, true)
    }

    fn vector_peak(
        &mut self,
        label: &str,
        fields: VectorFields<'_>,
        nodes: bool,
    ) -> Result<(), String> {
        let mut peak: Option<(&Value, VectorSample)> = None;
        let samples = if nodes { &self.nodes } else { &self.elements };
        samples.scan(
            label,
            fields.required(),
            |row, source, index| fields.read(row, source, index),
            |entry, value| {
                if peak
                    .as_ref()
                    .is_none_or(|(_, best)| !value.magnitude.total_cmp(&best.magnitude).is_lt())
                {
                    peak = Some((entry, value));
                }
                Ok(())
            },
        )?;
        if let Some((entry, value)) = peak {
            insert_finite(
                &mut self.summary,
                format!("{}_{label}_peak_magnitude", self.prefix),
                value.magnitude,
            )?;
            self.peak_identity(label, entry, value.magnitude)?;
            for (axis, component) in ["x", "y", "z"].into_iter().zip(value.components) {
                if let Some(component) = component {
                    insert_finite(
                        &mut self.summary,
                        format!("{}_{label}_peak_{axis}", self.prefix),
                        component,
                    )?;
                }
            }
            self.measured_groups += 1;
        }
        Ok(())
    }

    fn peak_identity(&mut self, label: &str, entry: &Value, value: f64) -> Result<(), String> {
        insert_finite(
            &mut self.summary,
            format!("{}_peak_{label}", self.prefix),
            value,
        )?;
        let id = entry.get("id").cloned().unwrap_or(Value::Null);
        self.summary
            .insert(format!("{}_peak_{label}_id", self.prefix), id.clone());
        self.summary
            .insert(format!("{}_{label}_peak_element_id", self.prefix), id);
        Ok(())
    }
}

pub(crate) fn configured_name<'a>(config: &'a Value, key: &str) -> Result<Option<&'a str>, String> {
    config
        .get(key)
        .map(|value| {
            value
                .as_str()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| format!("config.{key} must be a non-empty string"))
        })
        .transpose()
}

pub(crate) fn missing_sample(source: &str, index: usize, field: &str) -> String {
    format!("payload.{source}[{index}].{field} is missing a numeric sample")
}

pub(crate) fn insert_finite(
    summary: &mut Map<String, Value>,
    field: String,
    value: f64,
) -> Result<(), String> {
    if !value.is_finite() {
        return Err(format!("{field} is non-finite"));
    }
    summary.insert(field, value.into());
    Ok(())
}
