defmodule KyuubikiWeb.WorkflowNativeInvocation do
  @moduledoc false
  alias KyuubikiWeb.Storage.DurableJson

  @schema "kyuubiki.workflow-native-invocation/v1"
  @context_key "workflow_native_invocation_sha256"

  def bind_context(context, payload, config)
      when is_map(context) and is_map(payload) and is_map(config) do
    context = Map.delete(context, @context_key)

    # TaskIR v1 rounds floats to fixed-15; bind exact JSON without changing that wire contract.
    binding =
      %{
        "schema_version" => @schema,
        "input_artifact" => payload,
        "config" => config,
        "orchestration_context" => context
      }
      |> DurableJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    Map.put(context, @context_key, binding)
  end
end
