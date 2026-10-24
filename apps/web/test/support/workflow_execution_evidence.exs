defmodule KyuubikiWeb.TestSupport.WorkflowExecutionEvidence do
  @moduledoc false
  @fixture Path.expand("../../../../tests/fixtures/workflow-execution-evidence.json", __DIR__)
           |> File.read!()
           |> Jason.decode!()

  def cases, do: @fixture["cases"]

  def request(value, reverse_nodes \\ false, backup_first \\ false) do
    @fixture["request"]
    |> Map.put("input_artifacts", %{"a" => value, "b" => value})
    |> update_in(["graph", "nodes"], &if(reverse_nodes, do: Enum.reverse(&1), else: &1))
    |> update_in(["graph", "edges"], fn [a, b | rest] ->
      if backup_first, do: [b, a | rest], else: [a, b | rest]
    end)
  end

  def failure(selected, aliased \\ false, reverse_nodes \\ false) do
    request(true, reverse_nodes, selected == "b")
    |> put_in(["input_artifacts", selected], %{"text" => String.duplicate("x", 500_001)})
    |> alias_backup(aliased)
  end

  def alias_backup(request, true) do
    update_in(request, ["graph", "edges"], fn edges ->
      Enum.map(edges, fn
        %{"id" => "b-choose"} = edge -> put_in(edge, ["from", "node"], "a")
        edge -> edge
      end)
    end)
  end

  def alias_backup(request, false), do: request

  def node(request, id, update) do
    update_in(request, ["graph", "nodes"], fn nodes ->
      Enum.map(nodes, &if(&1["id"] == id, do: update.(&1), else: &1))
    end)
  end

  def trace(result, id), do: Enum.find(result["node_runs"], &(&1["node_id"] == id))

  def published(result, id) do
    result["artifact_lineage"]
    |> Enum.filter(&(&1["node_id"] == id))
    |> Enum.map(& &1["artifact_key"])
    |> Enum.sort()
  end
end
