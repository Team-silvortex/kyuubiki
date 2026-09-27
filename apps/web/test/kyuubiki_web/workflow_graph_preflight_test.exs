defmodule KyuubikiWeb.WorkflowGraphPreflightTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.{WorkflowGraphPreflight, WorkflowGraphRunner}
  alias KyuubikiWeb.Orchestra.Engine
  alias KyuubikiWeb.TestSupport.WorkflowPreflightContract, as: Fixture

  for test_case <- Fixture.cases() do
    @test_case test_case
    test "shared graph contract: #{test_case["id"]}" do
      request = Fixture.request(@test_case)

      for reverse <- [false, true] do
        graph =
          update_in(request["graph"], ["nodes"], fn nodes ->
            if reverse and is_list(nodes), do: Enum.reverse(nodes), else: nodes
          end)

        result = WorkflowGraphPreflight.validate(graph, request["input_artifacts"])

        if @test_case["accept"] do
          assert result == :ok
        else
          assert {:error, {:invalid_workflow_graph, reason}} = result
          assert reason =~ @test_case["error"]
          callback = fn _, _, _ -> flunk("bad graph executed an operator") end

          assert {:error, _} =
                   WorkflowGraphRunner.run(graph, request["input_artifacts"],
                     execute_solve: callback,
                     execute_transform: callback,
                     execute_extract: callback,
                     execute_export: callback,
                     progress_callback: fn _ -> flunk("bad graph emitted execution progress") end
                   )

          assert {:error, _} = Engine.run_workflow_graph(%{request | "graph" => graph})
        end
      end
    end
  end

  test "preflight stays independent of the callback's operator implementation" do
    request =
      put_in(Fixture.request(), ["graph", "nodes", Access.at(1), "operator_id"], "custom.pick")

    callback = fn "custom.pick", payload, _node -> {:ok, payload} end
    unused = fn _, _, _ -> flunk("unexpected callback") end

    assert {:ok, result} =
             WorkflowGraphRunner.run(request["graph"], request["input_artifacts"],
               execute_solve: unused,
               execute_transform: callback,
               execute_extract: unused,
               execute_export: unused
             )

    assert result["artifacts"]["output.payload"] == %{"value" => 42}
  end

  test "native improper graph lists return contract errors instead of raising" do
    request = Fixture.request()

    for path <- [
          ["nodes"],
          ["edges"],
          ["entry_nodes"],
          ["output_nodes"],
          ["nodes", Access.at(1), "inputs"],
          ["nodes", Access.at(1), "outputs"]
        ] do
      graph = update_in(request["graph"], path, &(&1 ++ :invalid_tail))

      assert {:error, {:invalid_workflow_graph, _}} =
               WorkflowGraphPreflight.validate(graph, request["input_artifacts"])
    end
  end

  test "valid graph executes in either declaration order after rejected requests" do
    for reverse <- [false, true] do
      request =
        update_in(Fixture.request(), ["graph", "nodes"], fn nodes ->
          if reverse, do: Enum.reverse(nodes), else: nodes
        end)

      assert {:ok, result} = Engine.run_workflow_graph(request)
      assert result["artifacts"]["output.payload"] == %{"value" => 42}
      assert result["failed_nodes"] == []
    end
  end
end
