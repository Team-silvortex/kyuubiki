from __future__ import annotations

from typing import Any

from .errors import WorkflowContractValidationError

WORKFLOW_DATASET_SCHEMA_VERSION = "kyuubiki.workflow-dataset/v1"
WORKFLOW_GRAPH_SCHEMA_VERSION = "kyuubiki.workflow-graph/v1"
WORKFLOW_DATA_CLASSES = {
    "study_model",
    "result",
    "field",
    "table",
    "report",
    "export",
    "scalar",
    "metadata",
}
WORKFLOW_ENCODINGS = {"json", "json_lines", "f64_le", "f32_le", "i64_le", "i32_le", "u8"}
WORKFLOW_CACHE_POLICIES = {"ephemeral", "cached", "persisted"}
WORKFLOW_NODE_KINDS = {"input", "solve", "transform", "extract", "export", "condition", "output"}
WORKFLOW_PORT_CARDINALITIES = {"one", "many"}
WORKFLOW_OPERATOR_NODE_KINDS = {"solve", "transform", "extract", "export", "condition"}
WORKFLOW_DISPATCH_POLICIES = {"orchestra_only", "central_fetch", "direct_mesh", "local_only"}


def validate_workflow_dataset_contract(contract: dict[str, Any]) -> dict[str, Any]:
    errors: list[str] = []
    _validate_dataset_contract(contract, errors, "dataset")
    if errors:
        raise WorkflowContractValidationError(errors)
    return contract


