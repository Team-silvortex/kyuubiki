defmodule KyuubikiSdk.ModelResearchFrontierTest do
  use ExUnit.Case, async: true

  alias KyuubikiSdk.ModelResearchFrontier

  test "verified submission binds real job id into next proposal" do
    submitted =
      receipt("workflow_submit_catalog",
        output: %{"job" => %{"job_id" => "job-real-001", "status" => "queued"}}
      )

    assert {:ok, frontier} = ModelResearchFrontier.start(submitted, fn _ -> true end)
    assert frontier["stage"] == "waiting_for_job"
    assert frontier["job_id"] == "job-real-001"
    assert frontier["origin_plan_digest"] == submitted["plan_digest"]
    assert frontier["evidence"]["plan_digest"] == submitted["plan_digest"]
    assert {:ok, proposal} = ModelResearchFrontier.build_proposal(frontier, fn _ -> true end)
    assert get_in(proposal, ["calls", Access.at(0), "action"]) == "job_wait"
    assert get_in(proposal, ["calls", Access.at(0), "payload", "job_id"]) == "job-real-001"
  end

  test "unverified receipt cannot create frontier" do
    submitted =
      receipt("workflow_submit_graph", output: %{"job" => %{"job_id" => "job-real-002"}})

    assert {:error, error} = ModelResearchFrontier.start(submitted, fn _ -> false end)
    assert error.message =~ "receipt verifier rejected"
  end

  test "wait and fetch advance to validation without guessing ids" do
    submitted = receipt("fem_submit", output: %{"job" => %{"job_id" => "job-real-003"}})
    assert {:ok, waiting} = ModelResearchFrontier.start(submitted, fn _ -> :ok end)

    waited =
      receipt("job_wait",
        job_id: "job-real-003",
        plan_digest: digest("1"),
        output: %{
          "terminal" => %{"job" => %{"job_id" => "job-real-003", "status" => "completed"}},
          "history" => []
        }
      )

    assert {:ok, fetch} =
             ModelResearchFrontier.advance(
               waiting,
               waited,
               fn _ -> true end,
               fn _ -> {:ok, :mac} end
             )

    assert fetch["stage"] == "ready_to_fetch_result"
    assert fetch["origin_plan_digest"] == digest("0")
    assert fetch["evidence"]["plan_digest"] == digest("1")
    assert {:ok, proposal} = ModelResearchFrontier.build_proposal(fetch, fn _ -> true end)
    assert get_in(proposal, ["calls", Access.at(0), "action"]) == "result_fetch"
    assert get_in(proposal, ["calls", Access.at(0), "payload", "job_id"]) == "job-real-003"

    result =
      receipt("result_fetch",
        job_id: "job-real-003",
        plan_digest: digest("2"),
        output: %{"result" => %{"artifacts" => []}}
      )

    assert {:ok, validate} =
             ModelResearchFrontier.advance(fetch, result, fn _ -> true end, fn _ -> true end)

    assert validate["stage"] == "ready_to_validate"
    assert validate["next_action"] == nil
    assert validate["origin_plan_digest"] == digest("0")
    assert validate["evidence"]["plan_digest"] == digest("2")
  end

  test "malformed plan digest cannot enter frontier chain" do
    submitted =
      receipt("workflow_submit_catalog",
        output: %{"job" => %{"job_id" => "job-real-008"}},
        plan_digest: "sha256:NOT-A-DIGEST"
      )

    assert {:error, error} = ModelResearchFrontier.start(submitted, fn _ -> true end)
    assert error.message =~ "receipt"
  end

  test "mismatched job binding is rejected" do
    submitted =
      receipt("workflow_submit_catalog", output: %{"job" => %{"job_id" => "job-real-004"}})

    assert {:ok, waiting} = ModelResearchFrontier.start(submitted, fn _ -> true end)

    wrong =
      receipt("job_wait",
        job_id: "job-guessed",
        output: %{"terminal" => %{"job" => %{"status" => "completed"}}}
      )

    assert {:error, error} =
             ModelResearchFrontier.advance(waiting, wrong, fn _ -> true end, fn _ -> true end)

    assert error.message =~ "job_id does not match"
  end

  test "terminal and execution failures block progression" do
    submitted =
      receipt("workflow_submit_catalog", output: %{"job" => %{"job_id" => "job-real-005"}})

    assert {:ok, waiting} = ModelResearchFrontier.start(submitted, fn _ -> true end)

    failed_job =
      receipt("job_wait",
        job_id: "job-real-005",
        output: %{"terminal" => %{"job" => %{"status" => "failed"}}}
      )

    assert {:ok, blocked} =
             ModelResearchFrontier.advance(
               waiting,
               failed_job,
               fn _ -> true end,
               fn _ -> true end
             )

    assert blocked["stage"] == "blocked"
    assert blocked["blocking_reason"] == "job reached terminal status failed"

    dispatch_failed =
      receipt("workflow_submit_catalog",
        output: nil,
        status: "failed",
        error: "control plane unavailable"
      )

    assert {:ok, initial_blocked} =
             ModelResearchFrontier.start(dispatch_failed, fn _ -> true end)

    assert initial_blocked["blocking_reason"] == "control plane unavailable"
  end

  test "repository frontier fixture matches sdk contract" do
    path = Path.expand("../../../schemas/examples.model-research-frontier.json", __DIR__)
    frontier = path |> File.read!() |> Jason.decode!()
    assert frontier["schema_version"] == ModelResearchFrontier.schema_version()

    expected_digest =
      "sha256:aba8f2d4289d4385f07fcb065f65b26a71d7c606397dd9d20700d604b9b25902"

    assert {:ok, ^expected_digest} = ModelResearchFrontier.compute_digest(frontier)
    assert {:ok, verifier} = ModelResearchFrontier.digest_verifier(expected_digest)
    assert {:ok, proposal} = ModelResearchFrontier.build_proposal(frontier, verifier)

    changed = %{frontier | "transition_count" => frontier["transition_count"] + 1}
    assert {:ok, changed_digest} = ModelResearchFrontier.compute_digest(changed)
    refute changed_digest == expected_digest
    assert {:error, mismatch} = ModelResearchFrontier.build_proposal(changed, verifier)
    assert mismatch.message =~ "trusted state"

    invalid = put_in(frontier, ["evidence", "action"], "ResultFetch")
    assert {:error, invalid_error} = ModelResearchFrontier.compute_digest(invalid)
    assert invalid_error.message =~ "incomplete"

    assert {:error, malformed} = ModelResearchFrontier.digest_verifier("sha256:not-valid")
    assert malformed.message =~ "digest is invalid"

    assert get_in(proposal, ["calls", Access.at(0), "payload", "job_id"]) ==
             "job-material-envelope-001"
  end

  test "inconsistent frontier state is rejected" do
    submitted =
      receipt("workflow_submit_catalog", output: %{"job" => %{"job_id" => "job-real-006"}})

    assert {:ok, frontier} = ModelResearchFrontier.start(submitted, fn _ -> true end)
    inconsistent = %{frontier | "next_action" => "result_fetch"}

    assert {:error, error} =
             ModelResearchFrontier.build_proposal(inconsistent, fn _ -> true end)

    assert error.message =~ "stage and next action"
  end

  test "unverified frontier cannot generate proposal" do
    submitted =
      receipt("workflow_submit_catalog", output: %{"job" => %{"job_id" => "job-real-007"}})

    assert {:ok, frontier} = ModelResearchFrontier.start(submitted, fn _ -> true end)
    assert {:error, error} = ModelResearchFrontier.build_proposal(frontier, fn _ -> false end)
    assert error.message =~ "frontier verifier rejected"
  end

  defp receipt(action, opts) do
    error = Keyword.get(opts, :error)

    %{
      "schema_version" => "kyuubiki.model-research-execution-receipt/v2",
      "plan_schema_version" => "kyuubiki.model-headless-plan/v1",
      "session_id" => "research-session",
      "workflow_id" => "workflow.material",
      "plan_digest" => Keyword.get(opts, :plan_digest, digest("0")),
      "status" => Keyword.get(opts, :status, "completed"),
      "execution_authority" => "kyuubiki-headless-sdk",
      "approval_id" => "approval-test",
      "completed_steps" => if(error, do: 0, else: 1),
      "failed_step" => if(error, do: 1, else: nil),
      "records" => [
        %{
          "index" => 1,
          "action" => action,
          "job_id" => Keyword.get(opts, :job_id),
          "authority" => if(error, do: nil, else: "control_plane"),
          "output" => Keyword.fetch!(opts, :output),
          "error" => error
        }
      ]
    }
  end

  defp digest(character), do: "sha256:" <> String.duplicate(character, 64)
end
