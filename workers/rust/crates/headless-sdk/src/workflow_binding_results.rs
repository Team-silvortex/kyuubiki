use crate::HeadlessExecutionBatch;
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

    // Reports are compacted before consuming the result. Keep only actual
    // whole-value outputs referenced by the unchanged binding grammar.
    pub(crate) fn insert(&mut self, index: usize, result: Value) {
        let Some(required) = self.remaining.get(&index) else {
            return;
        };
        if let Value::Object(fields) = result {
            let selected: Map<_, _> = fields
                .into_iter()
                .filter(|(key, _)| required.contains_key(key))
                .collect();
            if !selected.is_empty() {
                self.values.insert(index, selected);
            }
        }
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
