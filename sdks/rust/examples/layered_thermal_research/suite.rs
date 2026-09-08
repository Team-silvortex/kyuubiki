use super::{checks, model, patch};
use serde_json::Value;

#[derive(Clone, Copy)]
pub enum Case {
    Layered(model::Case),
    Patch(patch::Case),
}

impl Case {
    pub fn id(self) -> String {
        match self {
            Self::Layered(case) => case.id(),
            Self::Patch(case) => case.id(),
        }
    }
    pub fn workflow(self) -> (Value, Value) {
        match self {
            Self::Layered(case) => case.workflow(),
            Self::Patch(case) => case.workflow(),
        }
    }
    pub fn validate(self, result: &Value) -> Result<Value, String> {
        match self {
            Self::Layered(case) => checks::validate(case, result),
            Self::Patch(case) => case.validate(result),
        }
    }
}

pub fn cases(name: &str) -> Result<Vec<Case>, &'static str> {
    match name {
        "layered" => Ok(model::cases().into_iter().map(Case::Layered).collect()),
        "thermal-patch" => Ok(patch::cases().into_iter().map(Case::Patch).collect()),
        "all" => Ok(model::cases()
            .into_iter()
            .map(Case::Layered)
            .chain(patch::cases().into_iter().map(Case::Patch))
            .collect()),
        _ => Err("unknown research suite; expected layered, thermal-patch, or all"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kyuubiki_headless_sdk::WorkflowGraphDefinition;
    use std::collections::HashSet;

    #[test]
    fn every_suite_has_unique_cases_and_valid_public_graph_contracts() {
        for (name, count) in [("layered", 14), ("thermal-patch", 288), ("all", 302)] {
            let cases = cases(name).unwrap();
            assert_eq!(cases.len(), count);
            let mut seen = HashSet::new();
            for case in cases {
                assert!(seen.insert(case.id()), "duplicate research case");
                let graph: WorkflowGraphDefinition =
                    serde_json::from_value(case.workflow().0).unwrap();
                graph.validate().unwrap();
            }
        }
        assert!(cases("typo").is_err());
    }
}
