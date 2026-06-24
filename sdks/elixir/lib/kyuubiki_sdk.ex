defmodule KyuubikiSdk do
  @moduledoc "Protocol-first headless SDK entry point."

  alias KyuubikiSdk.Session
  alias KyuubikiSdk.WorkflowContracts
  alias KyuubikiSdk.WorkflowBuilders
  alias KyuubikiSdk.WorkflowResults
  alias KyuubikiSdk.MaterialReports

  def new_session(opts \\ []), do: Session.new(opts)
  def material_study_catalog, do: MaterialReports.material_study_catalog()
  def describe_material_study(study), do: MaterialReports.describe_material_study(study)

  def extract_material_result_payloads(payload),
    do: MaterialReports.extract_material_result_payloads(payload)

  def build_material_report(study, result_payloads, opts \\ []),
    do: MaterialReports.build_material_report(study, result_payloads, opts)

  def build_material_report_from_payload(study, payload, opts \\ []),
    do: MaterialReports.build_material_report_from_payload(study, payload, opts)

  def validate_workflow_dataset_contract(contract),
    do: WorkflowContracts.validate_dataset_contract(contract)

  def validate_workflow_graph(graph), do: WorkflowContracts.validate_graph(graph)
  def build_workflow_output_manifest(graph), do: WorkflowResults.build_output_manifest(graph)

  def normalize_workflow_progression(history, result_payload \\ nil),
    do: WorkflowResults.normalize_progression(history, result_payload)

  def normalize_workflow_runtime(payload), do: WorkflowResults.normalize_runtime(payload)

  def validate_workflow_result_against_graph(graph, payload),
    do: WorkflowResults.validate_result_against_graph(graph, payload)

  defdelegate workflow_schema_ref(schema, version), to: WorkflowBuilders, as: :schema_ref
  defdelegate workflow_axis(axis_id, attrs \\ %{}), to: WorkflowBuilders, as: :axis
  defdelegate workflow_shape(attrs \\ %{}), to: WorkflowBuilders, as: :shape

  defdelegate workflow_dataset_value(value_id, data_class, element_type, attrs \\ %{}),
    to: WorkflowBuilders,
    as: :dataset_value

  defdelegate workflow_dataset_contract(contract_id, version, values, attrs \\ %{}),
    to: WorkflowBuilders,
    as: :dataset_contract

  defdelegate workflow_port(port_id, artifact_type, attrs \\ %{}), to: WorkflowBuilders, as: :port
  defdelegate workflow_defaults(attrs \\ %{}), to: WorkflowBuilders, as: :defaults

  defdelegate workflow_operator_fetch_entry(node_id, operator_id, attrs \\ %{}),
    to: WorkflowBuilders,
    as: :operator_fetch_entry

  defdelegate workflow_node(node_id, kind, attrs \\ %{}), to: WorkflowBuilders, as: :node

  defdelegate workflow_edge(
                edge_id,
                from_node,
                from_port,
                to_node,
                to_port,
                artifact_type,
                attrs \\ %{}
              ), to: WorkflowBuilders, as: :edge

  defdelegate workflow_graph(graph_id, name, version, entry_nodes, nodes, edges, attrs \\ %{}),
    to: WorkflowBuilders,
    as: :graph
end
