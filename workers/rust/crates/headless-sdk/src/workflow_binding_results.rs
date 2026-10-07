use crate::HeadlessExecutionBatch;
use crate::report_compaction::{compact_owned_report_value, compact_report_value};
use crate::workflow_bindings::parse_binding;
use serde_json::{Map, Value};
use std::collections::HashMap;

#[derive(Default)]
pub(crate) struct BindingResults {
    remaining: HashMap<usize, HashMap<String, usize>>,
    values: HashMap<usize, Map<String, Value>>,
}

impl BindingResults {
    pub(crate) fn new(batch: &HeadlessExecutionBatch) -> Self {
        let mut store = Self::default();
        for step in &batch.steps {
            store.collect(&step.payload);
        }
        store
    }

    fn collect(&mut self, value: &Value) {
        match value {
            Value::String(text) => {
                if let Some((step, output)) = parse_binding(text) {
                    *self
                        .remaining
                        .entry(step)
                        .or_default()
                        .entry(output.into())
                        .or_default() += 1;
                }
            }
            Value::Array(items) => items.iter().for_each(|item| self.collect(item)),
            Value::Object(fields) => fields.values().for_each(|value| self.collect(value)),
            _ => {}
        }
    }

    // Keep original referenced outputs in the cache; move only unused fields into the report.
    pub(crate) fn insert(&mut self, index: usize, result: Value) -> Value {
        let Some(required) = self.remaining.get(&index) else {
            return compact_owned_report_value(result);
        };
        let Value::Object(mut fields) = result else {
            return compact_owned_report_value(result);
        };
        let mut selected = Map::new();
        for (key, value) in &mut fields {
            if required.contains_key(key) {
                let preview = compact_report_value(value);
                selected.insert(key.clone(), std::mem::replace(value, preview));
            } else {
                *value = compact_owned_report_value(std::mem::take(value));
            }
        }
        if !selected.is_empty() {
            self.values.insert(index, selected);
        }
        Value::Object(fields)
    }

    // Local preparation/dry-run previews also belong to the report; clone only
    // their referenced outputs, not the entire preview including unrelated data.
    pub(crate) fn insert_preview(&mut self, index: usize, result: &Value) {
        let Some(required) = self.remaining.get(&index) else {
            return;
        };
        if let Some(fields) = result.as_object() {
            let selected: Map<_, _> = fields
                .iter()
                .filter(|(key, _)| required.contains_key(*key))
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect();
            if !selected.is_empty() {
                self.values.insert(index, selected);
            }
        }
    }

    pub(crate) fn take(&mut self, step: usize, output: &str) -> Result<Value, String> {
        let missing = || format!("binding source step {step} has no output {output}");
        let required = self.remaining.get_mut(&step).ok_or_else(missing)?;
        let uses = required.get_mut(output).ok_or_else(missing)?;
        let fields = self.values.get_mut(&step).ok_or_else(missing)?;
        let value = if *uses == 1 {
            fields.remove(output)
        } else {
            fields.get(output).cloned()
        }
        .ok_or_else(missing)?;
        *uses -= 1;
        if *uses == 0 {
            required.remove(output);
        }
        if required.is_empty() {
            self.remaining.remove(&step);
        }
        if fields.is_empty() {
            self.values.remove(&step);
        }
        Ok(value)
    }
}

#[cfg(test)]
#[path = "workflow_binding_results_tests.rs"]
mod tests;
