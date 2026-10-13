defmodule KyuubikiWeb.Orchestra.WorkflowCancellationIntentTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Storage.DurableJson

  @key "cancellation_intent"
  @job %{job_id: "job", created_at: ~U[2026-10-13 00:00:00Z]}
  @parent %{
    "envelope_sha256" => String.duplicate("a", 64),
    "generation" => 1,
    "owner_session_id" => "session"
  }
  @endpoint %{
    id: "original",
    host: "127.0.0.1",
    port: 5001,
    agent_session_id: "agent-session",
    token: "DO-NOT-PERSIST"
  }
  @target %{
    "job_id" => "job",
    "request_id" => "request",
    "generation" => 1,
    "process_instance_id" => "process"
  }

  test "intent is bounded, sealed and contains no endpoint credentials or payload" do
    recovery = recovery()
    assert :ok = Intent.verify(recovery, @job)
    encoded = Jason.encode!(recovery[@key])
    for secret <- ["DO-NOT-PERSIST", "127.0.0.1", "agent-session"], do: refute(encoded =~ secret)
    assert Intent.pending?(recovery[@key])
    assert Intent.summary(recovery)["counts"] == %{"pending" => 1}

    assert {:error, :invalid_workflow_cancellation_intent} =
             Intent.attach(recovery, @job, snapshot())
  end

  test "empty read-only snapshot does not invent a cancellation intent" do
    assert {:ok, @parent} = Intent.attach(@parent, @job, snapshot([]))
    assert Intent.summary(@parent) == nil
  end

  for field <- ~w(job_id job_created_at envelope_sha256 generation owner_session_id) do
    test "even a resealed intent cannot change its parent #{field}" do
      field = unquote(field)
      original = recovery()
      value = if field == "generation", do: 2, else: "different"
      changed = update_in(original, [@key], &seal(Map.put(&1, field, value)))
      assert {:error, :invalid_workflow_cancellation_intent} = Intent.verify(changed, @job)
    end
  end

  test "damaged digest and malformed or oversized entries fail closed" do
    original = recovery()
    entry = hd(original[@key]["entries"])

    invalid = [
      [],
      [nil],
      [entry, entry],
      List.duplicate(entry, 129),
      [Map.put(entry, "endpoint_fingerprint", "http://untrusted")],
      [Map.put(entry, "request_id", "\n")],
      [Map.put(entry, "token", "secret")],
      [Map.put(entry, "attempts", 4)],
      [Map.put(entry, "status", "requested")]
    ]

    for entries <- invalid do
      changed = update_in(original, [@key], &seal(Map.put(&1, "entries", entries)))
      assert {:error, :invalid_workflow_cancellation_intent} = Intent.verify(changed, @job)
    end

    changed = put_in(original, [@key, "sha256"], String.duplicate("0", 64))
    assert {:error, :invalid_workflow_cancellation_intent} = Intent.verify(changed, @job)
  end

  test "duplicate and oversized captured requests are refused before persistence" do
    [request] = snapshot().requests

    for requests <- [[request, request], List.duplicate(request, 129)] do
      assert {:error, :invalid_workflow_cancellation_intent} =
               Intent.attach(@parent, @job, snapshot(requests))
    end
  end

  test "bound target is immutable across retry and old delivery tokens cannot finish it" do
    original = recovery()
    bucket = original[@key]
    id = Intent.identity(hd(bucket["entries"]))
    assert {:ok, reserved, entry} = Intent.reserve(bucket, id)
    token = %{id: id, delivery_id: entry["delivery_id"]}
    assert {:ok, bound, _} = Intent.bind(reserved, token, @target)

    for target <- [
          Map.put(@target, "generation", 2),
          Map.put(@target, "job_id", "other"),
          Map.put(@target, "request_id", "later"),
          Map.put(@target, "process_instance_id", "new")
        ] do
      assert {:error, :cancellation_target_changed} = Intent.bind(bound, token, target)
    end

    assert {:ok, retried, second} = Intent.reserve(bound, id)
    assert second["execution_target"] == @target
    assert {:error, :stale_cancellation_delivery} = Intent.finish(retried, token, "requested")
    assert {:error, :stale_cancellation_delivery} = Intent.owned_entry(retried, %{})
    token = %{id: id, delivery_id: second["delivery_id"]}
    assert {:ok, acknowledged, _} = Intent.finish(retried, token, "requested")
    updated = Map.put(original, @key, acknowledged)
    assert :ok = Intent.verify(updated, @job)
    refute Intent.pending?(acknowledged)
    assert Intent.summary(updated)["status"] == "acknowledged"
    refute Intent.summary(updated)["execution_terminal_confirmed"]
    assert {:error, :cancellation_already_settled} = Intent.reserve(acknowledged, id)
  end

  test "lost acknowledgements exhaust a finite budget without claiming termination" do
    original = recovery()
    id = Intent.identity(hd(original[@key]["entries"]))

    exhausted =
      Enum.reduce(1..3, original[@key], fn attempt, bucket ->
        assert {:ok, reserved, entry} = Intent.reserve(bucket, id)
        assert entry["attempts"] == attempt
        token = %{id: id, delivery_id: entry["delivery_id"]}
        assert {:ok, next, _} = Intent.finish(reserved, token, "cancellation_outcome_unknown")
        next
      end)

    assert :ok = Intent.verify(Map.put(original, @key, exhausted), @job)
    assert Intent.summary(%{@key => exhausted})["status"] == "unresolved"
    refute Intent.pending?(exhausted)
    assert {:error, :cancellation_already_settled} = Intent.reserve(exhausted, id)
  end

  test "crashes after reservation consume the same retry budget" do
    original = recovery()
    id = Intent.identity(hd(original[@key]["entries"]))

    bucket =
      Enum.reduce(1..3, original[@key], fn _, bucket ->
        assert {:ok, next, _} = Intent.reserve(bucket, id)
        next
      end)

    assert {:ok, exhausted, entry} = Intent.reserve(bucket, id)
    assert entry["status"] == "exhausted"
    assert entry["attempts"] == 3
    assert entry["reason"] == "retry_budget_exhausted"
    assert :ok = Intent.verify(Map.put(original, @key, exhausted), @job)
  end

  test "unbound requests and arbitrary errors cannot become successful receipts" do
    bucket = recovery()[@key]
    id = Intent.identity(hd(bucket["entries"]))
    assert {:ok, reserved, entry} = Intent.reserve(bucket, id)
    token = %{id: id, delivery_id: entry["delivery_id"]}

    for outcome <- ["requested", "target_not_observed", "secret transport details", nil] do
      assert {:error, :invalid_cancellation_outcome} = Intent.finish(reserved, token, outcome)
    end
  end

  test "public summary cannot echo malformed status values or internal identities" do
    summary = Intent.summary(%{@key => %{"entries" => [%{"status" => "SECRET"}]}})
    assert summary["status"] == "invalid"
    assert summary["counts"] == %{"invalid" => 1}
    refute Jason.encode!(summary) =~ "SECRET"
    refute Jason.encode!(Intent.summary(recovery())) =~ "request_id"
  end

  defp recovery do
    assert {:ok, recovery} = Intent.attach(@parent, @job, snapshot())
    recovery
  end

  defp snapshot(requests \\ [%{endpoint: @endpoint, job_id: "job", request_id: "request"}]),
    do: %{requests: requests, queued_cancelled_count: 0, reserved_cancelled_count: 0}

  defp seal(bucket) do
    digest = :crypto.hash(:sha256, DurableJson.encode!(Map.delete(bucket, "sha256")))
    Map.put(bucket, "sha256", Base.encode16(digest, case: :lower))
  end
end
