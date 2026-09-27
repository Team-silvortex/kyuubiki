defmodule KyuubikiWeb.TestSupport.WorkflowPreflightContract do
  @moduledoc false
  @fixture Path.expand("../../../../tests/fixtures/workflow-graph-preflight.json", __DIR__)
           |> File.read!()
           |> Jason.decode!()

  def request, do: @fixture["request"]
  def cases, do: @fixture["cases"]
  def budgets, do: @fixture["budgets"]

  def budget_request(%{"path" => path, "kind" => kind, "size" => size}) do
    value =
      case kind do
        "array" -> List.duplicate(0, size)
        "nested" -> Enum.reduce(1..size, 0, fn _, inner -> [inner] end)
        "string" -> String.duplicate("x", size)
        "key" -> %{String.duplicate("x", size) => 0}
      end

    request(%{"changes" => [%{"path" => path, "value" => value}]})
  end

  def budget_request(%{"kind" => "nodes", "size" => size}) do
    update_in(request(), ["graph", "nodes"], fn nodes ->
      nodes ++
        for i <- 4..size,
            do: %{"id" => "n#{i}", "kind" => "output", "inputs" => [], "outputs" => []}
    end)
  end

  def budget_request(%{"kind" => direction, "size" => size})
      when direction in ["inputs", "outputs"] do
    update_in(request(), ["graph", "nodes", Access.at(1), direction], fn ports ->
      ports ++ for i <- 2..size, do: %{"id" => "p#{i}", "artifact_type" => "artifact/json"}
    end)
  end

  def budget_request(%{"kind" => "edges", "size" => size}) do
    ports = for i <- 0..31, do: %{"id" => "p#{i}", "artifact_type" => "artifact/json"}

    nodes =
      for i <- 0..div(size - 1, 32),
          do: %{"id" => "n#{i}", "kind" => "output", "inputs" => ports, "outputs" => []}

    edges =
      for i <- 0..(size - 1),
          do: %{
            "id" => "e#{i}",
            "from" => %{"node" => "input", "port" => "payload"},
            "to" => %{"node" => "n#{div(i, 32)}", "port" => "p#{rem(i, 32)}"},
            "artifact_type" => "artifact/json"
          }

    request()
    |> put_in(["graph", "nodes"], [hd(request()["graph"]["nodes"]) | nodes])
    |> put_in(["graph", "output_nodes"], [])
    |> put_in(["graph", "edges"], edges)
  end

  def request(test_case) do
    Enum.reduce(test_case["changes"], request(), fn change, request ->
      path = Enum.map(change["path"], &if(is_integer(&1), do: Access.at(&1), else: &1))

      update_in(request, path, fn old ->
        if change["op"] == "append", do: old ++ [change["value"]], else: change["value"]
      end)
    end)
  end
end
