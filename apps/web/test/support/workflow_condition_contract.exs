defmodule KyuubikiWeb.TestSupport.WorkflowConditionContract do
  @moduledoc false
  @fixture Path.expand("../../../../tests/fixtures/workflow-condition-contract.json", __DIR__)
           |> File.read!()
           |> Jason.decode!()

  def cases, do: @fixture["cases"]

  def request(sample, policy \\ "fail", reverse \\ false) do
    config = sample["config"]
    config = if is_map(config), do: Map.put(config, "on_error", policy), else: config

    @fixture["request"]
    |> put_in(["input_artifacts", "input"], sample["payload"])
    |> update_in(["graph", "nodes"], fn nodes ->
      nodes =
        Enum.map(nodes, &if(&1["id"] == "gate", do: Map.put(&1, "config", config), else: &1))

      if reverse, do: Enum.reverse(nodes), else: nodes
    end)
  end
end
