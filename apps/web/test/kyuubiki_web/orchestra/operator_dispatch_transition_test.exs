defmodule KyuubikiWeb.Orchestra.OperatorDispatchTransitionTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal

  @initial "dispatch_boundary_unconfirmed"
  @observations ~w(outcome_unknown observed_executed observed_failed observed_blocked not_dispatched)

  setup do
    root =
      Path.join(System.tmp_dir!(), "dispatch-transition-#{System.unique_integer([:positive])}")

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)

    request = %{
      "id" => "rpc",
      "params" => %{
        "job_id" => "parent",
        "task_ir" => %{
          "task_id" => "task",
          "integrity" => %{"task_digest" => String.duplicate("a", 64)},
          "operator" => %{"id" => "op"},
          "execution_program" => %{"program_id" => "op"}
        }
      }
    }

    endpoint = %{id: "owner", host: "127.0.0.1", port: 1, agent_session_id: "session"}
    {:ok, attempt} = Journal.begin_dispatch(request, endpoint, journal)
    %{root: root, journal: journal, attempt: attempt, request: request, endpoint: endpoint}
  end

  for observation <- @observations do
    test "#{observation} is sealed against every conflicting observation", ctx do
      assert :ok = Journal.finish_dispatch(ctx.attempt, unquote(observation), ctx.journal)
      original = record(ctx)
      bytes = File.read!(path(ctx))

      for other <- [@initial | @observations], other != unquote(observation) do
        assert {:error, :operator_task_dispatch_transition_rejected} =
                 Journal.finish_dispatch(ctx.attempt, other, ctx.journal)

        assert record(ctx) == original
        assert File.read!(path(ctx)) == bytes
      end

      assert_usable(ctx)
      ctx = restart(ctx)
      assert record(ctx) == original
      assert File.read!(path(ctx)) == bytes
    end
  end

  for observation <- [@initial | @observations] do
    test "duplicate #{observation} verifies disk without refreshing its generation", ctx do
      assert :ok = Journal.finish_dispatch(ctx.attempt, unquote(observation), ctx.journal)
      ctx = replace_offline(ctx, &Map.merge(&1, %{"created_at_ms" => 0, "updated_at_ms" => 0}))
      original = record(ctx)
      bytes = File.read!(path(ctx))
      inode = File.stat!(path(ctx)).inode

      results =
        for _ <- 1..8 do
          Task.async(fn ->
            Journal.finish_dispatch(ctx.attempt, unquote(observation), ctx.journal)
          end)
        end
        |> Enum.map(&Task.await/1)

      assert results == List.duplicate(:ok, 8)
      assert record(ctx) == original
      assert File.read!(path(ctx)) == bytes
      assert File.stat!(path(ctx)).inode == inode
      refute File.exists?(path(ctx) <> ".next")
    end
  end

  test "malformed finish states reject only the request, not unrelated dispatches", ctx do
    original = record(ctx)
    bytes = File.read!(path(ctx))

    for invalid <- [nil, true, 1, :observed_executed, "", "SECRET", <<255>>, [], %{}] do
      assert {:error, :operator_task_dispatch_state_invalid} =
               Journal.finish_dispatch(ctx.attempt, invalid, ctx.journal)

      assert record(ctx) == original
      assert File.read!(path(ctx)) == bytes
      assert_usable(ctx)
    end

    assert :ok = Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)
  end

  test "concurrent contradictory completions seal exactly one observation across restart", ctx do
    parent = self()

    tasks =
      for state <- ~w(observed_executed outcome_unknown) do
        Task.async(fn ->
          send(parent, {:ready, self()})

          receive do: (:finish ->
                         {state, Journal.finish_dispatch(ctx.attempt, state, ctx.journal)})
        end)
      end

    for _ <- tasks, do: assert_receive({:ready, _})
    for task <- tasks, do: send(task.pid, :finish)
    results = Enum.map(tasks, &Task.await/1)
    assert [{winner, :ok}] = Enum.filter(results, fn {_, result} -> result == :ok end)

    assert [{_, {:error, :operator_task_dispatch_transition_rejected}}] =
             Enum.reject(results, fn {_, result} -> result == :ok end)

    assert record(ctx)["state"] == winner
    assert record(restart(ctx))["state"] == winner
  end

  test "unknown cannot be rewritten to remove a workflow replay fence", ctx do
    assert :ok = Journal.finish_dispatch(ctx.attempt, "outcome_unknown", ctx.journal)

    assert {:error, :operator_task_dispatch_transition_rejected} =
             Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)

    assert {:error, :workflow_dispatch_outcome_unconfirmed} =
             Journal.workflow_replay_fence("parent", ["task"], ctx.journal)

    assert_usable(ctx)
  end

  test "first observation never moves a restored timestamp backwards", ctx do
    future = System.system_time(:millisecond) + 60_000
    ctx = replace_offline(ctx, &Map.put(&1, "updated_at_ms", future))
    assert :ok = Journal.finish_dispatch(ctx.attempt, "observed_failed", ctx.journal)
    assert record(ctx)["updated_at_ms"] >= future
  end

  test "legacy records keep their unbound schema and the same one-way transitions", ctx do
    ctx =
      replace_offline(ctx, fn record ->
        record
        |> Map.delete("job_id")
        |> Map.put("schema_version", "kyuubiki.operator-task-dispatch-record/v1")
      end)

    assert :ok = Journal.finish_dispatch(ctx.attempt, "outcome_unknown", ctx.journal)
    original = record(ctx)
    assert :ok = Journal.finish_dispatch(ctx.attempt, "outcome_unknown", ctx.journal)

    assert {:error, :operator_task_dispatch_transition_rejected} =
             Journal.finish_dispatch(ctx.attempt, "not_dispatched", ctx.journal)

    assert record(restart(ctx)) == original
    assert original["schema_version"] == "kyuubiki.operator-task-dispatch-record/v1"
    refute Map.has_key?(original, "job_id")
  end

  for corruption <- [:missing, :invalid, :oversized, :replacement, :symlink, :partial] do
    test "duplicate acknowledgement cannot hide #{corruption} storage", ctx do
      assert :ok = Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)
      original = record(ctx)
      path = path(ctx)

      case unquote(corruption) do
        :missing ->
          File.rm!(path)

        :invalid ->
          File.write!(path, "broken")

        :oversized ->
          File.write!(path, String.duplicate("x", Files.limits().max_file_bytes + 1))

        :replacement ->
          assert :ok =
                   Files.write(ctx.root, Map.put(original, "state", "observed_failed"), original)

        :symlink ->
          File.rename!(path, path <> ".target")
          File.ln_s!(path <> ".target", path)

        :partial ->
          File.write!(path <> ".next", "interrupted")
      end

      before = snapshot(ctx.root)
      assert {:error, _} = Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)
      assert {:error, _} = Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)
      assert {:error, _} = Journal.lookup_task("task", ctx.journal)
      assert snapshot(ctx.root) == before
    end
  end

  test "duplicate observations do not refresh retention and evicted attempts never resurrect",
       ctx do
    assert :ok = Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)
    for _ <- 1..(Journal.policy().terminal_retention - 1), do: assert_usable(ctx)
    {:ok, records} = Journal.lookup_task("task", ctx.journal)
    assert :ok = stop_supervised(Journal)

    # Deterministic retention order, independent of same-millisecond writes and random IDs.
    for record <- records do
      updated = if record["attempt_id"] == ctx.attempt, do: 0, else: 1
      old = Map.merge(record, %{"created_at_ms" => 0, "updated_at_ms" => updated})
      assert :ok = Files.write(ctx.root, old, record)
    end

    ctx = %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}
    assert :ok = Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)
    assert_usable(ctx)

    refute File.exists?(path(ctx))

    assert {:error, :operator_task_dispatch_attempt_missing} =
             Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)

    refute File.exists?(path(ctx))
    assert_usable(ctx)
  end

  for corruption <- [:missing, :symlink] do
    test "a #{corruption} journal directory cannot be acknowledged as durable", ctx do
      assert :ok = Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)
      moved = ctx.root <> "-moved"
      File.rename!(ctx.root, moved)
      on_exit(fn -> File.rm_rf!(moved) end)
      if unquote(corruption) == :symlink, do: File.ln_s!(moved, ctx.root)
      before = snapshot(moved)

      assert {:error, :operator_task_dispatch_journal_unavailable} =
               Journal.finish_dispatch(ctx.attempt, "observed_executed", ctx.journal)

      assert {:error, _} = Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)
      assert snapshot(moved) == before
      if unquote(corruption) == :missing, do: refute(File.exists?(ctx.root))
    end
  end

  defp assert_usable(ctx) do
    assert {:ok, id} = Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)
    assert :ok = Journal.finish_dispatch(id, "not_dispatched", ctx.journal)
  end

  defp record(ctx) do
    {:ok, records} = Journal.lookup_task("task", ctx.journal)
    Enum.find(records, &(&1["attempt_id"] == ctx.attempt))
  end

  defp path(ctx), do: Path.join(ctx.root, ctx.attempt <> ".json")

  defp restart(ctx) do
    assert :ok = stop_supervised(Journal)
    %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}
  end

  defp replace_offline(ctx, transform) do
    original = record(ctx)
    assert :ok = stop_supervised(Journal)
    assert :ok = Files.write(ctx.root, transform.(original), original)
    %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}
  end

  defp snapshot(root) do
    for name <- File.ls!(root), into: %{} do
      path = Path.join(root, name)
      {name, {File.lstat!(path).type, File.read(path)}}
    end
  end
end
