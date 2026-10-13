defmodule KyuubikiWeb.Orchestra.OperatorDispatchRetentionTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.OperatorDispatchRetention, as: Retention
  alias KyuubikiWeb.{AnalysisResultStore, Storage}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Storage.DispatchRetentionStore
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  setup do
    Fixture.setup_coordinator()

    root =
      Path.join(System.tmp_dir!(), "dispatch-retention-#{System.unique_integer([:positive])}")

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    %{root: root, journal: journal}
  end

  test "blocked, orphaned and malformed recovery records remain protected", ctx do
    # SQL foreign keys cascade job deletion; only memory storage can retain this orphan.
    attempts =
      for change <- [
            :blocked,
            :missing_job,
            :missing_result,
            :null,
            :array,
            :unknown_schema,
            :unknown_state,
            :mismatched_terminal,
            :retained_envelope,
            :retained_evidence,
            :retained_native_result
          ],
          change != :missing_job or not Storage.sql?() do
        f = Fixture.claimed_job()
        attempt = terminal(ctx.journal, f.id)
        recovery = f.runtime[Envelope.internal_key()]
        ended = Envelope.transition(recovery, "failed")

        modified =
          case change do
            :blocked -> Envelope.transition(recovery, "recovery_blocked")
            :missing_job -> recovery
            :missing_result -> recovery
            :null -> nil
            :array -> []
            :unknown_schema -> Map.put(ended, "schema_version", "future")
            :unknown_state -> Map.put(ended, "state", "future")
            :mismatched_terminal -> Envelope.transition(recovery, "completed")
            :retained_envelope -> Map.put(ended, "envelope", nil)
            :retained_evidence -> Map.put(ended, "native_dispatch_evidence", nil)
            :retained_native_result -> Map.put(ended, "native_result_checkpoints", nil)
          end

        assert :ok =
                 AnalysisResultStore.put(
                   f.id,
                   Map.put(f.runtime, Envelope.internal_key(), modified)
                 )

        case change do
          :missing_job ->
            assert {:ok, _} = Store.delete(f.id)

          :missing_result ->
            assert {:ok, _} = AnalysisResultStore.delete(f.id)

          _ ->
            assert {:ok, _} = Store.apply_progress(%{job_id: f.id, stage: :failed, progress: 1.0})
        end

        attempt
      end

    for _ <- 1..140, do: terminal(ctx.journal, "standalone")
    assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
    retained = MapSet.new(records, & &1["attempt_id"])
    assert Enum.all?(attempts, &MapSet.member?(retained, &1))
    assert length(records) == length(attempts) + 128
  end

  test "explicit joint deletion releases history instead of leaving a permanent pin", ctx do
    f = Fixture.claimed_job()
    attempt = terminal(ctx.journal, f.id)
    assert {:ok, _} = Store.delete_with_result(f.id)
    for _ <- 1..140, do: terminal(ctx.journal, "standalone")
    assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
    refute Enum.any?(records, &(&1["attempt_id"] == attempt))
    assert length(records) == 128
  end

  for status <- [:completed, :failed, :cancelled] do
    test "#{status} releases protected history without a journal restart", ctx do
      f = Fixture.claimed_job()
      attempt = terminal(ctx.journal, f.id)
      for _ <- 1..130, do: terminal(ctx.journal, "standalone")
      finish_parent(f, unquote(status))
      terminal(ctx.journal, "standalone")
      assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
      refute Enum.any?(records, &(&1["attempt_id"] == attempt))
      assert length(records) == 128
    end
  end

  test "protected history cannot exceed capacity and terminal release restores admission", ctx do
    f = Fixture.claimed_job()
    for _ <- 1..512, do: terminal(ctx.journal, f.id)
    before = files(ctx.root)

    assert {:error, :operator_task_dispatch_journal_full} =
             Journal.begin_dispatch(request("standalone"), endpoint(), ctx.journal)

    assert files(ctx.root) == before

    finish_parent(f, :completed)
    terminal(ctx.journal, "standalone")
    assert length(File.ls!(ctx.root)) == 128
  end

  test "snapshot queries are bounded and return no result payload", _ctx do
    f = Fixture.claimed_job()
    assert {:ok, parents} = DispatchRetentionStore.snapshot([f.id, f.id, "absent"])
    assert parents[f.id] == %{job_status: "solving", recovery_present: true, released_state: nil}
    assert parents["absent"] == %{job_status: nil, recovery_present: false, released_state: nil}
    assert map_size(parents) == 2
    assert {:ok, %{}} = DispatchRetentionStore.snapshot([])

    for ids <- [nil, [nil], [""], [String.duplicate("a", 1025)], List.duplicate("a", 513)] do
      assert {:error, :invalid_dispatch_retention_query} = DispatchRetentionStore.snapshot(ids)
    end
  end

  test "pruning a stale snapshot never deletes a later terminal observation", ctx do
    for _ <- 1..128, do: terminal(ctx.journal, "standalone")
    {:ok, candidates} = GenServer.call(ctx.journal, :retention_candidates)
    {:ok, allowed} = Retention.releasable(candidates)
    late = terminal(ctx.journal, "standalone")
    {:ok, [sample | _]} = Journal.lookup_task("task", ctx.journal)

    new = %{
      sample
      | "attempt_id" => String.duplicate("f", 32),
        "state" => "dispatch_boundary_unconfirmed"
    }

    assert {:ok, _} = GenServer.call(ctx.journal, {:begin, new, allowed})
    assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
    assert Enum.any?(records, &(&1["attempt_id"] == late))
    assert Enum.any?(records, &(&1["attempt_id"] == new["attempt_id"]))
  end

  test "temporary storage loss neither prunes nor quarantines a healthy journal", ctx do
    for _ <- 1..128, do: terminal(ctx.journal, "standalone")
    before = files(ctx.root)

    unavailable_store(fn ->
      assert {:error, :operator_task_dispatch_journal_unavailable} =
               Journal.begin_dispatch(request("standalone"), endpoint(), ctx.journal)

      assert files(ctx.root) == before
      assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
      assert length(records) == 128
    end)

    terminal(ctx.journal, "standalone")
    assert length(File.ls!(ctx.root)) == 128
  end

  @tag skip: not Storage.sql?()
  test "a saturated single-connection pool cannot lock journal readers behind pruning", ctx do
    for _ <- 1..128, do: terminal(ctx.journal, "standalone")
    repo = Storage.repo_module!()
    selected = start_supervised!({repo, name: nil, pool_size: 1}, id: :retention_pool)
    previous = apply(repo, :put_dynamic_repo, [selected])

    try do
      assert {:ok, task} =
               apply(repo, :transaction, [
                 fn ->
                   task =
                     Task.async(fn ->
                       apply(repo, :put_dynamic_repo, [selected])
                       Journal.begin_dispatch(request("standalone"), endpoint(), ctx.journal)
                     end)

                   assert Task.yield(task, 50) == nil
                   assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
                   assert length(records) == 128
                   task
                 end
               ])

      assert {:ok, _} = Task.await(task)
    after
      apply(repo, :put_dynamic_repo, [previous])
    end
  end

  defp unavailable_store(callback) do
    if Storage.sql?() do
      repo = Storage.repo_module!()
      previous = apply(repo, :put_dynamic_repo, [:missing_retention_repo])

      try do
        callback.()
      after
        apply(repo, :put_dynamic_repo, [previous])
      end
    else
      store = KyuubikiWeb.Storage.AnalysisMemoryState
      :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, store)

      try do
        callback.()
      after
        {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, store)
      end
    end
  end

  defp finish_parent(f, status) do
    recovery = Envelope.transition(f.runtime[Envelope.internal_key()], to_string(status))
    result = Map.put(f.runtime, Envelope.internal_key(), recovery)

    assert {:ok, _} =
             Store.apply_progress_with_result(
               %{job_id: f.id, stage: status, progress: 1.0},
               f.job,
               f.runtime,
               result
             )
  end

  defp files(root), do: Map.new(File.ls!(root), &{&1, File.read!(Path.join(root, &1))})

  test "active parent evidence survives unrelated history turnover and journal restart", ctx do
    f = Fixture.claimed_job()
    attempt = terminal(ctx.journal, f.id)
    for _ <- 1..140, do: terminal(ctx.journal, "standalone")

    assert {:ok, records} = Journal.lookup_task("task", ctx.journal)
    assert Enum.any?(records, &(&1["attempt_id"] == attempt))
    assert length(records) == 129

    assert :ok = stop_supervised(Journal)
    restarted = start_supervised!({Journal, root: ctx.root, name: nil})
    for _ <- 1..140, do: terminal(restarted, "standalone")
    assert {:ok, records} = Journal.lookup_task("task", restarted)
    assert Enum.any?(records, &(&1["attempt_id"] == attempt))
    assert length(File.ls!(ctx.root)) == 129
  end

  defp terminal(journal, job_id) do
    assert {:ok, id} = Journal.begin_dispatch(request(job_id), endpoint(), journal)
    assert :ok = Journal.finish_dispatch(id, "not_dispatched", journal)
    id
  end

  defp endpoint, do: %{id: "owner", host: "127.0.0.1", port: 5001}

  defp request(job_id) do
    %{
      "id" => "request",
      "params" => %{
        "job_id" => job_id,
        "task_ir" => %{
          "task_id" => "task",
          "integrity" => %{"task_digest" => String.duplicate("a", 64)},
          "operator" => %{"id" => "solve.bar_1d"},
          "execution_program" => %{"program_id" => "native-bar"}
        }
      }
    }
  end
end
