defmodule KyuubikiWeb.WorkflowDomainDecisionRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes generic domain guard and benchmark transforms" do
    for operator_id <- [
          "transform.evaluate_structural_guard",
          "transform.benchmark_structural_pair",
          "transform.evaluate_acoustic_guard",
          "transform.benchmark_acoustic_pair",
          "transform.evaluate_modal_guard",
          "transform.benchmark_modal_pair",
          "transform.evaluate_dynamic_guard",
          "transform.benchmark_dynamic_pair",
          "transform.evaluate_transport_guard",
          "transform.benchmark_transport_pair"
        ] do
      assert {:ok, %{"operator" => operator}} = WorkflowOperatorCatalog.fetch(operator_id)
      assert operator["kind"] == "transform"
      assert "decision" in operator["capability_tags"]
      assert "headless_safe" in operator["capability_tags"]
    end
  end

  test "evaluates and benchmarks dynamic response decision transforms" do
    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_dynamic_guard",
               %{"peak_frequency_hz" => 18.0, "max_displacement" => 0.035},
               %{
                 "rules" => [
                   %{
                     "field" => "peak_frequency_hz",
                     "comparison" => "lt",
                     "threshold" => 20.0,
                     "severity" => "warn"
                   },
                   %{
                     "field" => "max_displacement",
                     "comparison" => "gt",
                     "threshold" => 0.02,
                     "severity" => "block"
                   }
                 ]
               }
             )

    assert guard["guard_status"] == "block"
    assert guard["guard_warn_count"] == 1
    assert guard["guard_block_count"] == 1

    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_dynamic_pair",
               %{
                 "left" => %{"peak_frequency_hz" => 42.0, "max_displacement" => 0.014},
                 "right" => %{"peak_frequency_hz" => 35.0, "max_displacement" => 0.010}
               },
               %{
                 "criteria" => [
                   %{"field" => "peak_frequency_hz", "goal" => "max", "weight" => 2.0},
                   %{"field" => "max_displacement", "goal" => "min", "weight" => 1.0}
                 ]
               }
             )

    assert benchmark["left_score"] == 2.0
    assert benchmark["right_score"] == 1.0
    assert benchmark["benchmark_winner"] == "left"
  end

  test "evaluates structural guard threshold rules" do
    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_structural_guard",
               %{"max_stress" => 320.0, "max_displacement" => 0.01},
               %{
                 "rules" => [
                   %{"field" => "max_stress", "threshold" => 250.0, "severity" => "block"},
                   %{"field" => "max_displacement", "threshold" => 0.02, "severity" => "warn"}
                 ]
               }
             )

    assert guard["guard_status"] == "block"
    assert guard["guard_passed"] == false
    assert guard["guard_trigger_count"] == 1
    assert guard["guard_block_count"] == 1
  end

  test "evaluates and benchmarks decision transforms through shared metric aliases" do
    assert {:ok, guard} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.evaluate_structural_guard",
               %{"von_mises_peak" => 320.0},
               %{
                 "rules" => [
                   %{"field" => "max_stress", "threshold" => 250.0, "severity" => "block"}
                 ]
               }
             )

    assert guard["guard_status"] == "block"
    assert hd(guard["guard_triggers"])["value"] == 320.0

    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_dynamic_pair",
               %{
                 "left" => %{
                   "frequencies" => [
                     %{"freq_hz" => 10.0, "displacement_amplitude" => 0.01},
                     %{"freq_hz" => 42.0, "displacement_amplitude" => 0.03}
                   ]
                 },
                 "right" => %{
                   "response_peak_frequency_hz" => 35.0
                 }
               },
               %{
                 "criteria" => [
                   %{"field" => "peak_frequency_hz", "goal" => "max", "weight" => 2.0}
                 ]
               }
             )

    assert benchmark["left_score"] == 2.0
    assert benchmark["right_score"] == 0.0
    assert hd(benchmark["benchmark_breakdown"])["left_value"] == 42.0
  end

  test "benchmarks transport candidates with weighted criteria" do
    assert {:ok, benchmark} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.benchmark_transport_pair",
               %{
                 "left" => %{
                   "transport_peclet_peak" => 100.0,
                   "transport_concentration_span" => 0.6
                 },
                 "right" => %{
                   "transport_peclet_peak" => 140.0,
                   "transport_concentration_span" => 0.4
                 }
               },
               %{
                 "left_label" => "baseline",
                 "right_label" => "candidate",
                 "criteria" => [
                   %{"field" => "transport_peclet_peak", "goal" => "min", "weight" => 2.0},
                   %{"field" => "transport_concentration_span", "goal" => "min", "weight" => 1.0}
                 ]
               }
             )

    assert benchmark["baseline_score"] == 2.0
    assert benchmark["candidate_score"] == 1.0
    assert benchmark["benchmark_winner"] == "baseline"
    assert benchmark["benchmark_criteria_count"] == 2
  end

  test "runs a domain guard inside graph runner" do
    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.web-structural-guard-json",
      "entry_nodes" => ["summary"],
      "output_nodes" => ["guard_output"],
      "nodes" => [
        %{
          "id" => "summary",
          "kind" => "input",
          "outputs" => [port("summary", "report/summary", "structural_summary")]
        },
        %{
          "id" => "guard",
          "kind" => "transform",
          "operator_id" => "transform.evaluate_structural_guard",
          "config" => %{"rules" => [%{"field" => "max_stress", "threshold" => 250.0}]},
          "inputs" => [port("summary", "report/summary", "structural_summary")],
          "outputs" => [port("result", "report/summary", "structural_guard")]
        },
        %{
          "id" => "guard_output",
          "kind" => "output",
          "inputs" => [port("result", "report/summary", "structural_guard")],
          "outputs" => []
        }
      ],
      "edges" => [
        edge("e0", "summary", "summary", "guard", "summary", "structural_summary"),
        edge("e1", "guard", "result", "guard_output", "result", "structural_guard")
      ]
    }

    assert {:ok, run} =
             WorkflowGraphRunner.run(graph, %{"summary" => %{"max_stress" => 100.0}},
               execute_solve: &WorkflowOperatorRuntime.run_solve_operator/3,
               execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
               execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
               execute_export: &WorkflowOperatorRuntime.run_export_operator/3
             )

    assert run["artifacts"]["guard.result"]["guard_status"] == "pass"
  end

  defp port(id, artifact_type, dataset_value),
    do: %{"id" => id, "artifact_type" => artifact_type, "dataset_value" => dataset_value}

  defp edge(id, from_node, from_port, to_node, to_port, dataset_value) do
    %{
      "id" => id,
      "from" => %{"node" => from_node, "port" => from_port},
      "to" => %{"node" => to_node, "port" => to_port},
      "artifact_type" => "report/summary",
      "dataset_value" => dataset_value
    }
  end
end
