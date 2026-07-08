defmodule KyuubikiSdk.MaterialWorkflows do
  @moduledoc "Material workflow request helpers for headless catalog execution."

  @material_envelope_catalog_workflow_id "workflow.material-study-envelope-ranking-json"
  @material_study_execution_plan_schema_version "kyuubiki.material-study-execution-plan/v1"
  @material_study_execution_plan_example_path Path.expand(
                                                "../../../../schemas/examples.material-study-execution-plan.json",
                                                __DIR__
                                              )

  def material_envelope_catalog_workflow_id, do: @material_envelope_catalog_workflow_id

  def material_study_execution_plan_schema_version,
    do: @material_study_execution_plan_schema_version

  def material_study_execution_plan_example do
    @material_study_execution_plan_example_path
    |> File.read!()
    |> Jason.decode!()
  end

  def material_study_envelope_input_artifacts(attrs \\ %{}) do
    rows =
      Map.get(attrs, :rows) ||
        Map.get(attrs, "rows") ||
        [
          %{
            "case_id" => "cool_stiff",
            "summaries" => %{
              "thermal" => %{"max_temperature" => 82.0},
              "structural" => %{"max_stress" => 100.0}
            }
          },
          %{
            "case_id" => "warm_safe",
            "summaries" => %{
              "thermal" => %{"max_temperature" => 90.0},
              "structural" => %{"max_stress" => 120.0}
            }
          },
          %{
            "case_id" => "hot_light",
            "summaries" => %{
              "thermal" => %{"max_temperature" => 140.0},
              "structural" => %{"max_stress" => 110.0}
            }
          }
        ]

    %{"material_rows" => %{"rows" => rows}}
  end

  def material_study_envelope_catalog_request(input_artifacts \\ nil) do
    %{
      "workflow_id" => @material_envelope_catalog_workflow_id,
      "input_artifacts" => input_artifacts || material_study_envelope_input_artifacts()
    }
  end

  def material_workflow_catalog do
    [
      %{
        "id" => "material_study_envelope_catalog",
        "title" => "Material Study Envelope Catalog Job",
        "domain" => "multi_physics_materials",
        "objective" =>
          "submit the built-in material envelope workflow from the Orchestra catalog",
        "template_id" => "material_study_envelope_catalog",
        "workflow_kind" => "orchestra_catalog_job",
        "required_actions" => ["workflow_submit_catalog", "job_wait", "result_fetch"],
        "aliases" => ["material-envelope-catalog", "material_envelope_catalog"]
      },
      %{
        "id" => "material_study_envelope_ranking",
        "title" => "Material Study Envelope Ranking",
        "domain" => "multi_physics_materials",
        "objective" =>
          "compose material envelopes, rank candidates, and extract a Pareto frontier",
        "template_id" => "material_study_envelope_ranking",
        "workflow_kind" => "operator_graph",
        "required_actions" => ["workflow_submit_graph", "job_wait", "result_fetch"],
        "aliases" => ["material-envelope", "material_envelope", "material.pareto_ranking.v1"]
      }
    ]
  end
end
