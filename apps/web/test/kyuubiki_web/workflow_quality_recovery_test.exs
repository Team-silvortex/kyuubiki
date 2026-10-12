defmodule KyuubikiWeb.WorkflowQualityRecoveryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime

  for {domain, field} <- [
        {"structural", "max_displacement"},
        {"thermal", "thermal_temperature_max"},
        {"acoustic", "max_sound_pressure_level_db"},
        {"modal", "total_mass"},
        {"dynamic", "max_displacement"},
        {"electrostatic", "electrostatic_field_peak_magnitude"},
        {"magnetostatic", "magnetostatic_field_peak_magnitude"},
        {"cfd", "cfd_divergence_error_peak"},
        {"transport", "transport_total_flux_peak_magnitude"}
      ],
      policy <- ~w(fail skip) do
    @domain domain
    @field field
    @policy policy

    test "#{domain}/#{policy}: invalid quality cannot reach objective or export and fresh execution recovers" do
      config = %{
        "enabled_terms" => [@field],
        "targets" => %{@field => 1},
        "weights" => %{@field => 1},
        "on_error" => @policy
      }

      valid = %{@field => 0.5}
      owner = self()
      ref = make_ref()

      options = [
        execute_extract: fn _, _, _ ->
          flunk("no extraction belongs in this controlled quality test")
        end,
        execute_solve: fn _, _, _ ->
          flunk("no solver belongs in this controlled quality test")
        end,
        execute_transform: fn operator, payload, config ->
          if operator == "transform.compose_quality_objective", do: send(owner, {ref, :objective})
          Runtime.run_transform_operator(operator, payload, config)
        end,
        execute_export: fn operator, payload, config ->
          send(owner, {ref, :export})
          Runtime.run_export_operator(operator, payload, config)
        end
      ]

      for {payload, bad_config, path} <- [
            {%{@field => nil}, config, "payload.#{@field}"},
            {Map.put(valid, "converged", false), config, "payload.converged=false"},
            {valid, Map.put(config, "enabled_terms", [@field, "unknown"]),
             "config.enabled_terms[1]"},
            {valid, put_in(config, ["weights", @field], -1), "config.weights.#{@field}"}
          ] do
        result =
          WorkflowGraphRunner.run(graph(@domain, bad_config), %{"input" => payload}, options)

        if @policy == "fail" do
          assert {:error, {:workflow_node_error, "quality", reason}} = result
          assert reason =~ path
        else
          assert {:ok, rejected} = result
          assert "quality" in rejected["failed_nodes"]

          for node <- ~w(quality objective export output),
              do: refute(node in rejected["completed_nodes"])

          for artifact <- ~w(quality.result objective.result export.json),
              do: refute(Map.has_key?(rejected["artifacts"], artifact))
        end

        refute_received {^ref, :objective}
        refute_received {^ref, :export}

        assert {:ok, recovered} =
                 WorkflowGraphRunner.run(graph(@domain, config), %{"input" => valid}, options)

        assert recovered["failed_nodes"] == []
        assert recovered["completed_nodes"] == ~w(input quality objective export output)
        summary = Jason.decode!(recovered["artifacts"]["export.json"]["content"])
        assert summary["composite_quality_ready"]
        assert summary["composite_quality_score"] == 0.5
        assert_received {^ref, :objective}
        assert_received {^ref, :export}
      end

      assert {:ok, missing} =
               WorkflowGraphRunner.run(graph(@domain, config), %{"input" => %{}}, options)

      summary = Jason.decode!(missing["artifacts"]["export.json"]["content"])
      refute summary["composite_quality_ready"]
      assert summary["composite_quality_grade"] == "block"
      assert summary["composite_quality_missing_metric_count"] == 1
      assert_received {^ref, :objective}
      assert_received {^ref, :export}
    end
  end

  defp graph(domain, config) do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.#{domain}-quality-recovery",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [port("summary")]},
        %{
          "id" => "quality",
          "kind" => "transform",
          "operator_id" => "transform.score_#{domain}_quality",
          "config" => config,
          "inputs" => [port("payload")],
          "outputs" => [port("result")]
        },
        %{
          "id" => "objective",
          "kind" => "transform",
          "operator_id" => "transform.compose_quality_objective",
          "config" => %{},
          "inputs" => [port(domain)],
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
        edge("e0", "input", "summary", "quality", "payload"),
        edge("e1", "quality", "result", "objective", domain),
        edge("e2", "objective", "result", "export", "summary"),
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
