from __future__ import annotations

from typing import Any

from .workflow_contracts import (
    WORKFLOW_DATASET_SCHEMA_VERSION,
    WORKFLOW_GRAPH_SCHEMA_VERSION,
    validate_workflow_dataset_contract,
    validate_workflow_graph,
)


def build_workflow_schema_ref(schema: str, version: str) -> dict[str, str]:
    return {"schema": schema, "version": version}


def build_workflow_axis(
    axis_id: str,
    *,
    label: str | None = None,
    size: int | None = None,
    semantic: str | None = None,
) -> dict[str, Any]:
    axis: dict[str, Any] = {"id": axis_id}
    if label is not None:
        axis["label"] = label
    if size is not None:
        axis["size"] = size
    if semantic is not None:
        axis["semantic"] = semantic
    return axis


def build_workflow_shape(*, axes: list[dict[str, Any]] | None = None) -> dict[str, Any]:
    shape: dict[str, Any] = {}
    if axes is not None:
        shape["axes"] = axes
    return shape


def build_workflow_dataset_value(
    value_id: str,
    *,
    data_class: str,
    element_type: str,
    shape: dict[str, Any] | None = None,
    semantic_type: str | None = None,
    unit: str | None = None,
    encoding: str | None = None,
    schema_ref: dict[str, Any] | None = None,
) -> dict[str, Any]:
    value: dict[str, Any] = {
        "id": value_id,
        "data_class": data_class,
        "element_type": element_type,
        "shape": shape or {},
    }
    if semantic_type is not None:
        value["semantic_type"] = semantic_type
    if unit is not None:
        value["unit"] = unit
    if encoding is not None:
        value["encoding"] = encoding
    if schema_ref is not None:
        value["schema_ref"] = schema_ref
    return value


def build_workflow_dataset_contract(
    contract_id: str,
    *,
    version: str,
    values: list[dict[str, Any]],
    name: str | None = None,
    description: str | None = None,
    metadata: dict[str, str] | None = None,
    validate: bool = True,
) -> dict[str, Any]:
    contract: dict[str, Any] = {
        "schema_version": WORKFLOW_DATASET_SCHEMA_VERSION,
        "id": contract_id,
        "version": version,
        "values": values,
    }
    if name is not None:
        contract["name"] = name
    if description is not None:
        contract["description"] = description
    if metadata is not None:
        contract["metadata"] = metadata
    return validate_workflow_dataset_contract(contract) if validate else contract


def build_workflow_port(
    port_id: str,
    *,
    artifact_type: str,
    name: str | None = None,
    required: bool | None = None,
    cardinality: str | None = None,
    dataset_value: str | None = None,
) -> dict[str, Any]:
    port: dict[str, Any] = {"id": port_id, "artifact_type": artifact_type}
    if name is not None:
        port["name"] = name
    if required is not None:
        port["required"] = required
    if cardinality is not None:
        port["cardinality"] = cardinality
    if dataset_value is not None:
        port["dataset_value"] = dataset_value
    return port


def build_workflow_node(
    node_id: str,
    *,
    kind: str,
    inputs: list[dict[str, Any]],
    outputs: list[dict[str, Any]],
    operator_id: str | None = None,
    name: str | None = None,
    description: str | None = None,
    config: dict[str, Any] | None = None,
    cache_policy: str | None = None,
) -> dict[str, Any]:
    node: dict[str, Any] = {
        "id": node_id,
        "kind": kind,
        "inputs": inputs,
        "outputs": outputs,
    }
    if operator_id is not None:
        node["operator_id"] = operator_id
    if name is not None:
        node["name"] = name
    if description is not None:
        node["description"] = description
    if config is not None:
        node["config"] = config
    if cache_policy is not None:
        node["cache_policy"] = cache_policy
    return node


def build_workflow_edge(
    edge_id: str,
    *,
    from_node: str,
    from_port: str,
    to_node: str,
    to_port: str,
    artifact_type: str,
    dataset_value: str | None = None,
) -> dict[str, Any]:
    edge: dict[str, Any] = {
        "id": edge_id,
        "from": {"node": from_node, "port": from_port},
        "to": {"node": to_node, "port": to_port},
        "artifact_type": artifact_type,
    }
    if dataset_value is not None:
        edge["dataset_value"] = dataset_value
    return edge


def build_workflow_graph(
    graph_id: str,
    *,
    name: str,
    version: str,
    entry_nodes: list[str],
    nodes: list[dict[str, Any]],
    edges: list[dict[str, Any]],
    description: str | None = None,
    dataset_contract: dict[str, Any] | None = None,
    output_nodes: list[str] | None = None,
    defaults: dict[str, Any] | None = None,
    validate: bool = True,
) -> dict[str, Any]:
    graph: dict[str, Any] = {
        "schema_version": WORKFLOW_GRAPH_SCHEMA_VERSION,
        "id": graph_id,
        "name": name,
        "version": version,
        "entry_nodes": entry_nodes,
        "nodes": nodes,
        "edges": edges,
    }
    if description is not None:
        graph["description"] = description
    if dataset_contract is not None:
        graph["dataset_contract"] = dataset_contract
    if output_nodes is not None:
        graph["output_nodes"] = output_nodes
    if defaults is not None:
        graph["defaults"] = defaults
    return validate_workflow_graph(graph) if validate else graph
