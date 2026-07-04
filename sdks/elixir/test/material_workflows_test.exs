defmodule KyuubikiSdk.MaterialWorkflowsTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.MaterialWorkflows

  test "catalog prefers Orchestra catalog workflow before graph fallback" do
    catalog = MaterialWorkflows.material_workflow_catalog()

    assert Enum.at(catalog, 0)["id"] == "material_study_envelope_catalog"
    assert Enum.at(catalog, 0)["workflow_kind"] == "orchestra_catalog_job"
    assert Enum.at(catalog, 0)["required_actions"] |> hd() == "workflow_submit_catalog"
    assert Enum.at(catalog, 1)["workflow_kind"] == "operator_graph"
  end

  test "catalog request targets the built-in material envelope workflow" do
    request = MaterialWorkflows.material_study_envelope_catalog_request()

    assert request["workflow_id"] == MaterialWorkflows.material_envelope_catalog_workflow_id()

    assert get_in(request, ["input_artifacts", "material_rows", "rows", Access.at(0), "case_id"]) ==
             "cool_stiff"
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
  end
end
