defmodule KyuubikiWeb.WorkflowMaterialCardRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "validates material cards with provenance units and reliability envelope" do
    assert {:ok, report} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.validate_material_card",
               %{"material_card" => material_card()},
               %{
                 "required_parameters" => ["youngs_modulus", "thermal_conductivity"],
                 "expected_units" => %{
                   "youngs_modulus" => "Pa",
                   "thermal_conductivity" => "W/(m*K)"
                 },
                 "required_temperature_c" => 80.0
               }
             )

    assert report["material_card_preflight_status"] == "pass"
    assert report["material_card_id"] == "al-6061-t6"
    assert report["material_card_parameter_count"] == 2
    assert report["material_card_quality_gates"]["units_valid"] == true
    assert report["material_card_reliability_envelope"]["trust_level"] == "review_ready"
  end

  test "reports material card preflight failures without hiding issue details" do
    invalid_card =
      material_card()
      |> put_in(["parameters", "youngs_modulus", "unit"], "MPa")
      |> update_in(["parameters"], &Map.delete(&1, "thermal_conductivity"))

    assert {:ok, report} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.validate_material_card",
               invalid_card,
               %{
                 "required_parameters" => ["youngs_modulus", "thermal_conductivity"],
                 "expected_units" => %{"youngs_modulus" => "Pa"},
                 "required_temperature_c" => 250.0
               }
             )

    assert report["material_card_preflight_status"] == "fail"
    assert report["material_card_error_count"] == 2
    assert report["material_card_warning_count"] == 1
    assert report["material_card_quality_gates"]["units_valid"] == false

    codes = Enum.map(report["material_card_issues"], & &1["code"])
    assert "missing_required_parameter" in codes
    assert "unit_mismatch" in codes
    assert "temperature_out_of_scope" in codes
  end

  test "summarizes material card batch preflight for candidate screening" do
    invalid_card =
      material_card()
      |> Map.put("material_id", "bad-polymer")
      |> put_in(["parameters", "youngs_modulus", "unit"], "MPa")

    assert {:ok, batch} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.validate_material_card_batch",
               %{"material_cards" => %{"aluminum" => material_card(), "polymer" => invalid_card}},
               %{"expected_units" => %{"youngs_modulus" => "Pa"}}
             )

    assert batch["material_card_batch_count"] == 2
    assert batch["material_card_batch_pass_count"] == 1
    assert batch["material_card_batch_fail_count"] == 1
    assert batch["material_card_batch_usable_count"] == 1
    assert batch["material_card_batch_best_review_ready_id"] == "al-6061-t6"
    assert batch["material_card_batch_issue_counts"] == %{"unit_mismatch" => 1}
  end

  test "builds material candidate summaries from card preflight reports" do
    assert {:ok, batch} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.validate_material_card_batch",
               %{"material_cards" => %{"aluminum" => material_card()}},
               %{"expected_units" => %{"youngs_modulus" => "Pa"}}
             )

    assert {:ok, summaries} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.build_material_card_candidate_summaries",
               batch,
               %{}
             )

    aluminum = summaries["candidates"]["aluminum"]

    assert summaries["material_card_candidate_count"] == 1
    assert summaries["material_card_candidate_usable_count"] == 1

    assert summaries["material_card_candidate_parameter_fields"] == [
             "material_param_thermal_conductivity",
             "material_param_youngs_modulus"
           ]

    assert aluminum["material_card_id"] == "al-6061-t6"
    assert aluminum["material_status"] == "pass"
    assert aluminum["material_trust_score"] == 1.0
    assert aluminum["material_param_youngs_modulus"] == 69.0e9
    assert aluminum["material_param_youngs_modulus_unit"] == "Pa"
    assert aluminum["material_param_thermal_conductivity"] == 167.0
  end

  test "explains material card screening rankings as a report" do
    score_payload = %{
      "material_score_candidate_count" => 2,
      "material_score_feasible_count" => 2,
      "material_score_criteria" => [
        %{"field" => "material_param_thermal_conductivity", "goal" => "max", "weight" => 0.6},
        %{"field" => "material_issue_count", "goal" => "min", "weight" => 0.4}
      ],
      "material_score_rankings" => [
        %{
          "candidate_id" => "copper",
          "feasible" => true,
          "final_score" => 1.0,
          "weighted_score" => 1.0,
          "metrics" => %{"material_param_thermal_conductivity" => 401.0},
          "criteria_breakdown" => [
            %{
              "field" => "material_param_thermal_conductivity",
              "goal" => "max",
              "actual" => 401.0,
              "weighted_score" => 0.6
            }
          ]
        }
      ]
    }

    assert {:ok, report} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.explain_material_card_screening",
               score_payload,
               %{"label" => "heat-spreader"}
             )

    assert report["material_card_screening_report_schema"] ==
             "kyuubiki.material-card-screening-report/v1"

    assert report["material_screening_best_candidate_id"] == "copper"
    assert report["material_screening_policy"]["label"] == "heat-spreader"

    assert report["material_screening_parameter_fields"] == [
             "material_param_thermal_conductivity"
           ]

    first = hd(report["material_screening_rankings"])
    assert first["candidate_id"] == "copper"
    assert [%{"field" => "material_param_thermal_conductivity"}] = first["deciding_fields"]
  end

  test "plans material experiments from material card screening reports" do
    assert {:ok, plan} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.plan_material_experiments",
               %{
                 "material_screening_rankings" => [
                   %{
                     "candidate_id" => "copper",
                     "feasible" => true,
                     "final_score" => 0.92,
                     "metrics" => %{"material_param_thermal_conductivity" => 401.0},
                     "criteria_breakdown" => []
                   },
                   %{
                     "candidate_id" => "aluminum",
                     "feasible" => true,
                     "final_score" => 0.73,
                     "metrics" => %{"material_param_thermal_conductivity" => 167.0},
                     "criteria_breakdown" => []
                   }
                 ]
               },
               %{"top_k" => 1, "label" => "card-screen"}
             )

    assert plan["material_experiment_plan_count"] == 1
    assert plan["material_experiment_primary_candidate_id"] == "copper"

    experiment = hd(plan["material_experiment_plan"])
    assert experiment["experiment_id"] == "card-screen-1"
    assert experiment["candidate_id"] == "copper"
    assert experiment["expected_score"] == 0.92
  end

  defp material_card do
    %{
      "schema_version" => "kyuubiki.material-card/v1",
      "material_id" => "al-6061-t6",
      "display_name" => "Aluminum 6061-T6",
      "family" => "aluminum",
      "unit_system" => "si",
      "provenance" => %{
        "source_id" => "internal-screening-db",
        "source_label" => "Internal screening database"
      },
      "confidence" => %{
        "level" => "measured",
        "rationale" => "Representative lab coupon values for workflow validation."
      },
      "applicability" => %{"temperature_range_c" => [-40.0, 150.0]},
      "parameters" => %{
        "youngs_modulus" => %{"kind" => "scalar", "value" => 69.0e9, "unit" => "Pa"},
        "thermal_conductivity" => %{
          "kind" => "scalar",
          "value" => 167.0,
          "unit" => "W/(m*K)"
        }
      }
    }
  end
end
