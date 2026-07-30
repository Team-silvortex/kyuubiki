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
          "transform.score_modal_quality",
          "transform.score_dynamic_quality"
        ] do
      assert {:ok, %{"operator" => operator}} = WorkflowOperatorCatalog.fetch(operator_id)
      assert operator["kind"] == "transform"
      assert "quality" in operator["capability_tags"]
      assert "headless_safe" in operator["capability_tags"]
    end
  end

  test "scores dynamic quality through web runtime" do
    assert {:ok, quality} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_dynamic_quality",
               %{
                 "peak_frequency_hz" => 32.0,
                 "max_displacement" => 0.012,
                 "max_acceleration" => 180.0,
                 "max_force" => 3200.0
               },
               %{
                 "targets" => %{
                   "peak_frequency_hz" => 25.0,
                   "max_displacement" => 0.02,
                   "max_acceleration" => 250.0,
                   "max_force" => 5000.0
                 },
                 "max_ready_score" => 8.0
               }
             )

    assert quality["dynamic_quality_contract"] == "kyuubiki.dynamic_quality_score/v1"
    assert quality["dynamic_quality_ready"] == true
    assert quality["dynamic_quality_missing_metric_count"] == 0
    assert quality["dynamic_quality_watch_count"] == 0
    assert quality["dynamic_quality_term_count"] == 4
    assert quality["dynamic_quality_peak_frequency_hz"] == 32.0
    assert quality["dynamic_quality_blocking_terms"] == []
  end

  test "derives dynamic quality from harmonic frequency aliases through web runtime" do
    assert {:ok, quality} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_dynamic_quality",
               %{
                 "frequencies" => [
                   %{
                     "freq_hz" => 5.0,
                     "displacement_amplitude" => 0.01,
                     "acceleration_amplitude" => 25.0,
                     "force_amplitude" => 100.0
                   },
                   %{
                     "freq_hz" => 12.0,
                     "displacement_amplitude" => 0.03,
                     "acceleration_amplitude" => 180.0,
                     "force_amplitude" => 450.0
                   },
                   %{
                     "freq_hz" => 30.0,
                     "displacement_amplitude" => 0.02,
                     "acceleration_amplitude" => 220.0,
                     "force_amplitude" => 390.0
                   }
                 ]
               },
               %{
                 "targets" => %{
                   "peak_frequency_hz" => 10.0,
                   "max_displacement" => 0.05,
                   "max_acceleration" => 300.0,
                   "max_force" => 1000.0
                 }
               }
             )

    assert quality["dynamic_quality_ready"] == true
    assert quality["dynamic_quality_missing_metric_count"] == 0
    assert quality["dynamic_quality_peak_frequency_hz"] == 12.0
    assert quality["dynamic_quality_max_displacement"] == 0.03
    assert quality["dynamic_quality_max_force"] == 450.0
  end

  test "derives dynamic quality from transient node aliases and enabled terms" do
    assert {:ok, quality} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_dynamic_quality",
               %{
                 "nodes" => [
                   %{"id" => "fixed", "ux" => 0.0, "vx" => 0.0, "ax" => 0.0},
                   %{"id" => "tip", "ux" => -0.012, "vx" => 0.8, "ax" => -12.0}
                 ],
                 "max_force" => 150.0
               },
               %{
                 "enabled_terms" => [
                   "max_displacement",
                   "max_velocity",
                   "max_acceleration",
                   "max_force"
                 ],
                 "targets" => %{
                   "max_displacement" => 0.02,
                   "max_velocity" => 1.0,
                   "max_acceleration" => 20.0,
                   "max_force" => 300.0
                 }
               }
             )

    assert quality["dynamic_quality_ready"] == true
    assert quality["dynamic_quality_missing_metric_count"] == 0
    assert quality["dynamic_quality_term_count"] == 4
    assert quality["dynamic_quality_max_velocity"] == 0.8
    assert quality_term_value(quality, "max_velocity") == 0.8
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
    assert Enum.any?(quality["structural_quality_blocking_terms"], &(&1["field"] == "mass"))
    assert is_map(quality["structural_quality_dominant_term"])
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

  test "scores domain quality aliases aligned with engine contracts" do
    cases = [
      {"structural", "transform.score_structural_quality",
       %{
         "peak_displacement" => 0.01,
         "von_mises_peak" => 120.0,
         "structure_mass" => 10.0,
         "stability_margin" => 1.4
       }},
      {"thermal", "transform.score_thermal_quality",
       %{
         "temperature_max" => 70.0,
         "temperature_min" => 20.0,
         "max_heat_flux" => 10.0,
         "thermal_stress_peak" => 120.0
       }},
      {"electrostatic", "transform.score_electrostatic_quality",
       %{
         "peak_electric_field" => 5.0,
         "peak_energy_density" => 0.4,
         "potential_max" => 4.0,
         "potential_min" => 1.0
       }},
      {"magnetostatic", "transform.score_magnetostatic_quality",
       %{
         "h_peak" => 6.0,
         "b_peak" => 8.0,
         "magnetic_energy_density_peak" => 2.0,
         "current_density_total" => 4.0
       }},
      {"acoustic", "transform.score_acoustic_quality",
       %{
         "peak_spl_db" => 70.0,
         "peak_acoustic_intensity" => 0.1,
         "peak_pressure" => 0.5,
         "damping_loss_total" => 0.2
       }},
      {"modal", "transform.score_modal_quality",
       %{
         "modes" => [
           %{"frequency_hz" => 40.0, "participation_norm" => 1.0},
           %{"frequency_hz" => 140.0}
         ],
         "modal_mass_total" => 10.0
       }},
      {"cfd", "transform.score_cfd_quality",
       %{
         "max_divergence_error" => 0.02,
         "re_peak" => 5.0,
         "dissipation_total" => 0.5,
         "velocity_span" => 1.0,
         "pressure_span" => 2.0
       }},
      {"transport", "transform.score_transport_quality",
       %{
         "peak_transport_flux" => 0.75,
         "peak_peclet" => 100.0,
         "concentration_max" => 1.0,
         "concentration_min" => 0.2,
         "net_source" => 1.0
       }}
    ]

    for {domain, operator_id, payload} <- cases do
      assert {:ok, quality} =
               WorkflowOperatorRuntime.run_transform_operator(operator_id, payload, %{})

      assert quality["#{domain}_quality_missing_metric_count"] == 0
    end
  end

  test "scores optional energy terms and emits domain metric mirrors" do
    cases = [
      {
        "transform.score_thermal_quality",
        "thermal",
        "thermal_total_energy",
        "thermal_quality_total_energy",
        %{"total_thermal_energy" => 50.0},
        50.0
      },
      {
        "transform.score_electrostatic_quality",
        "electrostatic",
        "electrostatic_total_stored_energy",
        "electrostatic_quality_total_energy",
        %{"electric_total_energy" => 4.0},
        4.0
      },
      {
        "transform.score_magnetostatic_quality",
        "magnetostatic",
        "magnetostatic_total_stored_energy",
        "magnetostatic_quality_total_energy",
        %{"magnetic_total_energy" => 3.0},
        3.0
      }
    ]

    for {operator_id, domain, term, mirror_field, payload, expected} <- cases do
      assert {:ok, quality} =
               WorkflowOperatorRuntime.run_transform_operator(operator_id, payload, %{
                 "enabled_terms" => [term],
                 "targets" => %{term => 100.0},
                 "max_ready_score" => 2.0
               })

      assert quality["#{domain}_quality_missing_metric_count"] == 0
      assert quality["#{domain}_quality_term_count"] == 1
      assert_in_delta quality[mirror_field], expected, 1.0e-9

      assert %{"value" => value} =
               Enum.find(quality["#{domain}_quality_terms"], &(&1["field"] == term))

      assert_in_delta value, expected, 1.0e-9
    end
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

  defp quality_term_value(quality, field) do
    quality["dynamic_quality_terms"]
    |> Enum.find(&(&1["field"] == field))
    |> Map.fetch!("value")
  end
end
