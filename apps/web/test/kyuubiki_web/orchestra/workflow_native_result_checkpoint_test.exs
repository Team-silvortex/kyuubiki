defmodule KyuubikiWeb.Orchestra.WorkflowNativeResultCheckpointTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowNativeResultCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryDispatchGuard, as: Guard

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskIR,
    LeaseStore,
    OperatorTaskExecutionSummary,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()
    root = Path.join(System.tmp_dir!(), "native-checkpoint-#{System.unique_integer([:positive])}")
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})

    on_exit(fn ->
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    %{root: root}
  end

  test "handoff commits privately and losslessly, then a new generation reuses it read-only" do
    f = fixture()
    assert :missing = access(f, {:lookup, f.query})
    assert :ok = access(f, {:commit, f.query, f.receipt})
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    {:ok, job} = Store.get(f.id)
    saved = %{f | runtime: runtime, job: job}
    assert runtime["artifacts"] == %{}
    assert runtime["progress_events"] == []
    assert :ok = access(f, {:commit, f.query, f.receipt})
    Fixture.unchanged(saved)
    assert {:error, :conflict} = access(f, {:lookup, f.query})
    resumed = advance(saved)
    assert {:ok, receipt} = access(resumed, {:lookup, f.query})
    assert receipt === f.receipt
    Fixture.unchanged(resumed)
    {:ok, public} = Analysis.fetch_job(f.id)
    assert public["result"]["recovery"]["native_result_checkpoint_count"] == 1
    assert public["result"]["recovery"]["native_result_checkpoint_policy"] == Checkpoint.policy()
    refute Map.has_key?(public["result"], Envelope.internal_key())
    refute Jason.encode!(public) =~ f.query["attempt_id"]
    assert {:error, :active_workflow_result_is_read_only} = Analysis.update_result(f.id, %{})
  end

  for field <- ~w(generation attempt owner_session_id) do
    test "stale #{field} cannot commit or read a native receipt" do
      f = fixture()

      stale =
        Map.update!(f.claim, unquote(field), fn x ->
          if is_integer(x), do: x + 1, else: x <> "x"
        end)

      for action <- [{:lookup, f.query}, {:commit, f.query, f.receipt}] do
        assert {:error, :stale_workflow_execution_claim} =
                 Coordinator.native_result_checkpoint(f.id, stale, action)

        Fixture.unchanged(f)
      end
    end
  end

  for state <- ~w(completed failed cancelled) do
    test "#{state} removes the bounded native receipt, while recovery_blocked preserves it" do
      f = fixture()
      assert :ok = access(f, {:commit, f.query, f.receipt})
      {:ok, runtime} = AnalysisResultStore.get(f.id)
      recovery = runtime[Envelope.internal_key()]
      refute Map.has_key?(Envelope.transition(recovery, unquote(state)), Checkpoint.key())

      assert Envelope.transition(recovery, "recovery_blocked")[Checkpoint.key()] ==
               recovery[Checkpoint.key()]
    end
  end

  test "conflicting results, observation drift and unbound attempts never overwrite evidence" do
    f = fixture()
    assert :ok = access(f, {:commit, f.query, f.receipt})
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    {:ok, job} = Store.get(f.id)
    saved = %{f | runtime: runtime, job: job}

    assert {:error, :conflict} =
             access(f, {:commit, f.query, put_in(f.receipt, ["result", "value"], 2.0)})

    for field <- ~w(task_digest attempt_id operator_id program_id observation_sha256) do
      query =
        Map.put(
          f.query,
          field,
          String.duplicate("f", if(field == "attempt_id", do: 32, else: 64))
        )

      assert {:error, _} = access(f, {:commit, query, f.receipt})
      Fixture.unchanged(saved)
    end

    resumed = advance(saved)

    assert {:error, :conflict} =
             access(
               resumed,
               {:lookup, Map.put(f.query, "observation_sha256", String.duplicate("f", 64))}
             )

    Fixture.unchanged(resumed)
  end

  for fault <- [:digest, :receipt, :parent, :envelope, :producer, :owner, :identity, :extra] do
    test "#{fault} checkpoint blocks admission before advancing generation" do
      f = fixture()
      assert :ok = access(f, {:commit, f.query, f.receipt})
      {:ok, runtime} = AnalysisResultStore.get(f.id)
      recovery = runtime[Envelope.internal_key()]
      bucket = recovery[Checkpoint.key()]
      id = f.query["attempt_id"]

      changed =
        case unquote(fault) do
          :digest ->
            Map.put(bucket, "sha256", "bad")

          :receipt ->
            put_in(bucket, ["entries", id, "receipt_json"], "{}") |> seal()

          :parent ->
            Map.put(bucket, "job_id", "other-parent") |> seal()

          :envelope ->
            Map.put(bucket, "envelope_sha256", String.duplicate("0", 64)) |> seal()

          :producer ->
            put_in(bucket, ["entries", id, "producer_claim", "generation"], 100) |> seal()

          :owner ->
            put_in(bucket, ["entries", id, "producer_claim", "owner_session_id"], "foreign")
            |> seal()

          :identity ->
            put_in(bucket, ["entries", id, "operator_id"], "foreign") |> seal()

          :extra ->
            Map.put(bucket, "unexpected", "SECRET") |> seal()
        end

      corrupt = Map.put(recovery, Checkpoint.key(), changed)

      assert {:error, :workflow_native_result_checkpoint_invalid} =
               Guard.claim(f.id, corrupt, "restart", :runner_loss, 3)

      assert :ok =
               AnalysisResultStore.put(f.id, Map.put(runtime, Envelope.internal_key(), corrupt))

      assert %{"blocked" => 1, "recovered" => 0} = Coordinator.recover_now()
      {:ok, ended} = AnalysisResultStore.get(f.id)
      assert ended[Envelope.internal_key()]["generation"] == 1
      assert ended[Envelope.internal_key()]["state"] == "recovery_blocked"
      assert ended[Envelope.internal_key()][Checkpoint.key()] == changed
    end
  end

  test "oversize success is not retained and cannot replace an already committed receipt" do
    f = fixture()

    large =
      put_in(f.receipt, ["result", "padding"], List.duplicate(String.duplicate("x", 270_000), 2))

    assert :ok = access(f, {:commit, f.query, large})
    Fixture.unchanged(f)
    assert :ok = access(f, {:commit, f.query, f.receipt})
    assert {:error, :conflict} = access(f, {:commit, f.query, large})
  end

  test "failed, mismatched and malformed receipts are never handed off" do
    f = fixture()

    for receipt <- [
          nil,
          %{},
          Map.put(f.receipt, "task_digest", String.duplicate("f", 64)),
          Map.put(f.receipt, "operator_task_ir_status", "failed"),
          put_in(f.receipt, ["result", "bad"], self())
        ] do
      assert {:error, :invalid} = access(f, {:commit, f.query, receipt})
      Fixture.unchanged(f)
    end
  end

  test "entry and aggregate byte caps do not evict existing completed evidence" do
    for {count, padding} <- [{32, 0}, {4, 440_000}] do
      f = fixture()
      receipt = put_in(f.receipt, ["result", "padding"], String.duplicate("x", padding))
      assert :ok = access(f, {:commit, f.query, receipt})

      for _ <- 2..count do
        next = next_attempt(f)
        assert :ok = access(next, {:commit, next.query, receipt})
      end

      overflow = next_attempt(f)
      {:ok, before} = AnalysisResultStore.get(f.id)
      assert :ok = access(overflow, {:commit, overflow.query, receipt})
      assert {:ok, ^before} = AnalysisResultStore.get(f.id)
      assert map_size(before[Envelope.internal_key()][Checkpoint.key()]["entries"]) == count
      assert :ok = Checkpoint.verify(before[Envelope.internal_key()], f.id)
    end
  end

  test "a retained native result cannot bypass missing dispatch history", ctx do
    f = fixture()
    assert :ok = access(f, {:commit, f.query, f.receipt})
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    :ok = stop_supervised(Journal)
    File.rm_rf!(ctx.root)
    start_supervised!({Journal, root: ctx.root})

    assert {:error, {:workflow_dispatch_journal_unavailable, :workflow_dispatch_evidence_missing}} =
             Guard.claim(f.id, runtime[Envelope.internal_key()], "restart", :runner_loss, 3)
  end

  test "loss of the orchestration lease cannot read or write a native checkpoint" do
    f = fixture()
    name = Coordinator.snapshot()["lease"]["lease_name"]
    {:ok, previous} = LeaseStore.current(name)
    assert :ok = LeaseStore.release(previous)
    {:ok, takeover} = LeaseStore.acquire(name, "different-owner", 60_000)
    on_exit(fn -> LeaseStore.release(takeover) end)
    assert {:error, :orchestra_lease_lost} = access(f, {:lookup, f.query})
    assert {:error, _} = access(f, {:commit, f.query, f.receipt})
    Fixture.unchanged(f)
  end

  test "rejected job-result transactions cannot leave a falsely committed receipt" do
    f = fixture()

    {:ok, future} =
      Store.apply_progress(%{
        job_id: f.id,
        stage: :solving,
        progress: 0.5,
        emitted_at: DateTime.add(DateTime.utc_now(), 60, :second)
      })

    assert {:error, _} = access(f, {:commit, f.query, f.receipt})
    Fixture.unchanged(%{f | job: future})
  end

  for reason <- [
        :result_checkpoint_invalid,
        :result_checkpoint_unavailable,
        :result_checkpoint_commit_failed
      ] do
    test "#{reason} retains private handoff evidence with a specific recovery action" do
      f = fixture()
      assert :ok = access(f, {:commit, f.query, f.receipt})
      {:ok, before} = AnalysisResultStore.get(f.id)
      assert :ok = Coordinator.block_native_recovery(f.id, f.claim, unquote(reason))
      {:ok, after_block} = AnalysisResultStore.get(f.id)
      key = Envelope.internal_key()
      assert after_block[key]["state"] == "recovery_blocked"
      assert after_block[key][Checkpoint.key()] == before[key][Checkpoint.key()]

      assert List.last(after_block[key]["history"])["next_action"] ==
               "inspect_native_result_checkpoint_before_explicit_resubmit"

      assert Coordinator.recover_now()["recovered"] == 0
    end
  end

  defp fixture do
    operator = "extract.thermal_result_diagnostics"

    f =
      Fixture.claimed_job(
        graph: %{
          "id" => "native-handoff",
          "nodes" => [
            %{"id" => "diagnose", "kind" => "extract", "operator_id" => operator}
          ]
        }
      )

    {:ok, task} =
      OperatorTaskIR.build(operator, %{}, %{}, task_id: "#{f.id}:diagnose:#{operator}")

    {:ok, summary} = OperatorTaskExecutionSummary.build(task)

    receipt =
      Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
        "operator_task_ir_status" => "executed",
        "execution_readiness" => OperatorTaskReadiness.local_executed(),
        "result" => %{"value" => 1.0, "tiny" => 1.0e-30}
      })

    next_attempt(
      Map.merge(f, %{
        receipt: receipt,
        summary: summary,
        rpc: %{"id" => f.id, "params" => %{"task_ir" => task, "job_id" => f.id}}
      })
    )
  end

  defp next_attempt(f) do
    {:ok, attempt} = Journal.begin_dispatch(f.rpc, %{id: "owner", host: "127.0.0.1", port: 1})

    assert :ok =
             Coordinator.native_dispatch_evidence(
               f.id,
               f.claim,
               {:commit, Map.take(f.summary, ~w(task_id task_digest operator_id program_id)),
                attempt}
             )

    assert :ok = Journal.finish_dispatch(attempt, "observed_executed")
    {:ok, records} = Journal.lookup_task(f.summary["task_id"])
    record = Enum.find(records, &(&1["attempt_id"] == attempt))
    {:ok, runtime} = AnalysisResultStore.get(f.id)
    {:ok, job} = Store.get(f.id)
    Map.merge(f, %{query: Checkpoint.request(record), runtime: runtime, job: job})
  end

  defp advance(f) do
    key = Envelope.internal_key()

    {:ok, recovery, claim} =
      Envelope.claim(f.runtime[key], f.claim["owner_session_id"], :runner_loss)

    runtime = Map.put(f.runtime, key, recovery)
    assert :ok = AnalysisResultStore.put(f.id, runtime)
    %{f | runtime: runtime, claim: claim}
  end

  defp access(f, action), do: Coordinator.native_result_checkpoint(f.id, f.claim, action)

  defp seal(bucket),
    do:
      Map.put(
        bucket,
        "sha256",
        :crypto.hash(:sha256, DurableJson.encode!(Map.delete(bucket, "sha256")))
        |> Base.encode16(case: :lower)
      )
end