def validate_workflow_graph(graph: dict[str, Any]) -> dict[str, Any]:
    errors: list[str] = []
    if not isinstance(graph, dict):
        raise WorkflowContractValidationError(["graph must be an object"])

    _require_non_empty_string(graph, "schema_version", errors, "graph")
    if graph.get("schema_version") != WORKFLOW_GRAPH_SCHEMA_VERSION:
        errors.append(f"graph.schema_version must be {WORKFLOW_GRAPH_SCHEMA_VERSION!r}")
    _require_non_empty_string(graph, "id", errors, "graph")
    _require_non_empty_string(graph, "name", errors, "graph")
    _require_non_empty_string(graph, "version", errors, "graph")
    _optional_string(graph, "description", errors, "graph")

    contract = graph.get("dataset_contract")
    dataset_semantics: dict[str, str | None] = {}
    if contract is not None:
        if isinstance(contract, dict):
            _validate_dataset_contract(contract, errors, "graph.dataset_contract")
            dataset_semantics = {
                value["id"]: value.get("semantic_type")
                for value in contract.get("values", [])
                if isinstance(value, dict) and isinstance(value.get("id"), str)
            }
        else:
            errors.append("graph.dataset_contract must be an object")

    defaults = graph.get("defaults")
    if defaults is not None:
        if not isinstance(defaults, dict):
            errors.append("graph.defaults must be an object")
        else:
            cache_policy = defaults.get("cache_policy")
            if cache_policy is not None and cache_policy not in WORKFLOW_CACHE_POLICIES:
                errors.append(f"graph.defaults.cache_policy must be one of {sorted(WORKFLOW_CACHE_POLICIES)!r}")
            orchestrated = defaults.get("orchestrated")
            if orchestrated is not None and not isinstance(orchestrated, bool):
                errors.append("graph.defaults.orchestrated must be a boolean")
            _validate_dispatch_policy(defaults.get("dispatch_policy"), errors, "graph.defaults.dispatch_policy")
            _validate_string_list(defaults.get("placement_tags"), errors, "graph.defaults.placement_tags")
            _validate_string_list(defaults.get("required_capabilities"), errors, "graph.defaults.required_capabilities")

    _validate_dispatch_policy(graph.get("dispatch_policy"), errors, "graph.dispatch_policy")
    _validate_string_list(graph.get("placement_tags"), errors, "graph.placement_tags")
    _validate_string_list(graph.get("required_capabilities"), errors, "graph.required_capabilities")
    _validate_operator_fetch_plan(graph.get("operator_fetch_plan"), errors, "graph.operator_fetch_plan")

    entry_nodes = _require_list(graph, "entry_nodes", errors, "graph", min_items=1)
    output_nodes = _optional_list(graph, "output_nodes", errors, "graph")
    nodes = _require_list(graph, "nodes", errors, "graph", min_items=1)
    edges = _require_list(graph, "edges", errors, "graph")

    node_map: dict[str, dict[str, Any]] = {}
    output_ports: dict[tuple[str, str], dict[str, Any]] = {}
    input_ports: dict[tuple[str, str], dict[str, Any]] = {}
    for index, node in enumerate(nodes):
        location = f"graph.nodes[{index}]"
        if not isinstance(node, dict):
            errors.append(f"{location} must be an object")
            continue
        node_id = _require_non_empty_string(node, "id", errors, location)
        kind = _require_non_empty_string(node, "kind", errors, location)
        if kind is not None and kind not in WORKFLOW_NODE_KINDS:
            errors.append(f"{location}.kind must be one of {sorted(WORKFLOW_NODE_KINDS)!r}")
        if node_id:
            if node_id in node_map:
                errors.append(f"graph.nodes contains duplicate id {node_id!r}")
            else:
                node_map[node_id] = node
        if kind in WORKFLOW_OPERATOR_NODE_KINDS:
            _require_non_empty_string(node, "operator_id", errors, location)
        else:
            _optional_string(node, "operator_id", errors, location)
        _optional_string(node, "name", errors, location)
        _optional_string(node, "description", errors, location)
        if "config" in node and not isinstance(node["config"], dict):
            errors.append(f"{location}.config must be an object")
        cache_policy = node.get("cache_policy")
        if cache_policy is not None and cache_policy not in WORKFLOW_CACHE_POLICIES:
            errors.append(f"{location}.cache_policy must be one of {sorted(WORKFLOW_CACHE_POLICIES)!r}")
        _validate_string_list(node.get("placement_tags"), errors, f"{location}.placement_tags")
        _validate_string_list(node.get("required_capabilities"), errors, f"{location}.required_capabilities")

        for port_kind, port_bucket in (("inputs", input_ports), ("outputs", output_ports)):
            ports = _require_list(node, port_kind, errors, location)
            seen_port_ids: set[str] = set()
            for port_index, port in enumerate(ports):
                port_location = f"{location}.{port_kind}[{port_index}]"
                if not isinstance(port, dict):
                    errors.append(f"{port_location} must be an object")
                    continue
                port_id = _require_non_empty_string(port, "id", errors, port_location)
                _optional_string(port, "name", errors, port_location)
                _require_non_empty_string(port, "artifact_type", errors, port_location)
                if "required" in port and not isinstance(port["required"], bool):
                    errors.append(f"{port_location}.required must be a boolean")
                cardinality = port.get("cardinality")
                if cardinality is not None and cardinality not in WORKFLOW_PORT_CARDINALITIES:
                    errors.append(f"{port_location}.cardinality must be one of {sorted(WORKFLOW_PORT_CARDINALITIES)!r}")
                _validate_dataset_reference(
                    port.get("dataset_value"),
                    port.get("artifact_type"),
                    dataset_semantics,
                    errors,
                    port_location,
                )
                if port_id:
                    if port_id in seen_port_ids:
                        errors.append(f"{location}.{port_kind} contains duplicate port id {port_id!r}")
                    else:
                        seen_port_ids.add(port_id)
                        if node_id:
                            port_bucket[(node_id, port_id)] = port

    for field_name, node_ids in (("entry_nodes", entry_nodes), ("output_nodes", output_nodes)):
        for index, node_id in enumerate(node_ids):
            if not isinstance(node_id, str) or not node_id.strip():
                errors.append(f"graph.{field_name}[{index}] must be a non-empty string")
            elif node_id not in node_map:
                errors.append(f"graph.{field_name}[{index}] references unknown node {node_id!r}")

    seen_edge_ids: set[str] = set()
    for index, edge in enumerate(edges):
        location = f"graph.edges[{index}]"
        if not isinstance(edge, dict):
            errors.append(f"{location} must be an object")
            continue
        edge_id = _require_non_empty_string(edge, "id", errors, location)
        if edge_id:
            if edge_id in seen_edge_ids:
                errors.append(f"graph.edges contains duplicate id {edge_id!r}")
            else:
                seen_edge_ids.add(edge_id)
        artifact_type = _require_non_empty_string(edge, "artifact_type", errors, location)
        dataset_value = edge.get("dataset_value")
        _validate_dataset_reference(
            dataset_value,
            artifact_type,
            dataset_semantics,
            errors,
            location,
        )

        source_ref = _validate_node_port_ref(edge.get("from"), errors, f"{location}.from")
        target_ref = _validate_node_port_ref(edge.get("to"), errors, f"{location}.to")
        if source_ref is None or target_ref is None:
            continue
        source_port = output_ports.get(source_ref)
        target_port = input_ports.get(target_ref)
        if source_port is None:
            errors.append(f"{location}.from references unknown output port {source_ref!r}")
        if target_port is None:
            errors.append(f"{location}.to references unknown input port {target_ref!r}")
        if source_port is not None and artifact_type and source_port.get("artifact_type") != artifact_type:
            errors.append(f"{location}.artifact_type does not match source output port artifact_type")
        if target_port is not None and artifact_type and target_port.get("artifact_type") != artifact_type:
            errors.append(f"{location}.artifact_type does not match target input port artifact_type")
        if source_port is not None and target_port is not None and source_port.get("artifact_type") != target_port.get("artifact_type"):
            errors.append(f"{location} connects ports with mismatched artifact_type values")
        if dataset_value is not None and source_port is not None and source_port.get("dataset_value") not in (None, dataset_value):
            errors.append(f"{location}.dataset_value does not match source output port dataset_value")
        if dataset_value is not None and target_port is not None and target_port.get("dataset_value") not in (None, dataset_value):
            errors.append(f"{location}.dataset_value does not match target input port dataset_value")

    if errors:
        raise WorkflowContractValidationError(errors)
    return graph


