defmodule KyuubikiWeb.TestSupport.WorkflowNamedInputContract do
  @moduledoc false
  @fixture Path.expand("../../../../tests/fixtures/workflow-named-input-contract.json", __DIR__)
           |> File.read!()
           |> Jason.decode!()

  def cases do
    Enum.map(@fixture["pair_operators"], &Map.put(@fixture["pair"], "operator_id", &1)) ++
      [@fixture["sweep"]]
  end

  def request(sample, envelope \\ nil) do
    ports = sample["ports"]
    [left, right] = sample["values"]

    @fixture["request"]
    |> Map.put("input_artifacts", %{"left" => left, "right" => right})
    |> update_in(["graph", "nodes"], fn nodes ->
      Enum.map(nodes, fn
        %{"id" => "combine"} = node ->
          node
          |> Map.put("operator_id", sample["operator_id"])
          |> Map.put("config", sample["config"])
          |> Map.put("inputs", Enum.map(ports, &port/1))

        node ->
          node
      end)
    end)
    |> update_in(["graph", "edges"], fn edges ->
      Enum.map(edges, fn edge ->
        case edge["id"] do
          "left-combine" -> put_in(edge, ["to", "port"], Enum.at(ports, 0))
          "right-combine" -> put_in(edge, ["to", "port"], Enum.at(ports, 1))
          _ -> edge
        end
      end)
    end)
    |> maybe_envelope(sample, envelope)
  end

  def reorder(request, reverse) do
    if reverse do
      request
      |> update_in(["graph", "nodes"], &Enum.reverse/1)
      |> update_in(["graph", "edges"], &Enum.reverse/1)
    else
      request
    end
  end

  defp maybe_envelope(request, _sample, nil), do: request

  defp maybe_envelope(request, sample, port_id) do
    request
    |> Map.put("input_artifacts", %{
      "left" => Map.new(Enum.zip(sample["ports"], sample["values"]))
    })
    |> put_in(["graph", "entry_nodes"], ["left"])
    |> update_in(["graph", "nodes"], fn nodes ->
      nodes
      |> Enum.reject(&(&1["id"] == "right"))
      |> Enum.map(
        &if(&1["id"] == "combine", do: Map.put(&1, "inputs", [port(port_id)]), else: &1)
      )
    end)
    |> update_in(["graph", "edges"], fn edges ->
      edges
      |> Enum.reject(&(&1["id"] == "right-combine"))
      |> Enum.map(
        &if(&1["id"] == "left-combine", do: put_in(&1, ["to", "port"], port_id), else: &1)
      )
    end)
  end

  defp port(id), do: %{"id" => id, "artifact_type" => "artifact/result_summary"}
end
