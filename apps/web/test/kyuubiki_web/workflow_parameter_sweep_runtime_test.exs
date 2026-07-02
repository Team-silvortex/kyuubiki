defmodule KyuubikiWeb.WorkflowParameterSweepRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.WorkflowOperatorCatalog
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "catalog exposes the full parameter sweep loop" do
    for {operator_id, family} <- [
          {"transform.expand_parameter_sweep", "parameter_sweep"},
          {"transform.join_parameter_sweep_results", "parameter_sweep_join_results"},
          {"transform.summarize_parameter_sweep", "parameter_sweep_summary"},
          {"transform.score_parameter_sweep", "parameter_sweep_score"}
        ] do
      assert {:ok, %{"operator" => operator}} = WorkflowOperatorCatalog.fetch(operator_id)
      assert operator["family"] == family
      assert "parameter_sweep" in operator["capability_tags"]
      assert "headless_safe" in operator["capability_tags"]
    end
  end

  test "joins, summarizes, and scores parameter sweep case results" do
    expanded = %{
      "cases" => [
        %{
          "id" => "quality_candidate_0",
          "parameters" => %{"thickness" => 0.01},
          "model" => %{"thickness" => 0.01}
        },
        %{
          "id" => "quality_candidate_1",
          "parameters" => %{"thickness" => 0.02},
          "model" => %{"thickness" => 0.02}
        }
      ]
    }

    results = [
      %{
        "case_id" => "quality_candidate_1",
        "status" => "ok",
        "summary" => %{"mass" => 4.8, "max_stress" => 88.0}
      },
      %{
        "case_id" => "quality_candidate_0",
        "status" => "ok",
        "summary" => %{"mass" => 2.2, "max_stress" => 140.0}
      }
    ]

    assert {:ok, joined} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.join_parameter_sweep_results",
               Map.put(expanded, "results", results),
               %{"summary_field" => "summary"}
             )

    assert joined["joined_summary_count"] == 2
    assert hd(joined["cases"])["summary"]["max_stress"] == 140.0

    assert {:ok, summarized} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.summarize_parameter_sweep",
               joined,
               %{"fields" => ["mass", "max_stress"]}
             )

    assert summarized["row_count"] == 2
    assert summarized["numeric_columns"]["mass"]["mean"] == 3.5

    assert {:ok, scored} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_parameter_sweep",
               summarized,
               %{
                 "objectives" => [
                   %{"field" => "mass", "goal" => "min", "weight" => 1.0},
                   %{
                     "field" => "max_stress",
                     "goal" => "min",
                     "weight" => 0.02,
                     "max_allowed" => 100.0
                   }
                 ]
               }
             )

    assert scored["scored_count"] == 2
    assert scored["best"]["case_id"] == "quality_candidate_1"
    assert scored["best"]["objective_feasible"] == true
    assert List.last(scored["scored_rows"])["objective_feasible"] == false
  end

  test "score parameter sweep reports missing numeric objective fields" do
    assert {:error, {:missing_parameter_sweep_score_field, 0}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_parameter_sweep",
               %{"rows" => [%{"case_id" => "candidate_without_mass"}]},
               %{"objectives" => [%{"field" => "mass", "goal" => "min"}]}
             )
  end

  test "score parameter sweep rejects unsupported objective goals" do
    assert {:error, {:invalid_parameter_sweep_objective, 0}} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.score_parameter_sweep",
               %{"rows" => [%{"case_id" => "candidate", "mass" => 1.0}]},
               %{"objectives" => [%{"field" => "mass", "goal" => "median"}]}
             )
  end
end
