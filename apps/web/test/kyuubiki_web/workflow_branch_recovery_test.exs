defmodule KyuubikiWeb.WorkflowBranchRecoveryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowReportingContract, as: Fixture
  alias KyuubikiWeb.{WorkflowGraphResponse, WorkflowGraphRunner, WorkflowOperatorRuntime}

  @policies Path.expand("../../../../tests/fixtures/workflow-recovery-policies.json", __DIR__)
            |> File.read!()
            |> Jason.decode!()

  for policy <- @policies do
    @policy policy
    test "shared recovery policy: #{policy["id"]}" do
      for {bad, node, path} <- Fixture.failures(Fixture.request()),
          reverse <- [false, true] do
        request =
          bad
          |> Fixture.node(
            node,
            &Map.update!(&1, "config", fn c -> Map.merge(c, @policy["config"]) end)
          )
          |> maybe_reverse(reverse)

        if @policy["recover"] do
          assert {:ok, result} = Engine.run_workflow_graph(request)
          assert result["failed_nodes"] == [node]
          assert [%{"node_id" => ^node, "error_message" => reason}] = result["node_failures"]
          assert reason =~ path
          assert result["performance"]["failed_node_count"] == 1
          assert result["artifacts"]["raw.payload"] == request["input_artifacts"]["input"]
          assert length(result["node_runs"]) == length(request["graph"]["nodes"])
          refute Map.has_key?(result["artifacts"], "output.summary")
          refute Map.has_key?(result["artifacts"], "#{node}.summary")
          refute node in result["skipped_nodes"]
          refute node in result["completed_nodes"]
          trace = Enum.find(result["node_runs"], &(&1["node_id"] == node))
          assert trace["status"] == "failed"
          assert trace["produced_artifacts"] == []
          assert trace["error_message"] == reason
          refute Enum.any?(result["artifact_lineage"], &(&1["node_id"] == node))
          assert "output" in result["skipped_nodes"]
        else
          assert {:error, {:workflow_node_error, ^node, reason}} =
                   Engine.run_workflow_graph(request)

          assert reason =~ Map.get(@policy, "error_path", path)
        end
      end
    end
  end

  test "compact and custom responses cannot discard failure receipts" do
    {bad, node, path} = hd(Fixture.failures(Fixture.request()))
    bad = Fixture.node(bad, node, &put_in(&1, ["config", "on_error"], "skip"))

    for options <- [%{"response_mode" => "compact"}, %{"include_node_runs" => false}] do
      assert {:ok, result} = Engine.run_workflow_graph(Map.put(bad, "response_options", options))
      assert result["failed_nodes"] == [node]
      assert [failure] = result["node_failures"]
      assert failure["error_message"] =~ path
      refute Map.has_key?(result, "node_runs")
      assert result["performance"]["node_kind_breakdown"]["extract"]["failed_count"] == 1
    end

    assert {:ok, clean} = Engine.run_workflow_graph(Fixture.request())
    assert clean["failed_nodes"] == []
    assert clean["node_failures"] == []
    assert clean["performance"]["failed_node_count"] == 0
    assert Map.has_key?(clean["artifacts"], "output.summary")
  end

  test "invalid recovery settings are rejected before any operator executes even on valid input" do
    for policy <- @policies, Map.has_key?(policy, "error_path") do
      request =
        Fixture.request()
        |> Fixture.node(
          "guard",
          &Map.update!(&1, "config", fn c -> Map.merge(c, policy["config"]) end)
        )

      callback = fn _, _, _ -> flunk("invalid recovery must be rejected before execution") end

      assert {:error, {:workflow_node_error, "guard", reason}} =
               run(request, execute_extract: callback)

      assert reason =~ policy["error_path"]
    end
  end

  test "missing recoverable input resolves all blocked descendants once without artifacts" do
    request =
      Fixture.request()
      |> update_in(["input_artifacts"], &Map.delete(&1, "input"))
      |> Fixture.node("input", &Map.put(&1, "config", %{"on_error" => "skip"}))
      |> maybe_reverse(true)

    assert {:ok, result} = Engine.run_workflow_graph(request)
    assert result["failed_nodes"] == ["input"]
    assert result["completed_nodes"] == ["extra"]
    assert length(result["skipped_nodes"]) == 7
    assert result["artifacts"] == %{"extra.payload" => request["input_artifacts"]["extra"]}
    assert hd(result["node_failures"])["error_message"] == "missing_input_artifact"
  end

  test "automatic compact mode retains failures for a 256-node graph" do
    {bad, id, path} = hd(Fixture.failures(Fixture.request()))

    request =
      bad
      |> Fixture.node(id, &put_in(&1, ["config", "on_error"], "skip"))
      |> update_in(["graph", "nodes"], fn nodes ->
        nodes ++
          for(i <- 1..247, do: %{"id" => "independent_#{i}", "kind" => "output", "inputs" => []})
      end)

    assert {:ok, result} = Engine.run_workflow_graph(request)
    assert result["failed_nodes"] == [id]
    assert hd(result["node_failures"])["error_message"] =~ path
    refute Map.has_key?(result, "node_runs")
    assert length(result["completed_nodes"]) == 250
  end

  test "partial-input fallback waits for dependencies and respects declared edge order" do
    for reverse <- [false, true], failed <- [false, true] do
      request = fallback_request(failed) |> maybe_reverse(reverse)
      assert {:ok, result} = Engine.run_workflow_graph(request)
      expected = request["input_artifacts"][if(failed, do: "extra", else: "input")]
      assert result["artifacts"]["raw.payload"] == expected
      assert result["failed_nodes"] == if(failed, do: ["primary"], else: [])
      trace = Enum.find(result["node_runs"], &(&1["node_id"] == "fallback"))

      assert trace["consumed_artifacts"] == [
               if(failed, do: "extra.payload", else: "primary.payload")
             ]
    end
  end

  test "all fallback inputs failed produces a skipped fallback rather than a fabricated output" do
    request =
      fallback_request(true)
      |> update_in(["input_artifacts"], &Map.delete(&1, "extra"))
      |> Fixture.node("extra", &Map.put(&1, "config", %{"on_error" => "skip"}))

    assert {:ok, result} = Engine.run_workflow_graph(request)
    assert Enum.sort(result["failed_nodes"]) == ["extra", "primary"]
    assert Enum.sort(result["skipped_nodes"]) == ["fallback", "raw"]
    refute Map.has_key?(result["artifacts"], "raw.payload")
  end

  test "progress includes every terminal node without counting failures as completed" do
    {bad, node, _} = hd(Fixture.failures(Fixture.request()))
    bad = Fixture.node(bad, node, &put_in(&1, ["config", "on_error"], "skip"))
    assert {:ok, result} = run(bad, progress_callback: &send(self(), {:progress, &1}))

    events =
      for _ <- bad["graph"]["nodes"] do
        assert_receive {:progress, event}
        event
      end

    assert Enum.map(events, & &1["resolved_nodes"]) == Enum.to_list(1..9)
    assert length(Enum.uniq_by(events, & &1["node_id"])) == 9
    assert Enum.find(events, &(&1["node_id"] == node))["status"] == "failed"
    last = List.last(events)
    assert last["completed_nodes"] == length(result["completed_nodes"])
    assert last["skipped_nodes"] == length(result["skipped_nodes"])
    assert last["failed_nodes"] == 1
    assert last["resolved_nodes"] == last["total_nodes"]
  end

  test "recoverable callbacks execute once and exceptions or cancellation never become skips" do
    request =
      Fixture.request() |> Fixture.node("field", &put_in(&1, ["config", "on_error"], "skip"))

    fail = fn _, _, _ ->
      send(self(), :called)
      {:error, :expected_failure}
    end

    assert {:ok, result} = run(request, execute_extract: fail)
    assert result["failed_nodes"] == ["field"]
    assert_receive :called
    refute_receive :called

    assert_raise RuntimeError, "unexpected bug", fn ->
      run(request, execute_extract: fn _, _, _ -> raise "unexpected bug" end)
    end

    assert catch_exit(run(request, execute_extract: fn _, _, _ -> exit(:operator_lost) end)) ==
             :operator_lost

    for phase <- ["failed", "skipped"] do
      callback = fn event ->
        if event["status"] == phase, do: throw({:workflow_cancelled, event["node_id"]})
      end

      assert {:error, {:workflow_cancelled, _}} =
               run(request, execute_extract: fail, progress_callback: callback)
    end
  end

  defp run(request, opts) do
    defaults = [
      execute_solve: fn _, _, _ -> flunk("no solver expected") end,
      execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
      execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
      execute_export: &WorkflowOperatorRuntime.run_export_operator/3,
      result_options: WorkflowGraphResponse.normalize_options(nil)
    ]

    WorkflowGraphRunner.run(
      request["graph"],
      request["input_artifacts"],
      Keyword.merge(defaults, opts)
    )
  end

  defp maybe_reverse(request, false), do: request
  defp maybe_reverse(request, true), do: update_in(request, ["graph", "nodes"], &Enum.reverse/1)

  defp fallback_request(failed) do
    request = Fixture.request()
    [input, extra] = Enum.take(request["graph"]["nodes"], 2)
    port = %{"id" => "payload", "artifact_type" => "artifact/json"}

    primary = %{
      "id" => "primary",
      "kind" => "condition",
      "inputs" => [port],
      "outputs" => [port],
      "config" => %{
        "on_error" => "skip",
        "predicate" => %{"operator" => if(failed, do: "unsupported", else: "truthy")}
      }
    }

    fallback = %{
      "id" => "fallback",
      "kind" => "transform",
      "operator_id" => "transform.first_available",
      "inputs" => [%{port | "id" => "primary"}, %{port | "id" => "backup"}],
      "outputs" => [port]
    }

    raw = %{"id" => "raw", "kind" => "output", "inputs" => [port], "outputs" => []}

    edges =
      for {from, to} <- [
            {"primary", "fallback"},
            {"extra", "fallback"},
            {"input", "primary"},
            {"fallback", "raw"}
          ] do
        %{
          "id" => "#{from}-#{to}",
          "from" => %{"node" => from, "port" => "payload"},
          "to" => %{
            "node" => to,
            "port" =>
              if(to == "fallback",
                do: if(from == "primary", do: "primary", else: "backup"),
                else: "payload"
              )
          },
          "artifact_type" => "artifact/json"
        }
      end

    put_in(
      request["graph"],
      Map.merge(request["graph"], %{
        "nodes" => [input, extra, fallback, raw, primary],
        "edges" => edges,
        "output_nodes" => ["raw"]
      })
    )
  end
end
