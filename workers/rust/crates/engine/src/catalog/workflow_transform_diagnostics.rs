use crate::catalog::descriptors::built_in_transform_descriptor;
use kyuubiki_protocol::OperatorDescriptor;

pub(super) fn workflow_diagnostics_transform_descriptors() -> Vec<OperatorDescriptor> {
    vec![
        built_in_transform_descriptor(
            "transform.compose_diagnostics_bundle",
            "multi_domain",
            "compose_diagnostics_bundle",
            "Compose multiple diagnostics payloads into a single workflow diagnostics bundle with domain, source, and metric-group metadata.",
            &[
                "transform",
                "diagnostics",
                "bundle",
                "compose",
                "headless_safe",
            ],
        ),
        built_in_transform_descriptor(
            "transform.evaluate_diagnostics_bundle_guard",
            "multi_domain",
            "evaluate_diagnostics_bundle_guard",
            "Evaluate a workflow diagnostics bundle against visible warn/block rules and emit a unified guard decision.",
            &[
                "transform",
                "diagnostics",
                "bundle",
                "guard",
                "headless_safe",
            ],
        ),
        built_in_transform_descriptor(
            "transform.compose_diagnostics_report_payload",
            "multi_domain",
            "compose_diagnostics_report_payload",
            "Compose a diagnostics bundle and guard result into a standard report payload for downstream export operators.",
            &[
                "transform",
                "diagnostics",
                "bundle",
                "report",
                "compose",
                "headless_safe",
            ],
        ),
        built_in_transform_descriptor(
            "transform.select_focus_payload",
            "multi_domain",
            "select_focus_payload",
            "Select one standard focus payload by metric id from a diagnostics report payload for downstream workflow chaining.",
            &[
                "transform",
                "diagnostics",
                "focus",
                "select",
                "headless_safe",
            ],
        ),
        built_in_transform_descriptor(
            "transform.compose_focus_chain_input",
            "multi_domain",
            "compose_focus_chain_input",
            "Compose a selected focus payload into a standard downstream chain input with bindings and orchestration annotations.",
            &[
                "transform",
                "diagnostics",
                "focus",
                "chain",
                "compose",
                "headless_safe",
            ],
        ),
        built_in_transform_descriptor(
            "transform.compose_focus_bridge_request",
            "multi_domain",
            "compose_focus_bridge_request",
            "Compose a focus chain input into a standard bridge request payload with explicit bridge operator and bridge config.",
            &[
                "transform",
                "diagnostics",
                "focus",
                "bridge",
                "compose",
                "headless_safe",
            ],
        ),
        built_in_transform_descriptor(
            "transform.resolve_focus_bridge_execution",
            "multi_domain",
            "resolve_focus_bridge_execution",
            "Resolve a focus bridge request into a directly executable bridge payload with operator id, source payload, and bridge config.",
            &[
                "transform",
                "diagnostics",
                "focus",
                "bridge",
                "execute",
                "headless_safe",
            ],
        ),
        built_in_transform_descriptor(
            "transform.execute_focus_bridge_execution",
            "multi_domain",
            "execute_focus_bridge_execution",
            "Execute a resolved focus bridge execution payload and emit the resulting bridge output with execution lineage.",
            &[
                "transform",
                "diagnostics",
                "focus",
                "bridge",
                "run",
                "headless_safe",
            ],
        ),
    ]
}