def _validate_dataset_contract(contract: dict[str, Any], errors: list[str], location: str) -> None:
    if not isinstance(contract, dict):
        errors.append(f"{location} must be an object")
        return
    _require_non_empty_string(contract, "schema_version", errors, location)
    if contract.get("schema_version") != WORKFLOW_DATASET_SCHEMA_VERSION:
        errors.append(f"{location}.schema_version must be {WORKFLOW_DATASET_SCHEMA_VERSION!r}")
    _require_non_empty_string(contract, "id", errors, location)
    _require_non_empty_string(contract, "version", errors, location)
    _optional_string(contract, "name", errors, location)
    _optional_string(contract, "description", errors, location)
    metadata = contract.get("metadata")
    if metadata is not None:
        if not isinstance(metadata, dict):
            errors.append(f"{location}.metadata must be an object")
        else:
            for key, value in metadata.items():
                if not isinstance(key, str) or not key.strip():
                    errors.append(f"{location}.metadata contains an empty key")
                if not isinstance(value, str):
                    errors.append(f"{location}.metadata[{key!r}] must be a string")
    values = _require_list(contract, "values", errors, location, min_items=1)
    seen_value_ids: set[str] = set()
    for index, value in enumerate(values):
        value_location = f"{location}.values[{index}]"
        if not isinstance(value, dict):
            errors.append(f"{value_location} must be an object")
            continue
        value_id = _require_non_empty_string(value, "id", errors, value_location)
        if value_id:
            if value_id in seen_value_ids:
                errors.append(f"{location}.values contains duplicate id {value_id!r}")
            else:
                seen_value_ids.add(value_id)
        data_class = _require_non_empty_string(value, "data_class", errors, value_location)
        if data_class is not None and data_class not in WORKFLOW_DATA_CLASSES:
            errors.append(f"{value_location}.data_class must be one of {sorted(WORKFLOW_DATA_CLASSES)!r}")
        _require_non_empty_string(value, "element_type", errors, value_location)
        shape = value.get("shape")
        if not isinstance(shape, dict):
            errors.append(f"{value_location}.shape must be an object")
        else:
            axes = _optional_list(shape, "axes", errors, f"{value_location}.shape")
            seen_axis_ids: set[str] = set()
            for axis_index, axis in enumerate(axes):
                axis_location = f"{value_location}.shape.axes[{axis_index}]"
                if not isinstance(axis, dict):
                    errors.append(f"{axis_location} must be an object")
                    continue
                axis_id = _require_non_empty_string(axis, "id", errors, axis_location)
                if axis_id:
                    if axis_id in seen_axis_ids:
                        errors.append(f"{value_location}.shape.axes contains duplicate id {axis_id!r}")
                    else:
                        seen_axis_ids.add(axis_id)
                _optional_string(axis, "label", errors, axis_location)
                _optional_string(axis, "semantic", errors, axis_location)
                if "size" in axis and (not isinstance(axis["size"], int) or isinstance(axis["size"], bool) or axis["size"] < 0):
                    errors.append(f"{axis_location}.size must be a non-negative integer")
        _optional_string(value, "semantic_type", errors, value_location)
        _optional_string(value, "unit", errors, value_location)
        encoding = value.get("encoding")
        if encoding is not None and encoding not in WORKFLOW_ENCODINGS:
            errors.append(f"{value_location}.encoding must be one of {sorted(WORKFLOW_ENCODINGS)!r}")
        schema_ref = value.get("schema_ref")
        if schema_ref is not None:
            if not isinstance(schema_ref, dict):
                errors.append(f"{value_location}.schema_ref must be an object")
            else:
                _require_non_empty_string(schema_ref, "schema", errors, f"{value_location}.schema_ref")
                _require_non_empty_string(schema_ref, "version", errors, f"{value_location}.schema_ref")


