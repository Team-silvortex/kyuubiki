defmodule KyuubikiWeb.WorkflowGraphProvenance do
  @moduledoc false
  alias KyuubikiWeb.Storage.DurableJson

  @schema "kyuubiki.workflow-input-binding/v1"
  @context_key "workflow_input_binding_sha256"

  def bind_context(context, binding) when is_map(context) and is_binary(binding),
    do: Map.put(context, @context_key, binding)

  def input_digest(state, consumed) do
    digest(%{"schema_version" => @schema, "sources" => sources(state, consumed)})
  end

  # Each artifact retains one ephemeral path digest, not its ancestors or value.
  def publish(state, key, node_id, port_id, consumed) do
    binding =
      digest(%{
        "schema_version" => @schema,
        "node_id" => node_id,
        "port_id" => port_id,
        "sources" => sources(state, consumed)
      })

    %{state | artifact_bindings: Map.put(state.artifact_bindings, key, binding)}
  end

  defp sources(state, consumed) do
    Enum.map(consumed, fn key ->
      %{"artifact_key" => key, "lineage_sha256" => Map.fetch!(state.artifact_bindings, key)}
    end)
  end

  defp digest(value),
    do:
      value
      |> DurableJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)
end
