defmodule KyuubikiSdk do
  @moduledoc "Protocol-first headless SDK entry point."

  alias KyuubikiSdk.Session
  alias KyuubikiSdk.WorkflowContracts
  alias KyuubikiSdk.WorkflowBuilders

  def new_session(opts \\ []), do: Session.new(opts)
  def validate_workflow_dataset_contract(contract), do: WorkflowContracts.validate_dataset_contract(contract)
  def validate_workflow_graph(graph), do: WorkflowContracts.validate_graph(graph)
  defdelegate workflow_schema_ref(schema, version), to: WorkflowBuilders, as: :schema_ref
  defdelegate workflow_axis(axis_id, attrs \\ %{}), to: WorkflowBuilders, as: :axis
  defdelegate workflow_shape(attrs \\ %{}), to: WorkflowBuilders, as: :shape
  defdelegate workflow_dataset_value(value_id, data_class, element_type, attrs \\ %{}), to: WorkflowBuilders, as: :dataset_value
  defdelegate workflow_dataset_contract(contract_id, version, values, attrs \\ %{}), to: WorkflowBuilders, as: :dataset_contract
  defdelegate workflow_port(port_id, artifact_type, attrs \\ %{}), to: WorkflowBuilders, as: :port
  defdelegate workflow_node(node_id, kind, attrs \\ %{}), to: WorkflowBuilders, as: :node
  defdelegate workflow_edge(edge_id, from_node, from_port, to_node, to_port, artifact_type, attrs \\ %{}), to: WorkflowBuilders, as: :edge
  defdelegate workflow_graph(graph_id, name, version, entry_nodes, nodes, edges, attrs \\ %{}), to: WorkflowBuilders, as: :graph
end
