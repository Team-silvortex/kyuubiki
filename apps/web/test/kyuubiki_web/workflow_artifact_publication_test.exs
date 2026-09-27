defmodule KyuubikiWeb.WorkflowArtifactPublicationTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.{WorkflowGraphRunner, WorkflowGraphResponse}
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowArtifactContract, as: Fixture

  for kind <- ~w(solve transform extract export) do
    @kind kind
    test "#{kind} rejects generated output before publishing any port or lineage" do
      request =
        Fixture.node(
          Fixture.request(),
          "producer",
          &Map.merge(
            &1,
            %{"kind" => @kind, "operator_id" => "custom.generate"}
          )
        )

      bad_values = [
        String.duplicate("x", 500_001),
        %{"not_json" => self()},
        %{"late" => <<0>>},
        [1 | 2]
      ]

      for value <- bad_values, reverse <- [false, true], compact <- [false, true] do
        request =
          if reverse, do: update_in(request, ["graph", "nodes"], &Enum.reverse/1), else: request

        callback = fn _, _, _ -> {:ok, value} end
        {:ok, result} = run(request, callback, compact)
        assert result["failed_nodes"] == ["producer"]
        assert Enum.sort(result["skipped_nodes"]) == ["consumer", "output"]
        assert Enum.sort(result["completed_nodes"]) == ["input", "raw"]
        refute_receive :artifact_consumer_called, 0
        assert [%{"node_id" => "producer", "error_message" => reason}] = result["node_failures"]
        assert reason =~ "workflow node producer output"
        assert_receive {:publication_progress, %{"node_id" => "producer", "status" => "failed"}}

        refute_receive {:publication_progress,
                        %{"node_id" => "producer", "status" => "completed"}},
                       0

        if compact do
          refute Map.has_key?(result, "artifacts")
        else
          assert Map.keys(result["artifacts"]) |> Enum.sort() == ["input.payload", "raw.payload"]
          assert result["artifacts"]["raw.payload"] == %{"value" => 42}
          refute Enum.any?(result["artifact_lineage"], &(&1["node_id"] == "producer"))
          trace = Enum.find(result["node_runs"], &(&1["node_id"] == "producer"))
          assert trace["produced_artifacts"] == []
        end
      end
    end
  end

  test "invalid callback envelope is a node error, not a case-clause crash" do
    for kind <- ~w(solve transform extract export),
        value <- [:ok, nil, %{}, {:ok, 1, 2}, {:unknown, self()}],
        policy <- ["skip", "fail"] do
      request =
        Fixture.node(
          Fixture.request(),
          "producer",
          &Map.merge(
            &1,
            %{
              "kind" => kind,
              "operator_id" => "custom.generate",
              "config" => %{"on_error" => policy}
            }
          )
        )

      result = run(request, fn _, _, _ -> value end)

      if policy == "skip" do
        assert {:ok, result} = result
        assert result["failed_nodes"] == ["producer"]
        assert hd(result["node_failures"])["error_message"] =~ "invalid_operator_result"
      else
        assert {:error, {:workflow_node_error, "producer", :invalid_operator_result}} = result
      end

      refute_receive :artifact_consumer_called, 0
    end
  end

  test "default failure stops execution and a fresh run publishes only healthy outputs" do
    request = Fixture.node(Fixture.request(), "producer", &Map.delete(&1, "config"))

    case run(request, fn _, _, _ -> {:ok, self()} end) do
      {:error, {:workflow_node_error, "producer", reason}} -> assert reason =~ "only JSON values"
      _ -> flunk("invalid output must fail before downstream execution")
    end

    {:ok, healthy} = run(request, fn _, _, _ -> {:ok, %{"healthy" => true}} end)
    assert healthy["failed_nodes"] == []
    assert healthy["artifacts"]["producer.payload"] == %{"healthy" => true}
    assert healthy["artifacts"]["producer.aux"] == %{"healthy" => true}
    assert healthy["artifacts"]["output.payload"] == %{"healthy" => true}
  end

  test "generated values are checked even when the node declares no output ports" do
    request =
      Fixture.node(Fixture.request(), "producer", &Map.put(&1, "outputs", []))
      |> update_in(
        ["graph", "nodes"],
        &Enum.reject(&1, fn node -> node["id"] in ["consumer", "output"] end)
      )
      |> put_in(["graph", "output_nodes"], ["raw"])
      |> update_in(
        ["graph", "edges"],
        &Enum.filter(&1, fn edge -> edge["from"]["node"] == "input" end)
      )

    {:ok, result} = run(request, fn _, _, _ -> {:ok, self()} end)
    assert result["failed_nodes"] == ["producer"]
    refute Enum.any?(result["artifact_lineage"], &(&1["node_id"] == "producer"))
  end

  test "valid generated scalar and collection outputs survive forwarding without truncation" do
    for recipe <- Fixture.cases(), recipe["accept"] do
      value = Fixture.value(recipe)
      {:ok, result} = run(Fixture.request(), fn _, _, _ -> {:ok, value} end)
      assert result["failed_nodes"] == []
      assert result["artifacts"]["output.payload"] == value
      assert result["artifacts"]["producer.aux"] == value
    end
  end

  test "every shared negative budget is rejected atomically by the callback runner" do
    for recipe <- Fixture.cases(), not recipe["accept"] do
      {:ok, result} = run(Fixture.request(), fn _, _, _ -> {:ok, Fixture.value(recipe)} end)
      assert result["failed_nodes"] == ["producer"]
      assert hd(result["node_failures"])["error_message"] =~ recipe["error"]
      assert Map.keys(result["artifacts"]) |> Enum.sort() == ["input.payload", "raw.payload"]
    end
  end

  test "real export expansion cannot turn a valid input into an oversized published report" do
    for reverse <- [false, true], policy <- ["skip", "fail"] do
      request = Fixture.exported(policy, true)

      request =
        if reverse, do: update_in(request, ["graph", "nodes"], &Enum.reverse/1), else: request

      case Engine.run_workflow_graph(request) do
        {:ok, result} ->
          assert policy == "skip"
          assert result["failed_nodes"] == ["producer"]
          refute Map.has_key?(result["artifacts"], "producer.payload")
          refute Map.has_key?(result["artifacts"], "producer.aux")
          refute Map.has_key?(result["artifacts"], "output.payload")
          assert result["artifacts"]["raw.payload"] == request["input_artifacts"]["input"]

        {:error, {:workflow_node_error, "producer", reason}} ->
          assert policy == "fail"
          assert reason =~ "length security budget"
      end
    end

    {:ok, clean} = Engine.run_workflow_graph(Fixture.exported("fail", false))
    assert clean["failed_nodes"] == []

    assert Jason.decode!(clean["artifacts"]["output.payload"]["content"]) == %{
             "text" => "healthy"
           }
  end

  test "raw inputs and condition forwarding retain the separate input budget" do
    request =
      Fixture.request()
      |> Fixture.node(
        "producer",
        &Map.merge(&1, %{
          "kind" => "condition",
          "config" => %{"predicate" => %{"operator" => "truthy"}}
        })
      )
      |> update_in(["graph", "nodes"], &Enum.reject(&1, fn node -> node["id"] == "consumer" end))
      |> update_in(["graph", "edges"], fn [first, second, _third, raw] ->
        [first, put_in(second, ["to", "node"], "output"), raw]
      end)
      |> put_in(["input_artifacts", "input"], %{"text" => String.duplicate("x", 500_001)})

    {:ok, result} = Engine.run_workflow_graph(request)
    assert result["failed_nodes"] == []
    assert result["artifacts"]["output.payload"] == request["input_artifacts"]["input"]
  end

  defp run(request, callback, compact \\ false) do
    request = Fixture.node(request, "consumer", &Map.put(&1, "operator_id", "custom.forward"))

    execute = fn operator_id, payload, node ->
      if operator_id == "custom.forward" do
        send(self(), :artifact_consumer_called)
        {:ok, payload}
      else
        callback.(operator_id, payload, node)
      end
    end

    WorkflowGraphRunner.run(request["graph"], request["input_artifacts"],
      execute_solve: execute,
      execute_transform: execute,
      execute_extract: execute,
      execute_export: execute,
      progress_callback: &send(self(), {:publication_progress, &1}),
      result_options: if(compact, do: WorkflowGraphResponse.compact_options(), else: %{})
    )
  end
end
