defmodule KyuubikiWeb.Orchestra.WorkflowCancellationStateTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Storage.DurableJson

  @key "cancellation_intent"
  @job %{job_id: "state-job", created_at: ~U[2026-10-28 00:00:00Z]}
  @parent %{
    "envelope_sha256" => String.duplicate("a", 64),
    "generation" => 1,
    "owner_session_id" => "state-owner"
  }
  @endpoint %{id: "state-agent", host: "127.0.0.1", port: 5001, agent_session_id: "session"}
  @target %{
    "job_id" => "state-job",
    "request_id" => "state-request",
    "generation" => 7,
    "process_instance_id" => "state-process"
  }
  @failures ~w(original_endpoint_not_configured endpoint_directory_unavailable delivery_budget_exhausted original_endpoint_unreachable request_not_observed_active agent_observation_invalid execution_target_invalid cancellation_outcome_unknown agent_acknowledgement_invalid)
  @states ~w(pending dispatching requested target_not_observed exhausted)

  for {label, status, attempts, reason} <- [
        {"fresh failure", "pending", 0, "original_endpoint_unreachable"},
        {"missing retry reason", "pending", 1, nil},
        {"spent retry", "pending", 3, "cancellation_outcome_unknown"},
        {"early budget reason", "pending", 1, "retry_budget_exhausted"},
        {"dispatch with outcome", "dispatching", 1, "cancellation_outcome_unknown"},
        {"accepted with failure", "requested", 1, "cancellation_outcome_unknown"},
        {"unobserved with failure", "target_not_observed", 1, "original_endpoint_unreachable"},
        {"early exhaustion", "exhausted", 1, "cancellation_outcome_unknown"},
        {"unexplained exhaustion", "exhausted", 3, nil}
      ] do
    test "resealed #{label} is rejected rather than treated as recoverable evidence" do
      original = recovery()
      entry = hd(original[@key]["entries"])

      changed =
        Map.merge(entry, %{
          "status" => unquote(status),
          "attempts" => unquote(attempts),
          "reason" => unquote(reason),
          "delivery_id" => if(unquote(attempts) > 0, do: String.duplicate("d", 32)),
          "execution_target" =>
            if(unquote(status) in ~w(requested target_not_observed), do: @target)
        })

      updated = replace_entry(original, changed)
      assert {:error, :invalid_workflow_cancellation_intent} = Intent.verify(updated, @job)
      assert Intent.summary(updated)["status"] == "invalid"
      assert Intent.summary(updated)["counts"] == %{"invalid" => 1}
      refute Intent.summary(updated)["execution_terminal_confirmed"]
    end
  end

  for status <- @states do
    test "#{status} projections agree with states produced by legitimate transitions" do
      original = recovery()
      base = hd(original[@key]["entries"])
      reachable = reachable_projections(original)
      reasons = [nil, "retry_budget_exhausted" | @failures]

      for attempts <- 0..3,
          reason <- reasons,
          delivery? <- [false, true],
          bound? <- [false, true] do
        entry =
          Map.merge(base, %{
            "status" => unquote(status),
            "attempts" => attempts,
            "reason" => reason,
            "delivery_id" => if(delivery?, do: String.duplicate("d", 32)),
            "execution_target" => if(bound?, do: @target)
          })

        updated = replace_entry(original, entry)
        expected = MapSet.member?(reachable, projection(entry))
        actual = Intent.verify(updated, @job) == :ok
        assert actual == expected, "unexpected state admission: #{inspect(projection(entry))}"

        summary = Intent.summary(updated)

        assert summary["status"] ==
                 if(expected, do: summary_status(unquote(status)), else: "invalid")

        assert summary["counts"] == %{if(expected, do: unquote(status), else: "invalid") => 1}
        refute summary["execution_terminal_confirmed"]
      end
    end
  end

  for attempt <- 1..3 do
    test "attempt #{attempt} cannot use reservation exhaustion as a delivery outcome" do
      bucket = reserve_count(recovery()[@key], unquote(attempt))
      entry = hd(bucket["entries"])
      token = token(entry)

      assert {:error, :invalid_cancellation_outcome} =
               Intent.finish(bucket, token, "retry_budget_exhausted")

      assert {:ok, ^entry} = Intent.owned_entry(bucket, token)
    end
  end

  defp reachable_projections(original) do
    fresh = original[@key]

    states =
      for attempt <- 1..3, bound? <- [false, true], reduce: [fresh] do
        acc ->
          reserved = reserve_count(fresh, attempt)
          entry = hd(reserved["entries"])
          token = token(entry)

          reserved =
            if bound? do
              {:ok, bound, _} = Intent.bind(reserved, token, @target)
              bound
            else
              reserved
            end

          outcomes = @failures ++ if(bound?, do: ~w(requested target_not_observed), else: [])

          finished =
            Enum.map(outcomes, fn outcome ->
              {:ok, next, _} = Intent.finish(reserved, token, outcome)
              next
            end)

          exhausted =
            if attempt == 3 do
              {:ok, next, _} = Intent.reserve(reserved, Intent.identity(entry))
              [next]
            else
              []
            end

          [reserved | exhausted ++ finished ++ acc]
      end

    Enum.each(states, fn bucket ->
      assert :ok = Intent.verify(Map.put(original, @key, bucket), @job)
    end)

    MapSet.new(states, &projection(hd(&1["entries"])))
  end

  defp reserve_count(bucket, count) do
    Enum.reduce(1..count, bucket, fn _, current ->
      {:ok, next, _} = Intent.reserve(current, Intent.identity(hd(current["entries"])))
      next
    end)
  end

  defp token(entry), do: %{id: Intent.identity(entry), delivery_id: entry["delivery_id"]}

  defp projection(entry),
    do:
      {entry["status"], entry["attempts"], entry["reason"], not is_nil(entry["delivery_id"]),
       not is_nil(entry["execution_target"])}

  defp summary_status(state) when state in ~w(pending dispatching), do: "pending"
  defp summary_status("exhausted"), do: "unresolved"
  defp summary_status(_), do: "acknowledged"

  defp recovery do
    snapshot = %{
      requests: [%{endpoint: @endpoint, job_id: "state-job", request_id: "state-request"}],
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    {:ok, recovery} = Intent.attach(@parent, @job, snapshot)
    recovery
  end

  defp replace_entry(original, entry) do
    bucket = Map.put(original[@key], "entries", [entry])
    digest = :crypto.hash(:sha256, DurableJson.encode!(Map.delete(bucket, "sha256")))
    Map.put(original, @key, Map.put(bucket, "sha256", Base.encode16(digest, case: :lower)))
  end
end
