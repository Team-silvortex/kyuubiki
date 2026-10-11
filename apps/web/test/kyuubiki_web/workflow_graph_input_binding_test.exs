defmodule KyuubikiWeb.WorkflowGraphInputBindingTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.{WorkflowGraphResponse, WorkflowGraphRunner}
  alias KyuubikiWeb.Orchestra.Engine

  @named_operators ~w(
    transform.benchmark_structural_pair transform.benchmark_acoustic_pair
    transform.benchmark_modal_pair transform.benchmark_dynamic_pair
    transform.benchmark_transport_pair transform.benchmark_coupled_heat_pair
    transform.benchmark_electrostatic_pair transform.benchmark_magnetostatic_pair
    transform.benchmark_cfd_pair transform.join_parameter_sweep_results
    transform.merge_summary_pair transform.compare_summary_pair
    transform.aggregate_summary_collection transform.select_best_summary
    transform.compose_quality_objective transform.compose_diagnostics_bundle
    transform.compose_diagnostics_report_payload transform.resolve_focus_bridge_execution
    transform.validate_electrostatic_heat_bridge transform.validate_heat_thermo_bridge
  )

  for operator <- @named_operators do
    @operator operator
    test "named input binding preserves both sources: #{operator}" do
      for reverse <- [false, true] do
        request = request(@operator, reverse)

        execute = fn id, payload, _config, node ->
          assert id == @operator
          assert node["id"] == "join"
          assert payload == %{"left" => input("a"), "right" => input("b")}
          send(self(), :executed)
          {:ok, payload}
        end

        assert {:ok, result} = run(request, execute)
        assert_receive :executed
        refute_receive :executed, 0

        assert result["artifacts"]["output.payload"] == %{
                 "left" => input("a"),
                 "right" => input("b")
               }

        assert join_run(result)["consumed_artifacts"] == ["a.payload", "b.payload"]
        assert join_lineage(result)["source_artifacts"] == ["a.payload", "b.payload"]
        assert result["failed_nodes"] == []
        assert result["skipped_nodes"] == []
      end
    end
  end

  test "fallback priority and lineage change together when edge priority changes" do
    for reverse <- [false, true], preferred <- ["a", "b"] do
      request = request("transform.first_available", reverse)

      request =
        if preferred == "b",
          do: update_in(request["graph"]["edges"], fn [a, b, out] -> [b, a, out] end),
          else: request

      assert {:ok, result} = Engine.run_workflow_graph(request)
      assert result["artifacts"]["output.payload"] == input(preferred)
      assert join_run(result)["consumed_artifacts"] == ["#{preferred}.payload"]
      assert join_lineage(result)["source_artifacts"] == ["#{preferred}.payload"]
    end
  end

  test "custom callbacks cannot silently discard an additional connected input" do
    for kind <- ~w(solve transform extract export), reverse <- [false, true] do
      request =
        request("custom.operator", reverse)
        |> update_in(["graph", "nodes"], fn nodes ->
          Enum.map(nodes, fn node ->
            if node["id"] == "join", do: Map.put(node, "kind", kind), else: node
          end)
        end)

      assert {:error, {:invalid_workflow_graph, message}} =
               run(request, fn _, _, _, _ -> flunk("ambiguous input executed") end)

      assert message =~ "single input"
    end
  end

  test "unused declared ports do not become consumed artifacts" do
    request =
      request("custom.operator", true)
      |> update_in(["graph", "edges"], fn [a, _b, out] -> [a, out] end)

    assert {:ok, result} = run(request, fn _, payload, _, _ -> {:ok, payload} end)
    assert result["artifacts"]["output.payload"] == input("a")
    assert join_run(result)["consumed_artifacts"] == ["a.payload"]
    assert join_lineage(result)["source_artifacts"] == ["a.payload"]
  end

  test "multi-port output publishes each input with its own lineage" do
    request =
      request("transform.first_available", true)
      |> update_in(["graph", "nodes"], fn nodes ->
        Enum.map(nodes, fn node ->
          if node["id"] == "output",
            do: Map.put(node, "inputs", [port("payload"), port("backup")]),
            else: node
        end)
      end)
      |> update_in(["graph", "edges"], &(&1 ++ [edge("b", "output", "backup")]))

    assert {:ok, result} = Engine.run_workflow_graph(request)
    assert result["artifacts"]["output.payload"] == input("a")
    assert result["artifacts"]["output.backup"] == input("b")

    for {port, source} <- [{"payload", "join.payload"}, {"backup", "b.payload"}] do
      entry = Enum.find(result["artifact_lineage"], &(&1["artifact_key"] == "output.#{port}"))
      assert entry["source_artifacts"] == [source]
    end
  end

  defp run(request, transform) do
    unused = fn _, _, _ -> flunk("unexpected operator executed") end

    WorkflowGraphRunner.run(request["graph"], request["input_artifacts"],
      execute_solve: unused,
      execute_transform: transform,
      execute_extract: unused,
      execute_export: unused,
      result_options: WorkflowGraphResponse.normalize_options(nil)
    )
  end

  defp join_run(result), do: Enum.find(result["node_runs"], &(&1["node_id"] == "join"))

  defp join_lineage(result),
    do: Enum.find(result["artifact_lineage"], &(&1["artifact_key"] == "join.payload"))

  defp input(id), do: %{"value" => id, "status" => "ready"}
  defp port(id), do: %{"id" => id, "artifact_type" => "artifact/json"}

  defp request(operator, reverse) do
    nodes = [
      %{"id" => "a", "kind" => "input", "inputs" => [], "outputs" => [port("payload")]},
      %{"id" => "b", "kind" => "input", "inputs" => [], "outputs" => [port("payload")]},
      %{
        "id" => "join",
        "kind" => "transform",
        "operator_id" => operator,
        "config" => %{"on_error" => "skip"},
        "inputs" => [port("left"), port("right")],
        "outputs" => [port("payload")]
      },
      %{"id" => "output", "kind" => "output", "inputs" => [port("payload")], "outputs" => []}
    ]

    %{
      "graph" => %{
        "id" => "input-binding",
        "schema_version" => "kyuubiki.workflow-graph/v1",
        "nodes" => if(reverse, do: Enum.reverse(nodes), else: nodes),
        "edges" => [
          edge("a", "join", "left"),
          edge("b", "join", "right"),
          edge("join", "output", "payload")
        ]
      },
      "input_artifacts" => %{"a" => input("a"), "b" => input("b")}
    }
  end

  defp edge(from, to, port),
    do: %{
      "id" => "#{from}-#{to}",
      "from" => %{"node" => from, "port" => "payload"},
      "to" => %{"node" => to, "port" => port},
      "artifact_type" => "artifact/json"
    }
end
