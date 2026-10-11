defmodule KyuubikiWeb.Orchestra.WorkflowSolveCheckpointTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.TestSupport.WorkflowSolveCheckpointFixture, as: Fixture

  test "later claims reuse a lossless successful artifact without changing it" do
    {recovery, claim} = Fixture.running()
    request = Fixture.request(recovery)
    result = %{"value" => 1.0, "tiny" => 1.0e-30, "negative" => -2.5e-28}
    assert :execute = Checkpoint.lookup(recovery, request, claim)
    assert {:ok, saved, :retained} = Checkpoint.put(recovery, request, result, claim)
    assert :ok = Checkpoint.verify(saved, "parent")
    assert {:ok, replay, next} = Envelope.claim(saved, "new-owner", :process_restart)
    assert {:ok, ^result} = Checkpoint.lookup(replay, request, next)
    assert {:error, :conflict} = Checkpoint.lookup(saved, request, claim)
    assert {:ok, ^saved, :unchanged} = Checkpoint.put(saved, request, result, claim)
    assert {:error, :conflict} = Checkpoint.put(saved, request, %{"value" => 42}, claim)
  end

  test "tiny differences cannot collide through TaskIR's fixed-decimal formatter" do
    {recovery, claim} = Fixture.running()
    first = Fixture.request(recovery, 1, %{"value" => 1.0e-30})
    second = Fixture.request(recovery, 1, %{"value" => 2.0e-30})
    refute first["invocation_sha256"] == second["invocation_sha256"]
    assert {:ok, saved, :retained} = Checkpoint.put(recovery, first, 42, claim)
    {:ok, replay, next} = Envelope.claim(saved, "new-owner", :process_restart)
    assert {:error, :input_changed} = Checkpoint.lookup(replay, second, next)
  end

  test "payload numeric representation remains part of the invocation identity" do
    {recovery, _claim} = Fixture.running()

    refute Fixture.request(recovery, 1, %{"value" => 1})["invocation_sha256"] ==
             Fixture.request(recovery, 1, %{"value" => 1.0})["invocation_sha256"]

    refute Fixture.request(recovery, 1, %{"value" => 0.0})["invocation_sha256"] ==
             Fixture.request(recovery, 1, %{"value" => -0.0})["invocation_sha256"]
  end

  for field <- ~w(config placement_tags operator_id outputs) do
    test "changed node #{field} cannot attach an old checkpoint" do
      {recovery, claim} = Fixture.running()
      request = Fixture.request(recovery)
      {:ok, saved, :retained} = Checkpoint.put(recovery, request, 42, claim)
      {:ok, replay, next} = Envelope.claim(saved, "new-owner", :process_restart)
      node = hd(recovery["envelope"]["graph"]["nodes"])
      changed = Map.put(node, unquote(field), "different")

      request =
        Checkpoint.request(
          changed,
          %{"value" => 41},
          recovery["envelope"]["orchestration_context"]
        )

      assert {:error, :invalid} = Checkpoint.lookup(replay, request, next)
    end
  end

  test "authority or parent context changes cannot reuse a completed solve" do
    {recovery, claim} = Fixture.running()
    {:ok, saved, :retained} = Checkpoint.put(recovery, Fixture.request(recovery), 42, claim)
    {:ok, replay, next} = Envelope.claim(saved, "new-owner", :process_restart)
    node = hd(recovery["envelope"]["graph"]["nodes"])

    for context <- [%{"job_id" => "foreign"}, %{"job_id" => "parent", "orch_id" => "other"}] do
      request = Checkpoint.request(node, %{"value" => 41}, context)
      assert {:error, :input_changed} = Checkpoint.lookup(replay, request, next)
    end
  end

  test "equal inputs cannot reuse a solve with changed or previously unbound provenance" do
    for original_binding <- [nil, String.duplicate("a", 64)] do
      {recovery, claim} = Fixture.running()
      node = hd(recovery["envelope"]["graph"]["nodes"])
      context = recovery["envelope"]["orchestration_context"]

      context =
        if original_binding,
          do: Map.put(context, "workflow_input_binding_sha256", original_binding),
          else: context

      request = Checkpoint.request(node, %{"value" => 41}, context)
      {:ok, saved, :retained} = Checkpoint.put(recovery, request, 42, claim)
      {:ok, replay, next} = Envelope.claim(saved, "new-owner", :process_restart)
      assert {:ok, 42} = Checkpoint.lookup(replay, request, next)
      context = Map.put(context, "workflow_input_binding_sha256", String.duplicate("b", 64))
      changed = Checkpoint.request(node, %{"value" => 41}, context)
      assert {:error, :input_changed} = Checkpoint.lookup(replay, changed, next)
      assert replay[Checkpoint.key()] == saved[Checkpoint.key()]
    end
  end

  for field <- ~w(schema_version job_id envelope_sha256 sha256 entries) do
    test "invalid bucket #{field} never becomes a cache miss" do
      {recovery, claim} = Fixture.running()
      {:ok, saved, :retained} = Checkpoint.put(recovery, Fixture.request(recovery), 42, claim)
      corrupt = put_in(saved, [Checkpoint.key(), unquote(field)], "invalid")
      assert {:error, :invalid} = Checkpoint.verify(corrupt, "parent")
      assert {:error, :invalid} = Checkpoint.lookup(corrupt, Fixture.request(recovery), claim)
      assert {:error, :invalid} = Checkpoint.put(corrupt, Fixture.request(recovery), 42, claim)
    end
  end

  for field <- ~w(node_id node_sha256 invocation_sha256 result_sha256 result_json producer_claim) do
    test "a resealed bucket with invalid entry #{field} is still rejected" do
      {recovery, claim} = Fixture.running()
      {:ok, saved, :retained} = Checkpoint.put(recovery, Fixture.request(recovery), 42, claim)
      bucket = put_in(saved[Checkpoint.key()], ["entries", "solve-1", unquote(field)], "invalid")
      corrupt = Map.put(saved, Checkpoint.key(), reseal(bucket))
      assert {:error, :invalid} = Checkpoint.verify(corrupt, "parent")
    end
  end

  test "closed fields, future producers and foreign parents are rejected" do
    {recovery, claim} = Fixture.running()
    {:ok, saved, :retained} = Checkpoint.put(recovery, Fixture.request(recovery), 42, claim)
    bucket = saved[Checkpoint.key()]

    for corrupt <- [
          Map.put(bucket, "unexpected", true),
          put_in(bucket, ["entries", "solve-1", "unexpected"], true),
          put_in(bucket, ["entries", "solve-1", "producer_claim", "generation"], 99),
          put_in(bucket, ["entries", "solve-1", "producer_claim", "attempt"], 99)
        ] do
      assert {:error, :invalid} =
               Checkpoint.verify(Map.put(saved, Checkpoint.key(), reseal(corrupt)), "parent")
    end

    assert {:error, :invalid} = Checkpoint.verify(saved, "foreign")

    for invalid <- [nil, [], %{}, %{"extra" => true}] do
      assert {:error, :invalid} = Checkpoint.put(recovery, invalid, 42, claim)
    end
  end

  test "output and entry budgets omit new artifacts without evicting old ones" do
    {recovery, claim} = Fixture.running(33)
    large = [String.duplicate("x", 270_000), String.duplicate("x", 270_000)]

    assert {:ok, ^recovery, :not_retained} =
             Checkpoint.put(recovery, Fixture.request(recovery), large, claim)

    saved =
      Enum.reduce(1..32, recovery, fn i, acc ->
        assert {:ok, next, :retained} =
                 Checkpoint.put(acc, Fixture.request(recovery, i), i, claim)

        next
      end)

    assert {:ok, ^saved, :not_retained} =
             Checkpoint.put(saved, Fixture.request(recovery, 33), 33, claim)

    assert map_size(saved[Checkpoint.key()]["entries"]) == 32
    assert :ok = Checkpoint.verify(saved, "parent")
  end

  test "total byte and decoded output JSON-node budgets are enforced before retention" do
    {recovery, claim} = Fixture.running(5)

    saved =
      Enum.reduce(1..4, recovery, fn i, acc ->
        assert {:ok, next, :retained} =
                 Checkpoint.put(
                   acc,
                   Fixture.request(recovery, i),
                   String.duplicate("x", 450_000),
                   claim
                 )

        next
      end)

    assert {:ok, ^saved, :not_retained} =
             Checkpoint.put(
               saved,
               Fixture.request(recovery, 5),
               String.duplicate("x", 450_000),
               claim
             )

    dense = List.duplicate(0, 249_990)

    assert {:ok, ^recovery, :not_retained} =
             Checkpoint.put(recovery, Fixture.request(recovery), dense, claim)

    assert :ok = Checkpoint.verify(saved, "parent")
  end

  test "terminal pruning and public summaries never expose checkpoint payloads" do
    {recovery, claim} = Fixture.running()

    {:ok, saved, :retained} =
      Checkpoint.put(recovery, Fixture.request(recovery), %{"secret_artifact" => 42}, claim)

    public = Envelope.public_result(%{Envelope.internal_key() => saved})
    assert public["recovery"]["solve_checkpoint_count"] == 1
    assert public["recovery"]["solve_checkpoint_policy"]["max_bytes"] == 2_097_152
    refute Jason.encode!(public) =~ "secret_artifact"

    for state <- ~w(completed failed cancelled) do
      ended = Envelope.transition(saved, state)
      refute Map.has_key?(ended, Checkpoint.key())
      refute Map.has_key?(ended, "envelope")
    end

    blocked = Envelope.transition(saved, "recovery_blocked")
    assert blocked[Checkpoint.key()] == saved[Checkpoint.key()]
  end

  test "malformed buckets remain safely inspectable without pretending they are valid" do
    {recovery, _claim} = Fixture.running()

    for malformed <- [nil, "broken", [], %{"entries" => "broken"}] do
      damaged = Map.put(recovery, Checkpoint.key(), malformed)
      assert {:error, :invalid} = Checkpoint.verify(damaged, "parent")
      public = Envelope.public_result(%{Envelope.internal_key() => damaged})
      assert public["recovery"]["solve_checkpoint_count"] == 0
      refute Map.has_key?(public, Envelope.internal_key())
    end
  end

  test "callbacks fence both execution and publication, and solver errors are not retained" do
    node = Fixture.solve_node()
    callback = fn {:lookup, _} -> {:error, :input_changed} end

    assert {:error, {:workflow_solve_checkpoint_blocked, :input_changed}} =
             Checkpoint.run(node, %{}, %{}, [solve_checkpoint: callback], fn ->
               flunk("must not execute")
             end)

    callback = fn
      {:lookup, _} -> :execute
      {:commit, _, _} -> {:error, :storage_unavailable}
    end

    assert {:error, {:workflow_solve_checkpoint_blocked, :storage_unavailable}} =
             Checkpoint.run(node, %{}, %{}, [solve_checkpoint: callback], fn -> {:ok, 42} end)

    callback = fn {:lookup, _} -> :execute end

    assert {:error, :solver_failure} =
             Checkpoint.run(node, %{}, %{}, [solve_checkpoint: callback], fn ->
               {:error, :solver_failure}
             end)

    assert {:ok, 42} = Checkpoint.run(node, %{}, %{}, [], fn -> {:ok, 42} end)
  end

  defp reseal(bucket) do
    sha =
      bucket
      |> Map.delete("sha256")
      |> DurableJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    Map.put(bucket, "sha256", sha)
  end
end
