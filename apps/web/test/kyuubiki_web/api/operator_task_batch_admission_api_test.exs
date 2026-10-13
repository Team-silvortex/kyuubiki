defmodule KyuubikiWeb.Api.OperatorTaskBatchAdmissionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase
  alias KyuubikiWeb.Orchestra.OperatorTaskIR, as: IR
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun, as: Run

  test "every public batch route rejects malformed structure and ambiguous identity with HTTP 422" do
    good = batch()
    checkpoint = Run.checkpoint(good)

    for {bad, code} <- [
          {put_in(good, ["tasks", Access.at(1), "task_ir"], 42),
           "operator_task_batch_structure_invalid"},
          {put_in(good, ["tasks", Access.at(1), "task_ir", "operator"], []),
           "operator_task_batch_structure_invalid"},
          {put_in(good, ["tasks", Access.at(1), "case_id"], "case-a"),
           "operator_task_batch_identity_conflict"},
          {put_in(good, ["tasks", Access.at(1), "task_ir", "task_id"], "task-a"),
           "operator_task_batch_identity_conflict"}
        ],
        route <-
          ~w(prepare-batch execute-batch checkpoint-batch verify-checkpoint-batch resume-plan-batch) do
      assert {422, result} = request(route, %{"batch" => bad, "checkpoint" => checkpoint})
      assert result["error_code"] == code
    end

    assert {200, preparation} = request("prepare-batch", %{"batch" => good})
    assert preparation["verified_count"] == 2
    assert {200, execution} = request("execute-batch", %{"batch" => good})
    assert execution["ok_count"] == 2

    assert {200, complete} =
             request("checkpoint-batch", %{"batch" => good, "execution" => execution})

    assert {200, plan} =
             request("resume-plan-batch", %{"batch" => good, "checkpoint" => complete})

    assert plan["next_action"] == "archive"

    assert {200, _} =
             request("verify-checkpoint-batch", %{"batch" => good, "checkpoint" => checkpoint})
  end

  test "checkpoint API returns actionable structural errors instead of HTTP 500 or dropped evidence" do
    for extra <- [
          %{"preparation" => 42},
          %{"execution" => []},
          %{"execution" => %{"failed_case_ids" => ["foreign-case"]}},
          %{"preparation" => %{"failure_receipts" => [%{"recovery" => "retry"}]}},
          %{"checkpoint_id" => %{}},
          %{"created_at" => 42}
        ] do
      assert {422, result} = request("checkpoint-batch", Map.put(extra, "batch", batch()))
      assert result["error_code"] == "operator_task_batch_checkpoint_structure_invalid"
    end
  end

  test "checkpoint API requires preparation evidence and rejects a rehashed false completion" do
    good = batch()

    assert {422, rejected} =
             request("checkpoint-batch", %{
               "batch" => good,
               "preparation" => %{"error_count" => 0}
             })

    assert rejected["error_code"] == "operator_task_checkpoint_evidence_invalid"

    assert {200, prepared} = request("prepare-batch", %{"batch" => good})

    assert {200, checkpoint} =
             request("checkpoint-batch", %{"batch" => good, "preparation" => prepared})

    assert {200, %{"next_action" => "execute"}} =
             request("resume-plan-batch", %{"batch" => good, "checkpoint" => checkpoint})

    assert {200, execution} = request("execute-batch", %{"batch" => good})

    assert {200, complete} =
             request("checkpoint-batch", %{"batch" => good, "execution" => execution})

    assert {200, %{"next_action" => "archive"}} =
             request("resume-plan-batch", %{"batch" => good, "checkpoint" => complete})

    changed = put_in(complete, ["execution", "completion_evidence", "completed_tasks"], [])

    digest =
      changed
      |> Map.delete("checkpoint_digest")
      |> legacy_json()
      |> KyuubikiWeb.CanonicalJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    changed = Map.put(changed, "checkpoint_digest", digest)

    for route <- ~w(verify-checkpoint-batch resume-plan-batch) do
      assert {422, response} = request(route, %{"batch" => good, "checkpoint" => changed})
      assert response["error_code"] == "operator_task_checkpoint_evidence_invalid"
    end

    assert {200, %{"next_action" => "archive"}} =
             request("resume-plan-batch", %{"batch" => good, "checkpoint" => complete})
  end

  defp legacy_json(value) when is_map(value),
    do: Map.new(value, fn {key, nested} -> {key, legacy_json(nested)} end)

  defp legacy_json([]), do: %{}
  defp legacy_json(value) when is_list(value), do: Enum.map(value, &legacy_json/1)
  defp legacy_json(value), do: value

  defp batch do
    tasks =
      for id <- ~w(a b) do
        {:ok, task} = IR.build("export.summary_json", %{"value" => 1}, %{}, task_id: "task-#{id}")
        %{"case_id" => "case-#{id}", "task_ir" => task}
      end

    %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "operator_id" => "export.summary_json",
      "task_count" => 2,
      "tasks" => tasks,
      "case_index" =>
        Enum.map(tasks, fn entry ->
          %{
            "case_id" => entry["case_id"],
            "task_id" => entry["task_ir"]["task_id"],
            "task_digest" => entry["task_ir"]["integrity"]["task_digest"]
          }
        end)
    }
  end

  defp request(route, payload) do
    response =
      :post
      |> conn("/api/v1/operator-tasks/#{route}", Jason.encode!(payload))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {response.status, Jason.decode!(response.resp_body)}
  end
end
