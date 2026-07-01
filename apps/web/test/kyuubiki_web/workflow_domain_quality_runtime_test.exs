defmodule KyuubikiWeb.WorkflowDomainQualityRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowGraphRunner
  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes domain quality score transforms" do
    for operator_id <- [
          "transform.score_thermal_quality",
          "transform.score_structural_quality",
          "transform.score_electrostatic_quality",
          "transform.score_magnetostatic_quality",
          "transform.score_cfd_quality",
          "transform.score_transport_quality",
          "transform.score_acoustic_quality",
          "transform.score_modal_quality"
        ] do
      assert {:ok, %{"operator" => operator}} = WorkflowOperatorCatalog.fetch(operator_id)
      assert operator["kind"] == "transform"
      assert "quality" in operator["capability_tags"]
      assert "headless_safe" in operator["capability_tags"]
    end
  end

  test "scores thermal quality with configurable targets and weights" do
    assert {:ok, quality} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_thermal_quality",
               %{
                 "thermal_temperature_max" => 60.0,
                 "thermo_temperature_delta_max" => 40.0,
                 "thermal_flux_peak_magnitude" => 10.0,
                 "thermo_stress_peak" => 125.0
               },
               %{"targets" => %{"thermal_temperature_max" => 120.0}, "max_ready_score" => 8.0}
             )

    assert quality["thermal_quality_contract"] == "kyuubiki.thermal_quality_score/v1"
    assert quality["thermal_quality_score"] == 4.0
    assert quality["thermal_quality_grade"] == "good"
    assert quality["thermal_quality_ready"] == true
    assert quality["thermal_quality_missing_metric_count"] == 0
  end

  test "blocks domain quality when required metrics are missing" do
    assert {:ok, quality} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_structural_quality",
               %{"max_displacement" => 0.01, "max_stress" => 125.0},
               %{}
             )

    assert quality["structural_quality_ready"] == false
    assert quality["structural_quality_grade"] == "block"
    assert quality["structural_quality_missing_metric_count"] == 2
  end

  test "derives modal and cfd span fields when summaries expose min and max values" do
    assert {:ok, modal_quality} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_modal_quality",
               %{
                 "min_frequency_hz" => 40.0,
                 "max_frequency_hz" => 140.0,
                 "total_mass" => 10.0,
                 "mode_1_participation_norm" => 1.0
               },
               %{}
             )

    assert Enum.any?(modal_quality["modal_quality_terms"], &(&1["field"] == "frequency_span_hz"))
    assert modal_quality["modal_quality_missing_metric_count"] == 0

    assert {:ok, cfd_quality} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_cfd_quality",
               %{
                 "cfd_divergence_error_peak" => 0.02,
                 "cfd_reynolds_number_peak" => 5.0,
                 "cfd_viscous_dissipation_total" => 0.5,
                 "cfd_velocity_min" => -0.5,
                 "cfd_velocity_max" => 0.5,
                 "cfd_pressure_min" => -1.0,
                 "cfd_pressure_max" => 1.0
               },
               %{}
             )

    assert cfd_quality["cfd_quality_missing_metric_count"] == 0

    assert Enum.find(cfd_quality["cfd_quality_terms"], &(&1["field"] == "cfd_velocity_span"))[
             "value"
           ] == 1.0
  end

  test "runs domain quality scores into a composite objective inside graph runner" do
    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.web-domain-quality-objective-json",
      "entry_nodes" => ["thermal_summary", "transport_summary"],
      "output_nodes" => ["objective_output"],
      "nodes" => [
        input_node("thermal_summary", "thermal_summary"),
        input_node("transport_summary", "transport_summary"),
        transform_node(
          "thermal_quality",
          "transform.score_thermal_quality",
          "thermal_summary",
          "quality"
        ),
        transform_node(
          "transport_quality",
          "transform.score_transport_quality",
          "transport_summary",
          "quality"
        ),
        %{
          "id" => "compose_objective",
          "kind" => "transform",
          "operator_id" => "transform.compose_quality_objective",
          "config" => %{"weights" => %{"thermal" => 2.0, "transport" => 1.0}},
          "inputs" => [
            port("thermal", "report/summary", "thermal_quality"),
            port("transport", "report/summary", "transport_quality")
          ],
          "outputs" => [port("objective", "report/summary", "composite_quality_objective")]
        },
        %{
          "id" => "objective_output",
          "kind" => "output",
          "inputs" => [port("objective", "report/summary", "composite_quality_objective")],
          "outputs" => []
        }
      ],
      "edges" => [
        edge("e0", "thermal_summary", "summary", "thermal_quality", "summary", "thermal_summary"),
        edge(
          "e1",
          "transport_summary",
          "summary",
          "transport_quality",
          "summary",
          "transport_summary"
        ),
        edge(
          "e2",
          "thermal_quality",
          "quality",
          "compose_objective",
          "thermal",
          "thermal_quality"
        ),
        edge(
          "e3",
          "transport_quality",
          "quality",
          "compose_objective",
          "transport",
          "transport_quality"
        ),
        edge(
          "e4",
          "compose_objective",
          "objective",
          "objective_output",
          "objective",
          "composite_quality_objective"
        )
      ]
    }

    assert {:ok, run} =
             WorkflowGraphRunner.run(graph, graph_input_artifacts(),
               execute_solve: &WorkflowOperatorRuntime.run_solve_operator/3,
               execute_transform: &WorkflowOperatorRuntime.run_transform_operator/3,
               execute_extract: &WorkflowOperatorRuntime.run_extract_operator/3,
               execute_export: &WorkflowOperatorRuntime.run_export_operator/3
             )

    objective = run["artifacts"]["compose_objective.objective"]
    assert objective["composite_quality_contract"] == "kyuubiki.composite_quality_objective/v1"
    assert objective["composite_quality_ready"] == true
    assert objective["composite_quality_term_count"] == 2
  end

  defp graph_input_artifacts do
    %{
      "thermal_summary" => %{
        "thermal_temperature_max" => 60.0,
        "thermo_temperature_delta_max" => 40.0,
        "thermal_flux_peak_magnitude" => 10.0,
        "thermo_stress_peak" => 125.0
      },
      "transport_summary" => %{
        "transport_total_flux_peak_magnitude" => 0.75,
        "transport_peclet_peak" => 100.0,
        "transport_concentration_span" => 0.5,
        "transport_source_sum" => 1.0
      }
    }
  end

  defp input_node(id, dataset_value) do
    %{
      "id" => id,
      "kind" => "input",
      "outputs" => [port("summary", "report/summary", dataset_value)]
    }
  end

  defp transform_node(id, operator_id, input_dataset, output_port) do
    %{
      "id" => id,
      "kind" => "transform",
      "operator_id" => operator_id,
      "config" => %{},
      "inputs" => [port("summary", "report/summary", input_dataset)],
      "outputs" => [port(output_port, "report/summary", id)]
    }
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
