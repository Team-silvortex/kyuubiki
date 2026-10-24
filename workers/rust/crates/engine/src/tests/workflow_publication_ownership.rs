use super::for_each_output_value;
use kyuubiki_protocol::WorkflowPort;
use serde_json::{Value, json};
use std::collections::HashSet;

#[test]
fn workflow_publication_ownership_moves_last_port_and_clones_only_independent_siblings() {
    for count in [0, 1, 2, 32] {
        let ports: Vec<WorkflowPort> = (0..count)
            .map(|index| {
                serde_json::from_value(json!({
                    "id":format!("port-{index}"),"artifact_type":"artifact/json"
                }))
                .unwrap()
            })
            .collect();
        let original = json!({"text":"x".repeat(128*1024),"items":["y".repeat(64*1024)]});
        let expected = original.clone();
        let pointers = |value: &Value| {
            (
                value["text"].as_str().unwrap().as_ptr() as usize,
                value["items"].as_array().unwrap().as_ptr() as usize,
                value["items"][0].as_str().unwrap().as_ptr() as usize,
            )
        };
        let original_pointers = pointers(&original);
        let mut published = Vec::new();
        for_each_output_value(&ports, original, |port, value| {
            published.push((port.id.clone(), value));
        });
        assert_eq!(published.len(), count);
        let mut buffers = HashSet::new();
        for (index, (port, value)) in published.iter().enumerate() {
            assert_eq!(port, &format!("port-{index}"));
            assert_eq!(value, &expected);
            assert!(buffers.insert(pointers(value)));
            if index + 1 == count {
                assert_eq!(pointers(value), original_pointers);
            } else {
                let actual = pointers(value);
                assert_ne!(actual.0, original_pointers.0);
                assert_ne!(actual.1, original_pointers.1);
                assert_ne!(actual.2, original_pointers.2);
            }
        }
    }
}
