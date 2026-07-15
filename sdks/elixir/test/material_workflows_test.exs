defmodule KyuubikiSdk.MaterialWorkflowsTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.MaterialWorkflows

  @fixture_path Path.expand(
                  "../../../schemas/examples.material-envelope-catalog-request.json",
                  __DIR__
                )

  test "catalog prefers Orchestra catalog workflow before graph fallback" do
    catalog = MaterialWorkflows.material_workflow_catalog()

    assert Enum.at(catalog, 0)["id"] == "material_study_envelope_catalog"
    assert Enum.at(catalog, 0)["workflow_kind"] == "orchestra_catalog_job"
    assert Enum.at(catalog, 0)["required_actions"] |> hd() == "workflow_submit_catalog"
    assert Enum.at(catalog, 1)["workflow_kind"] == "operator_graph"
  end

  test "catalog request targets the built-in material envelope workflow" do
    request = MaterialWorkflows.material_study_envelope_catalog_request()
    {:ok, fixture} = @fixture_path |> File.read!() |> Jason.decode()

    assert request["workflow_id"] == MaterialWorkflows.material_envelope_catalog_workflow_id()
    assert request == Map.delete(fixture, "$schema")

    assert get_in(request, ["input_artifacts", "material_rows", "rows", Access.at(0), "case_id"]) ==
             "cool_stiff"
  end

  test "execution plan helper exposes the shared contract fixture" do
    plan = MaterialWorkflows.material_study_execution_plan_example()

    assert MaterialWorkflows.material_study_execution_plan_schema_version() ==
             "kyuubiki.material-study-execution-plan/v1"

    assert plan["schema_version"] ==
             MaterialWorkflows.material_study_execution_plan_schema_version()

    assert plan["study_id"] == "material_heat_spreader_screening"
    assert plan["step_count"] == length(plan["steps"])
    assert plan["solve_step_count"] == 3
    assert plan["candidate_count"] == 3
    assert plan["material_card_contract_required"] == true
    assert plan["material_card_schema_version"] == "kyuubiki.material-card/v1"
    assert plan["material_card_ref_count"] == 3
    assert "copper_c110" in plan["candidate_ids"]
    assert plan["recommended_command"] =~ "heat-spreader"
  end

  test "execution plan helper returns a fresh decoded payload" do
    first = MaterialWorkflows.material_study_execution_plan_example()
    second = MaterialWorkflows.material_study_execution_plan_example()

    first = put_in(first, ["steps"], [])

    assert first["steps"] == []
    assert length(second["steps"]) == second["step_count"]
  end

  test "input artifacts accept explicit candidate rows" do
    artifacts =
      MaterialWorkflows.material_study_envelope_input_artifacts(%{
        rows: [
          %{
            "case_id" => "candidate-a",
            "summaries" => %{"thermal" => %{"max_temperature" => 77.0}}
          }
        ]
      })

    assert get_in(artifacts, ["material_rows", "rows", Access.at(0), "case_id"]) == "candidate-a"
  end

  test "top-level SDK exposes material workflow helpers" do
    assert KyuubikiSdk.material_envelope_catalog_workflow_id() ==
             "workflow.material-study-envelope-ranking-json"

    assert hd(KyuubikiSdk.material_workflow_catalog())["id"] == "material_study_envelope_catalog"
    assert KyuubikiSdk.material_study_execution_plan_example()["study_id"] ==
             "material_heat_spreader_screening"
  end
end
