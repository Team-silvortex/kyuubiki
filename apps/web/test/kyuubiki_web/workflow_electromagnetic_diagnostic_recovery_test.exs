defmodule KyuubikiWeb.WorkflowElectromagneticDiagnosticRecoveryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  for {domain, scalar, x, y, magnitude} <- [
        {"electrostatic", "potential", "electric_field_x", "electric_field_y",
         "electric_field_magnitude"},
        {"magnetostatic", "vector_potential", "magnetic_field_strength_x",
         "magnetic_field_strength_y", "magnetic_field_strength_magnitude"}
      ],
      policy <- ["fail", "skip"] do
    @domain domain
    @scalar scalar
    @magnitude magnitude
    @policy policy
    @valid %{
      "nodes" => [%{scalar => 1}, %{scalar => 3}],
      "elements" => [%{x => 3, y => 4}, %{x => 5, y => 12}]
    }

    test "#{domain}/#{policy}: corrupt diagnostics cannot reach decisions or export; fresh runs recover" do
      graph = graph(@domain, @policy)
      ref = make_ref()
      owner = self()

      options = [
        execute_solve: fn _, _, _ -> flunk("no solver should run in this diagnostic test") end,
        execute_extract: &Runtime.run_extract_operator/3,
        execute_transform: fn id, input, config ->
          send(owner, {ref, :decision})
          Runtime.run_transform_operator(id, input, config)
        end,
        execute_export: fn id, input, config ->
          send(owner, {ref, :export})
          Runtime.run_export_operator(id, input, config)
        end
      ]

      for {invalid, path} <- [
            {put_in(@valid, ["nodes", Access.at(1)], %{}), "payload.nodes[1].#{@scalar}"},
            {put_in(@valid, ["elements", Access.at(1), @magnitude], nil),
             "payload.elements[1].#{@magnitude}"},
            {Map.put(@valid, "converged", false), "payload.converged=false"},
            {%{"nodes" => [], "elements" => []}, "diagnostic fields"}
          ] do
        result = WorkflowGraphRunner.run(graph, %{"input" => invalid}, options)

        if @policy == "fail" do
          assert {:error, {:workflow_node_error, "diagnostics", reason}} = result
          assert reason =~ path
        else
          assert {:ok, rejected} = result
          assert "diagnostics" in rejected["failed_nodes"]
          refute "guard" in rejected["completed_nodes"]
          refute "export" in rejected["completed_nodes"]
          refute Map.has_key?(rejected["artifacts"], "diagnostics.result")
          refute Map.has_key?(rejected["artifacts"], "export.json")
        end

        refute_received {^ref, :decision}
        refute_received {^ref, :export}

        assert {:ok, recovered} = WorkflowGraphRunner.run(graph, %{"input" => @valid}, options)
        assert recovered["failed_nodes"] == []
        assert recovered["completed_nodes"] == ~w(input diagnostics guard export output)
        assert recovered["artifacts"]["diagnostics.result"]["diagnostic_node_count"] == 2
        summary = Jason.decode!(recovered["artifacts"]["export.json"]["content"])
        assert summary["guard_status"] == "block"
        assert summary["guard_checked_rule_count"] == 1
        assert_received {^ref, :decision}
        assert_received {^ref, :export}
      end
    end
  end

  defp graph(domain, policy) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.#{domain}-diagnostic-recovery",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [port("result")]},
        %{
          "id" => "diagnostics",
          "kind" => "extract",
          "operator_id" => "extract.#{domain}_result_diagnostics",
          "config" => %{"on_error" => policy},
          "inputs" => [port("payload")],
          "outputs" => [port("result")]
        },
        %{
          "id" => "guard",
          "kind" => "transform",
          "operator_id" => "transform.evaluate_#{domain}_guard",
          "config" => %{
            "rules" => [
              %{
                "field" => "#{domain}_field_peak_magnitude",
                "threshold" => 10,
                "severity" => "block"
              }
            ]
          },
          "inputs" => [port("payload")],
          "outputs" => [port("result")]
        },
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
      ],
      "edges" => [
        edge("e0", "input", "result", "diagnostics", "payload"),
        edge("e1", "diagnostics", "result", "guard", "payload"),
        edge("e2", "guard", "result", "export", "summary"),
        edge("e3", "export", "json", "output", "json", "export/json")
      ]
    }
  end

  defp port(id, type \\ "report/summary"), do: %{"id" => id, "artifact_type" => type}

  defp edge(id, from, from_port, to, to_port, type \\ "report/summary"),
    do: %{
      "id" => id,
      "from" => %{"node" => from, "port" => from_port},
      "to" => %{"node" => to, "port" => to_port},
      "artifact_type" => type
    }
end
