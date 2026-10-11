defmodule KyuubikiWeb.Orchestra.WorkflowBranchCheckpointTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.WorkflowBranchCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.TestSupport.WorkflowBranchCheckpointFixture, as: Fixture

  test "later claims recover only the stored decision, without reevaluation or payload retention" do
    fixture = Fixture.running()
    node = hd(fixture.nodes)
    payload = %{"value" => 1.0, "secret" => "not-in-record"}
    req = Checkpoint.request(node, payload, fixture.context)
    decision = Fixture.decision()

    assert {:ok, saved, :retained} =
             Checkpoint.put(fixture.recovery, req, decision, fixture.claim)

    assert {:error, :conflict} = Checkpoint.lookup(saved, req, fixture.claim)
    assert :ok = Checkpoint.verify(saved, "branch-job")
    refute DurableJson.encode!(saved[Checkpoint.key()]) =~ "not-in-record"
    roundtrip = saved |> DurableJson.encode!() |> Jason.decode!()
    {:ok, resumed, claim} = Envelope.claim(roundtrip, "next-owner", :process_restart)
    assert {:ok, ^decision} = Checkpoint.lookup(resumed, req, claim)
    callback = fn {:lookup, ^req} -> Checkpoint.lookup(resumed, req, claim) end

    assert {:ok, ^decision} =
             Checkpoint.run(node, payload, fixture.context, [branch_checkpoint: callback], fn ->
               flunk("must not reevaluate the predicate")
             end)

    assert resumed[Checkpoint.key()] == saved[Checkpoint.key()]
  end

  for payload <- [
        %{"value" => -1.0},
        %{"value" => 1},
        %{"value" => 0.0},
        %{"value" => -0.0},
        %{"value" => 1.0e-30}
      ] do
    test "changed lossless input #{inspect(payload)} cannot silently select a new branch" do
      fixture = Fixture.running()
      node = hd(fixture.nodes)
      req = Checkpoint.request(node, %{"value" => 1.0}, fixture.context)

      {:ok, saved, :retained} =
        Checkpoint.put(fixture.recovery, req, Fixture.decision(), fixture.claim)

      {:ok, resumed, claim} = Envelope.claim(saved, "next-owner", :runner_loss)
      changed = Checkpoint.request(node, unquote(Macro.escape(payload)), fixture.context)
      assert {:error, :input_changed} = Checkpoint.lookup(resumed, changed, claim)
    end
  end

  test "signed zero and tiny differences produce distinct invocation identities" do
    node = Fixture.condition()

    for {a, b} <- [{0.0, -0.0}, {1.0e-30, 2.0e-30}] do
      refute Checkpoint.request(node, a, %{}) == Checkpoint.request(node, b, %{})
    end
  end

  test "equal inputs cannot reuse a decision with changed or previously unbound provenance" do
    for original_binding <- [nil, String.duplicate("a", 64)] do
      fixture = Fixture.running()
      node = hd(fixture.nodes)

      context =
        if original_binding,
          do: Map.put(fixture.context, "workflow_input_binding_sha256", original_binding),
          else: fixture.context

      request = Checkpoint.request(node, %{"value" => 1.0}, context)
      decision = Fixture.decision()

      {:ok, saved, :retained} =
        Checkpoint.put(fixture.recovery, request, decision, fixture.claim)

      {:ok, resumed, claim} = Envelope.claim(saved, "next-owner", :process_restart)
      assert {:ok, ^decision} = Checkpoint.lookup(resumed, request, claim)
      context = Map.put(context, "workflow_input_binding_sha256", String.duplicate("b", 64))
      changed = Checkpoint.request(node, %{"value" => 1.0}, context)
      assert {:error, :input_changed} = Checkpoint.lookup(resumed, changed, claim)
      assert resumed[Checkpoint.key()] == saved[Checkpoint.key()]
    end
  end

  for field <- ~w(job_id envelope_sha256 schema_version sha256) do
    test "a changed #{field} cannot be interpreted as a missing checkpoint" do
      fixture = Fixture.running()
      req = Checkpoint.request(hd(fixture.nodes), 1, fixture.context)

      {:ok, saved, :retained} =
        Checkpoint.put(fixture.recovery, req, Fixture.decision(), fixture.claim)

      corrupt = put_in(saved, [Checkpoint.key(), unquote(field)], "bad")
      assert {:error, :invalid} = Checkpoint.verify(corrupt, "branch-job")
    end
  end

  test "resealed invalid entries still fail structural and branch-port checks" do
    fixture = Fixture.running()
    node = hd(fixture.nodes)
    req = Checkpoint.request(node, 1, fixture.context)

    {:ok, saved, :retained} =
      Checkpoint.put(fixture.recovery, req, Fixture.decision(), fixture.claim)

    entry = saved[Checkpoint.key()]["entries"][node["id"]]

    for changed <- [
          Map.put(entry, "extra", true),
          Map.put(entry, "node_id", "another"),
          put_in(entry, ["producer_claim", "generation"], 2),
          put_in(entry, ["producer_claim", "attempt"], 2),
          put_in(entry, ["decision", "chosen_output"], "if_false"),
          put_in(entry, ["decision", "predicate_result"], "true"),
          Map.put(entry, "decision", Map.put(Fixture.decision(), "payload", "forbidden"))
        ] do
      bucket = put_in(saved[Checkpoint.key()], ["entries", node["id"]], changed) |> seal()

      assert {:error, :invalid} =
               Checkpoint.verify(Map.put(saved, Checkpoint.key(), bucket), "branch-job")
    end
  end

  test "node configuration, output layout and authority are bound to each decision" do
    fixture = Fixture.running()
    node = hd(fixture.nodes)
    req = Checkpoint.request(node, 1, fixture.context)

    {:ok, saved, :retained} =
      Checkpoint.put(fixture.recovery, req, Fixture.decision(), fixture.claim)

    {:ok, resumed, claim} = Envelope.claim(saved, "next-owner", :runner_loss)

    for changed <- [
          Map.put(node, "config", %{}),
          Map.put(node, "outputs", Enum.reverse(node["outputs"])),
          Map.put(node, "kind", "solve")
        ] do
      assert {:error, :invalid} =
               Checkpoint.lookup(resumed, Checkpoint.request(changed, 1, fixture.context), claim)
    end

    assert {:error, :input_changed} =
             Checkpoint.lookup(
               resumed,
               Checkpoint.request(node, 1, Map.put(fixture.context, "orch_id", "other")),
               claim
             )
  end

  test "entry and byte limits omit new records without evicting existing decisions" do
    fixture = Fixture.running(65)

    saved =
      Enum.reduce(Enum.take(fixture.nodes, 64), fixture.recovery, fn node, acc ->
        {:ok, updated, :retained} =
          Checkpoint.put(
            acc,
            Checkpoint.request(node, 1, fixture.context),
            Fixture.decision(),
            fixture.claim
          )

        updated
      end)

    req = Checkpoint.request(List.last(fixture.nodes), 1, fixture.context)

    assert {:ok, ^saved, :not_retained} =
             Checkpoint.put(saved, req, Fixture.decision(), fixture.claim)

    assert :ok = Checkpoint.verify(saved, "branch-job")
    long_port = String.duplicate("x", 66_000)
    node = Map.put(hd(fixture.nodes), "outputs", [%{"id" => long_port}])
    recovery = put_in(fixture.recovery, ["envelope", "graph", "nodes"], [node])
    req = Checkpoint.request(node, 1, fixture.context)

    assert {:ok, ^recovery, :not_retained} =
             Checkpoint.put(
               recovery,
               req,
               %{"chosen_output" => long_port, "predicate_result" => true},
               fixture.claim
             )
  end

  for state <- ~w(completed failed cancelled) do
    test "#{state} prunes branch decisions and the private execution envelope" do
      fixture = Fixture.running()
      req = Checkpoint.request(hd(fixture.nodes), 1, fixture.context)

      {:ok, saved, :retained} =
        Checkpoint.put(fixture.recovery, req, Fixture.decision(), fixture.claim)

      ended = Envelope.transition(saved, unquote(state))
      refute Map.has_key?(ended, Checkpoint.key())
      refute Map.has_key?(ended, "envelope")
      assert Envelope.public_summary(ended)["branch_checkpoint_count"] == 0
    end
  end

  test "damaged buckets are safe to inspect but never authorize recovery" do
    fixture = Fixture.running()

    for bucket <- [nil, "bad", [], %{"entries" => "bad"}] do
      saved = Map.put(fixture.recovery, Checkpoint.key(), bucket)
      assert Envelope.public_summary(saved)["branch_checkpoint_count"] == 0
      assert {:error, :invalid} = Checkpoint.verify(saved, "branch-job")
    end
  end

  test "invalid callback responses, invalid decisions and commit errors fail closed" do
    node = Fixture.condition()

    for callback <- [
          :bad,
          fn _ -> :bad end,
          fn _ -> {:ok, Fixture.decision(false) |> Map.put("chosen_output", "if_true")} end,
          fn
            {:lookup, _} -> :execute
            {:commit, _, _} -> {:error, :disk_failed}
          end
        ] do
      assert {:error, {:workflow_branch_checkpoint_blocked, _}} =
               Checkpoint.run(node, 1, %{}, [branch_checkpoint: callback], fn ->
                 {:ok, Fixture.decision()}
               end)
    end
  end

  test "ordinary predicate failures remain failures and are never committed" do
    callback = fn
      {:lookup, _} -> :execute
      _ -> flunk("no failed decision may be retained")
    end

    assert {:error, :predicate_failed} =
             Checkpoint.run(Fixture.condition(), 1, %{}, [branch_checkpoint: callback], fn ->
               {:error, :predicate_failed}
             end)
  end

  test "invalid evaluation return shapes are blocked before a checkpoint can commit" do
    callback = fn
      {:lookup, _} -> :execute
      _ -> flunk("invalid decisions cannot commit")
    end

    for result <- [nil, :bad, %{}, {:ok, %{}}] do
      assert {:error, {:workflow_branch_checkpoint_blocked, :invalid}} =
               Checkpoint.run(Fixture.condition(), 1, %{}, [branch_checkpoint: callback], fn ->
                 result
               end)
    end
  end

  defp seal(bucket) do
    digest =
      :crypto.hash(:sha256, DurableJson.encode!(Map.delete(bucket, "sha256")))
      |> Base.encode16(case: :lower)

    Map.put(bucket, "sha256", digest)
  end
end
