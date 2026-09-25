defmodule KyuubikiWeb.WorkflowGraphRecovery do
  @moduledoc false

  def validate(nodes) do
    Enum.reduce_while(nodes, :ok, fn node, :ok ->
      case validate_config(Map.get(node, "config")) do
        :ok -> {:cont, :ok}
        {:error, reason} -> {:halt, {:error, {:workflow_node_error, node["id"], reason}}}
      end
    end)
  end

  def skip_on_error?(%{"on_error" => policy}), do: policy == "skip"
  def skip_on_error?(%{"recovery" => %{"on_error" => policy}}), do: policy == "skip"
  def skip_on_error?(_config), do: false

  defp validate_config(config) when is_map(config) do
    with :ok <- validate_policy(config, "config.on_error") do
      case Map.get(config, "recovery") do
        recovery when is_map(recovery) -> validate_policy(recovery, "config.recovery.on_error")
        _ -> :ok
      end
    end
  end

  defp validate_config(_config), do: :ok

  defp validate_policy(%{"on_error" => policy}, path) when policy not in ["skip", "fail"],
    do: {:error, "#{path} must be a string recovery policy: skip or fail"}

  defp validate_policy(_config, _path), do: :ok
end
