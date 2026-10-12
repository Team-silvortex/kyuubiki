defmodule KyuubikiWeb.TestSupport.QualityRecovery do
  @moduledoc false

  def valid("objective"), do: %{"thermal_quality_score" => 0.5, "thermal_quality_ready" => true}

  def valid("ranking"),
    do: %{"candidates" => %{"a" => %{"qualities" => %{"thermal" => valid("objective")}}}}

  def valid("round"),
    do: %{"ranking" => [%{"candidate_id" => "a", "score" => 0.5, "ready" => true}]}

  def invalid("objective"), do: %{"thermal_quality_score" => nil, "thermal_quality_ready" => true}
  def invalid("ranking"), do: %{"candidates" => %{"a" => nil}}
  def invalid("round"), do: Map.put(valid("round"), "rejected_candidate_count", 1)
  def partial, do: put_in(valid("ranking"), ["candidates", "b"], nil)

  def repaired,
    do: put_in(valid("ranking"), ["candidates", "b"], valid("ranking")["candidates"]["a"])

  def graph(kind, policy \\ "fail") do
    stages =
      case kind do
        "objective" ->
          [{"objective", "compose_quality_objective", "thermal"}]

        "ranking" ->
          [
            {"ranking", "rank_quality_candidates", "payload"},
            {"round", "prepare_quality_next_round_request", "payload"}
          ]

        "round" ->
          [{"round", "prepare_quality_next_round_request", "payload"}]
      end

    transforms =
      Enum.map(stages, fn {id, operator, input} ->
        %{
          "id" => id,
          "kind" => "transform",
          "operator_id" => "transform.#{operator}",
          "config" => %{"on_error" => policy},
          "inputs" => [port(input)],
          "outputs" => [port("result")]
        }
      end)

    nodes =
      [%{"id" => "input", "kind" => "input", "outputs" => [port("result")]}] ++
        transforms ++
        [
          %{
            "id" => "export",
            "kind" => "export",
            "operator_id" => "export.summary_json",
            "config" => %{},
            "inputs" => [port("summary")],
            "outputs" => [port("json", "export/json")]
          },
          %{
            "id" => "output",
            "kind" => "output",
            "inputs" => [port("json", "export/json")],
            "outputs" => []
          }
        ]

    edges =
      nodes
      |> Enum.chunk_every(2, 1, :discard)
      |> Enum.with_index()
      |> Enum.map(fn {[from, to], index} ->
        output = hd(from["outputs"])
        input = hd(to["inputs"])

        %{
          "id" => "e#{index}",
          "from" => %{"node" => from["id"], "port" => output["id"]},
          "to" => %{"node" => to["id"], "port" => input["id"]},
          "artifact_type" => output["artifact_type"]
        }
      end)

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.quality-#{kind}-#{policy}",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => nodes,
      "edges" => edges
    }
  end

  defp port(id, type \\ "report/summary"), do: %{"id" => id, "artifact_type" => type}
end
