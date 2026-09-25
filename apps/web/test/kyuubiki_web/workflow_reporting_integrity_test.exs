defmodule KyuubikiWeb.WorkflowReportingIntegrityTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.{Engine, OperatorTaskExecutor, OperatorTaskIR}
  alias KyuubikiWeb.{WorkflowOperatorRuntime, WorkflowReportingRuntime}
  alias KyuubikiWeb.TestSupport.WorkflowReportingContract, as: Fixture

  test "report graph fails at corrupt evidence and corrected replay produces the complete report" do
    for hotspots <- [false, true] do
      clean = Fixture.request(hotspots)

      for {bad, node, path} <- Fixture.failures(clean) do
        assert {:error, {:workflow_node_error, ^node, reason}} = Engine.run_workflow_graph(bad)
        assert reason =~ path
        assert {:ok, result} = Engine.run_workflow_graph(clean)
        assert result["artifacts"]["raw.payload"] == clean["input_artifacts"]["input"]
        report = Jason.decode!(result["artifacts"]["output.summary"]["content"])
        assert report["report_guard_status"] == "pass"
        assert report["guard_payload"]["guard_checked_rule_count"] == 1
        assert report["bundle_total_node_count"] == nil
      end
    end
  end

  test "valid threshold violations produce blocked reports not execution failures" do
    for hotspots <- [false, true] do
      request =
        Fixture.request(hotspots)
        |> Fixture.node("guard", &put_in(&1, ["config", "rules", Access.at(0), "threshold"], 2))

      assert {:ok, result} = Engine.run_workflow_graph(request)
      report = Jason.decode!(result["artifacts"]["output.summary"]["content"])
      assert report["report_guard_status"] == "block"
      assert report["guard_payload"]["guard_passed"] == false
    end
  end

  test "operator task IR cannot bypass reporting validation" do
    for {operator, input, config, path} <- [
          {"extract.field_statistics", %{"nodes" => [%{"v" => 1}, %{}]}, %{"field" => "v"},
           "nodes[1].v"},
          {"extract.field_hotspots", %{"elements" => [%{"v" => 1}, %{}]},
           %{"field" => "v", "threshold" => 0}, "elements[1].v"},
          {"transform.compose_diagnostics_bundle",
           %{"a" => %{"metric" => 2, "converged" => false}}, %{"include_non_diagnostics" => true},
           "payload.a.converged"},
          {"transform.evaluate_diagnostics_bundle_guard", %{},
           %{"rules" => [%{"field" => "missing", "threshold" => 3}]}, "payload.missing"}
        ] do
      assert {:ok, task} = OperatorTaskIR.build(operator, input, config)
      assert {:error, reason} = OperatorTaskExecutor.execute(task)
      assert reason =~ path
    end

    assert {:ok, task} =
             OperatorTaskIR.build("extract.field_statistics", %{"nodes" => [%{"v" => 0}]}, %{
               "field" => "v"
             })

    assert {:ok, result} = OperatorTaskExecutor.execute(task)
    assert result["v_mean"] == 0.0
  end

  test "hotspot samples remain bounded while validating the entire source" do
    rows = for i <- 1..257, do: %{"id" => i, "v" => i}
    config = %{"field" => "v", "threshold" => 0, "sample_limit" => 999}

    assert {:ok, result} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_hotspots",
               %{"elements" => rows},
               config
             )

    assert result["v_hotspot_count"] == 257
    assert length(result["v_hotspot_ids"]) == 257
    assert length(result["v_hotspot_samples"]) == 32
    assert Enum.map(result["v_hotspot_samples"], & &1["id"]) == Enum.to_list(257..226//-1)
    bad = List.replace_at(rows, 256, %{"v" => nil})

    assert {:error, reason} =
             WorkflowOperatorRuntime.run_extract_operator(
               "extract.field_hotspots",
               %{"elements" => bad},
               config
             )

    assert reason =~ "elements[256].v"
  end

  test "unrepresentable integers and malformed configuration return errors without crashing" do
    huge = Integer.pow(10, 400)

    assert {:error, reason} =
             WorkflowReportingRuntime.extract_field_statistics(%{"nodes" => [%{"v" => huge}]}, %{
               "field" => "v"
             })

    assert reason =~ "nodes[0].v"

    for bad <- [false, [], 1, "config"] do
      assert {:error, _} = WorkflowReportingRuntime.extract_field_statistics(%{}, bad)

      assert {:error, _} =
               WorkflowOperatorRuntime.run_transform_operator(
                 "transform.compose_diagnostics_bundle",
                 %{},
                 bad
               )
    end
  end

  test "unknown counts are visible in Markdown rather than blank or zero" do
    assert {:ok, bundle} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_diagnostics_bundle",
               %{"a" => %{"metric" => 2}},
               %{"include_non_diagnostics" => true}
             )

    assert {:ok, report} =
             WorkflowReportingRuntime.export_diagnostics_bundle_markdown(bundle, %{})

    assert report["content"] =~ "- Total Nodes: null"
    assert report["content"] =~ "- Total Elements: null"
    assert report["content"] =~ "- Nodes: null"
    assert report["content"] =~ "- Elements: null"
  end
end
