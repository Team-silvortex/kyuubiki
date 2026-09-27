defmodule KyuubikiWeb.WorkflowNamedInputContractTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowNamedInputContract, as: Fixture
  alias KyuubikiWeb.WorkflowOperatorRuntime

  for sample <- Fixture.cases() do
    @sample sample
    test "named graph inputs match direct operator execution: #{sample["operator_id"]}" do
      for reverse <- [false, true], envelope <- [nil, "input", "payload"] do
        request = @sample |> Fixture.request(envelope) |> Fixture.reorder(reverse)
        payload = Map.new(Enum.zip(@sample["ports"], @sample["values"]))

        assert {:ok, direct} =
                 WorkflowOperatorRuntime.run_transform_operator(
                   @sample["operator_id"],
                   payload,
                   @sample["config"]
                 )

        assert {:ok, run} = Engine.run_workflow_graph(request)
        assert run["failed_nodes"] == []
        assert run["artifacts"]["result.summary"] == direct
        assert Map.take(direct, Map.keys(@sample["expected"])) == @sample["expected"]
        assert run["artifacts"]["raw.payload"] == request["input_artifacts"]["left"]
        trace = Enum.find(run["node_runs"], &(&1["node_id"] == "combine"))

        expected_sources =
          if envelope, do: ["left.payload"], else: ["left.payload", "right.payload"]

        assert Enum.sort(trace["consumed_artifacts"]) == expected_sources
      end
    end
  end

  test "pair routing follows target ports, not edge order or nested left/right fields" do
    for sample <- pair_cases(), reverse <- [false, true] do
      poisoned =
        put_in(sample, ["values"], [
          %{
            "cost" => 10,
            "left" => %{"cost" => 1},
            "right" => %{"cost" => 99},
            "payload" => %{"cost" => 1}
          },
          %{"cost" => 4}
        ])

      assert {:ok, run} =
               Engine.run_workflow_graph(
                 poisoned
                 |> Fixture.request()
                 |> Fixture.reorder(reverse)
               )

      assert run["artifacts"]["result.summary"]["benchmark_winner"] == "right"

      swapped =
        sample
        |> Fixture.request()
        |> update_in(["graph", "edges"], fn edges ->
          Enum.map(edges, fn edge ->
            if edge["to"]["node"] == "combine" do
              put_in(
                edge,
                ["to", "port"],
                if(edge["to"]["port"] == "left", do: "right", else: "left")
              )
            else
              edge
            end
          end)
        end)
        |> Fixture.reorder(reverse)

      assert {:ok, run} = Engine.run_workflow_graph(swapped)
      assert run["artifacts"]["result.summary"]["benchmark_winner"] == "left"
    end
  end

  test "invalid second inputs isolate without a winner or result lineage and replay cleanly" do
    for sample <- pair_cases(), reverse <- [false, true] do
      request =
        sample
        |> Fixture.request()
        |> put_in(["input_artifacts", "right"], nil)
        |> Fixture.reorder(reverse)

      assert {:error, {:workflow_node_error, "combine", _}} = Engine.run_workflow_graph(request)

      request =
        update_in(request, ["graph", "nodes"], fn nodes ->
          Enum.map(
            nodes,
            &if(&1["id"] == "combine", do: put_in(&1, ["config", "on_error"], "skip"), else: &1)
          )
        end)

      assert {:ok, run} = Engine.run_workflow_graph(request)
      assert run["failed_nodes"] == ["combine"]
      assert run["skipped_nodes"] == ["result"]
      refute Map.has_key?(run["artifacts"], "combine.summary")
      refute Enum.any?(run["artifact_lineage"], &(&1["node_id"] == "combine"))
      assert run["artifacts"]["raw.payload"] == hd(sample["values"])
      assert {:ok, clean} = Engine.run_workflow_graph(Fixture.request(sample))
      assert clean["failed_nodes"] == []
    end
  end

  test "same-source fan-out retains both named bindings and the independent raw output" do
    for sample <- pair_cases(), reverse <- [false, true] do
      request = sample |> Fixture.request() |> Fixture.reorder(reverse)

      request =
        update_in(request, ["graph", "edges"], fn edges ->
          Enum.map(
            edges,
            &if(&1["id"] == "right-combine", do: put_in(&1, ["from", "node"], "left"), else: &1)
          )
        end)

      assert {:ok, run} = Engine.run_workflow_graph(request)
      assert run["artifacts"]["result.summary"]["benchmark_winner"] == "tie"
      assert run["artifacts"]["raw.payload"] == hd(sample["values"])
      trace = Enum.find(run["node_runs"], &(&1["node_id"] == "combine"))
      assert trace["consumed_artifacts"] == ["left.payload", "left.payload"]
    end
  end

  test "one missing named edge cannot reinterpret a nested pair as an input envelope" do
    for sample <- pair_cases(), reverse <- [false, true] do
      request = sample |> Fixture.request() |> Fixture.reorder(reverse)

      request =
        request
        |> put_in(["input_artifacts", "left"], %{
          "cost" => 10,
          "left" => %{"cost" => 1},
          "right" => %{"cost" => 99}
        })
        |> update_in(
          ["graph", "edges"],
          &Enum.reject(&1, fn edge -> edge["id"] == "right-combine" end)
        )

      assert {:error, {:workflow_node_error, "combine", _}} = Engine.run_workflow_graph(request)
    end
  end

  test "legacy single-field source-port envelopes unwrap without changing retained artifacts" do
    for sample <- Fixture.cases(), reverse <- [false, true] do
      request = sample |> Fixture.request() |> Fixture.reorder(reverse)

      request =
        update_in(request, ["input_artifacts"], fn inputs ->
          Map.new(inputs, fn {id, value} -> {id, %{"payload" => value}} end)
        end)

      assert {:ok, run} = Engine.run_workflow_graph(request)
      result = run["artifacts"]["result.summary"]
      assert Map.take(result, Map.keys(sample["expected"])) == sample["expected"]
      assert run["artifacts"]["raw.payload"] == request["input_artifacts"]["left"]
    end
  end

  defp pair_cases,
    do:
      Enum.reject(
        Fixture.cases(),
        &(&1["operator_id"] == "transform.join_parameter_sweep_results")
      )
end
