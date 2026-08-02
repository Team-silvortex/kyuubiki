defmodule KyuubikiSdk.ModelResearchValidationTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.ModelResearchValidation
  alias KyuubikiSdk.ModelResearchFrontier

  @schemas Path.expand("../../../schemas", __DIR__)

  test "validates a bound result without overclaiming" do
    assert {:ok, report} =
             ModelResearchValidation.validate(
               frontier(),
               receipt(result_payload()),
               graph(),
               nil,
               &allow/1,
               &allow/1
             )

    assert report["stage"] == "workflow_result_validated"
    assert report["claim_boundary"] == "screening_only_not_qualification"
    assert report["external_validation_required"]
    assert report["origin_plan_digest"] == digest("0")
    assert report["result_plan_digest"] == digest("0")
    assert report["workflow_result"]["artifact_keys"] == ["thermo_summary.result"]
  end

  test "validates a retained screening bundle" do
    bundle = load("examples.material-research-bundle.json")

    assert {:ok, report} =
             ModelResearchValidation.validate(
               frontier(),
               receipt(result_payload()),
               graph(),
               bundle,
               &allow/1,
               &allow/1
             )

    assert report["stage"] == "screening_bundle_validated"
    assert report["material_bundle"]["bundle_id"] == bundle["bundle_id"]
    assert "external_validation_required" in report["next_actions"]
  end

  test "digest verifier reaches result validation" do
    checkpoint = frontier()
    assert {:ok, frontier_digest} = ModelResearchFrontier.compute_digest(checkpoint)
    assert {:ok, verifier} = ModelResearchFrontier.digest_verifier(frontier_digest)

    assert {:ok, report} =
             ModelResearchValidation.validate(
               checkpoint,
               receipt(result_payload()),
               graph(),
               nil,
               verifier,
               &allow/1
             )

    assert report["origin_plan_digest"] == digest("0")
  end

  test "rejects wrong job and unverified frontier" do
    assert {:error, wrong_job} =
             ModelResearchValidation.validate(
               frontier(),
               receipt(result_payload(), "job-guessed"),
               graph(),
               nil,
               &allow/1,
               &allow/1
             )

    assert wrong_job.message =~ "does not match"

    assert {:error, unverified} =
             ModelResearchValidation.validate(
               frontier(),
               receipt(result_payload()),
               graph(),
               nil,
               &deny/1,
               &allow/1
             )

    assert unverified.message =~ "verifier rejected"
  end

  test "rejects a non-completed runtime" do
    assert {:error, error} =
             ModelResearchValidation.validate(
               frontier(),
               receipt(result_payload("running")),
               graph(),
               nil,
               &allow/1,
               &allow/1
             )

    assert error.message =~ "status must be completed"
  end

  test "rejects result receipt from another verified plan" do
    result_receipt = %{receipt(result_payload()) | "plan_digest" => digest("1")}

    assert {:error, error} =
             ModelResearchValidation.validate(
               frontier(),
               result_receipt,
               graph(),
               nil,
               &allow/1,
               &allow/1
             )

    assert error.message =~ "does not match"
  end

  defp graph, do: load("examples.workflow-graph.json")

  defp load(name) do
    @schemas
    |> Path.join(name)
    |> File.read!()
    |> Jason.decode!()
  end

  defp frontier do
    %{
      "schema_version" => "kyuubiki.model-research-frontier/v2",
      "session_id" => "research-session",
      "workflow_id" => "workflow.heat-to-thermo-quad-2d",
      "origin_plan_digest" => digest("0"),
      "stage" => "ready_to_validate",
      "job_id" => "job-validation-001",
      "next_action" => nil,
      "transition_count" => 3,
      "evidence" => %{
        "approval_id" => "approval-test",
        "plan_digest" => digest("0"),
        "action" => "result_fetch",
        "record_index" => 1,
        "authority" => "control_plane",
        "job_status" => nil
      },
      "blocking_reason" => nil
    }
  end

  defp result_payload(status \\ "completed") do
    %{
      "result" => %{
        "workflow_id" => "workflow.heat-to-thermo-quad-2d",
        "run_id" => "run-validation-001",
        "status" => status,
        "artifacts" => %{
          "result/thermal_plane_quad_2d" => %{
            "artifact_id" => "artifact.thermo.result",
            "artifact_type" => "result/thermal_plane_quad_2d",
            "dataset_value" => "thermo_result"
          }
        }
      }
    }
  end

  defp receipt(output, job_id \\ "job-validation-001") do
    %{
      "schema_version" => "kyuubiki.model-research-execution-receipt/v2",
      "plan_schema_version" => "kyuubiki.model-headless-plan/v1",
      "session_id" => "research-session",
      "workflow_id" => "workflow.heat-to-thermo-quad-2d",
      "plan_digest" => digest("0"),
      "status" => "completed",
      "execution_authority" => "kyuubiki-headless-sdk",
      "approval_id" => "approval-test",
      "completed_steps" => 1,
      "failed_step" => nil,
      "records" => [
        %{
          "index" => 1,
          "action" => "result_fetch",
          "job_id" => job_id,
          "authority" => "control_plane",
          "output" => output,
          "error" => nil
        }
      ]
    }
  end

  defp digest(character), do: "sha256:" <> String.duplicate(character, 64)

  defp allow(_value), do: true
  defp deny(_value), do: false
end