def _validate_node_port_ref(value: Any, errors: list[str], location: str) -> tuple[str, str] | None:
    if not isinstance(value, dict):
        errors.append(f"{location} must be an object")
        return None
    node = _require_non_empty_string(value, "node", errors, location)
    port = _require_non_empty_string(value, "port", errors, location)
    if node is None or port is None:
        return None
    return node, port


def _validate_dataset_reference(
    dataset_value: Any,
    artifact_type: Any,
    dataset_semantics: dict[str, str | None],
    errors: list[str],
    location: str,
) -> None:
    if dataset_value is None:
        return
    if not isinstance(dataset_value, str) or not dataset_value.strip():
        errors.append(f"{location}.dataset_value must be a non-empty string")
        return
    if not dataset_semantics:
        return
    if dataset_value not in dataset_semantics:
        errors.append(
            f"{location}.dataset_value {dataset_value!r} is not declared in graph.dataset_contract"
        )
        return
    semantic_type = dataset_semantics[dataset_value]
    if semantic_type is not None and semantic_type != artifact_type:
        errors.append(
            f"{location}.dataset_value {dataset_value!r} semantic_type {semantic_type!r} "
            f"does not match artifact_type {artifact_type!r}"
        )


def _validate_dispatch_policy(value: Any, errors: list[str], location: str) -> None:
    if value is None:
        return
    if value not in WORKFLOW_DISPATCH_POLICIES:
        errors.append(f"{location} must be one of {sorted(WORKFLOW_DISPATCH_POLICIES)!r}")


def _validate_string_list(value: Any, errors: list[str], location: str) -> None:
    if value is None:
        return
    if not isinstance(value, list):
        errors.append(f"{location} must be a list")
        return
    for index, item in enumerate(value):
        if not isinstance(item, str) or not item.strip():
            errors.append(f"{location}[{index}] must be a non-empty string")


def _validate_operator_fetch_plan(value: Any, errors: list[str], location: str) -> None:
    if value is None:
        return
    if not isinstance(value, list):
        errors.append(f"{location} must be a list")
        return
    for index, entry in enumerate(value):
        entry_location = f"{location}[{index}]"
        if not isinstance(entry, dict):
            errors.append(f"{entry_location} must be an object")
            continue
        _require_non_empty_string(entry, "node_id", errors, entry_location)
        _require_non_empty_string(entry, "operator_id", errors, entry_location)
        _optional_string(entry, "package_ref", errors, entry_location)
        _optional_string(entry, "version", errors, entry_location)
        _optional_string(entry, "integrity", errors, entry_location)
        _optional_string(entry, "cache_scope", errors, entry_location)


def _require_list(container: dict[str, Any], key: str, errors: list[str], location: str, *, min_items: int = 0) -> list[Any]:
    value = container.get(key)
    if not isinstance(value, list):
        errors.append(f"{location}.{key} must be a list")
        return []
    if len(value) < min_items:
        errors.append(f"{location}.{key} must contain at least {min_items} item(s)")
    return value


def _optional_list(container: dict[str, Any], key: str, errors: list[str], location: str) -> list[Any]:
    value = container.get(key)
    if value is None:
        return []
    if not isinstance(value, list):
        errors.append(f"{location}.{key} must be a list")
        return []
    return value


def _require_non_empty_string(container: dict[str, Any], key: str, errors: list[str], location: str) -> str | None:
    value = container.get(key)
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{location}.{key} must be a non-empty string")
        return None
    return value


def _optional_string(container: dict[str, Any], key: str, errors: list[str], location: str) -> str | None:
    value = container.get(key)
    if value is None:
        return None
    if not isinstance(value, str):
        errors.append(f"{location}.{key} must be a string")
        return None
    if not value and key != "description":
        errors.append(f"{location}.{key} must not be empty")
        return None
    return value
