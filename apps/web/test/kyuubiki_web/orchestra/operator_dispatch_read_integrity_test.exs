defmodule KyuubikiWeb.Orchestra.OperatorDispatchReadIntegrityTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal

  setup do
    root = Path.join(System.tmp_dir!(), "dispatch-read-#{System.unique_integer([:positive])}")
    journal = start_supervised!({Journal, root: root, name: nil})

    on_exit(fn ->
      File.rm_rf!(root)
      File.rm_rf!(root <> "-moved")
    end)

    endpoint = %{id: "owner", host: "127.0.0.1", port: 1}

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

    {:ok, attempt} = Journal.begin_dispatch(request, endpoint, journal)
    :ok = Journal.finish_dispatch(attempt, "observed_executed", journal)
    {:ok, [record]} = Journal.lookup_task("task", journal)
    %{root: root, journal: journal, request: request, endpoint: endpoint, record: record}
  end

  for operation <- [:lookup, :lookup_task, :fence],
      fault <- [
        :deleted,
        :corrupt,
        :replacement,
        :partial,
        :oversized,
        :symlink,
        :root_missing,
        :root_symlink
      ] do
    test "#{operation} rejects #{fault} evidence despite a healthy cached success", ctx do
      damage(ctx, unquote(fault))
      before = snapshot(ctx)
      assert {:error, _} = read(ctx, unquote(operation))
      assert {:error, _} = Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)
      assert snapshot(ctx) == before
      assert Process.alive?(ctx.journal)
    end
  end

  for operation <- [:lookup, :lookup_task, :fence, :begin],
      fault <- [:root_missing, :unexpected, :partial] do
    test "#{operation} cannot mistake #{fault} inventory for clean unrelated history", ctx do
      damage(ctx, unquote(fault))
      before = snapshot(ctx)
      assert {:error, _} = unrelated(ctx, unquote(operation))
      assert snapshot(ctx) == before
    end
  end

  test "legitimate absent and empty journals stay read-only until first dispatch", ctx do
    :ok = stop_supervised(Journal)
    File.rm_rf!(ctx.root)
    journal = start_supervised!({Journal, root: ctx.root, name: nil})
    assert {:ok, []} = Journal.lookup_task("missing", journal)
    assert {:ok, []} = Journal.lookup("missing", String.duplicate("a", 64), journal)
    assert :ok = Journal.workflow_replay_fence("parent", ["task"], journal)
    refute File.exists?(ctx.root)
    File.mkdir!(ctx.root)
    assert {:ok, []} = Journal.lookup_task("missing", journal)
    assert File.ls!(ctx.root) == []
    assert {:ok, _} = Journal.begin_dispatch(ctx.request, ctx.endpoint, journal)
  end

  test "healthy reads neither write files nor refresh timestamps or permissions", ctx do
    before = snapshot(ctx)
    inode = File.stat!(path(ctx)).inode

    for _ <- 1..4 do
      assert {:ok, [record]} = read(ctx, :lookup)
      assert record == ctx.record
      assert {:ok, [^record]} = read(ctx, :lookup_task)
      assert :ok = read(ctx, :fence)
      assert {:ok, []} = unrelated(ctx, :lookup)
    end

    assert snapshot(ctx) == before
    assert File.stat!(path(ctx)).inode == inode
  end

  test "payload checks are scoped to selected evidence, not every unrelated record", ctx do
    other = put_in(ctx.request, ["params", "task_ir", "task_id"], "other-task")
    {:ok, id} = Journal.begin_dispatch(other, ctx.endpoint, ctx.journal)
    File.write!(Path.join(ctx.root, id <> ".json"), "broken")
    assert {:ok, [record]} = read(ctx, :lookup_task)
    assert record == ctx.record
    assert {:error, _} = Journal.lookup_task("other-task", ctx.journal)
    assert {:error, _} = read(ctx, :lookup_task)
  end

  test "parent fencing verifies terminal evidence even when child task names differ", ctx do
    damage(ctx, :replacement)
    assert {:error, _} = Journal.workflow_replay_fence("parent", ["different-child"], ctx.journal)
  end

  test "legacy unbound records remain readable without migration and detect live corruption",
       ctx do
    :ok = stop_supervised(Journal)

    legacy =
      ctx.record
      |> Map.delete("job_id")
      |> Map.put("schema_version", "kyuubiki.operator-task-dispatch-record/v1")

    :ok = Files.write(ctx.root, legacy, ctx.record)
    journal = start_supervised!({Journal, root: ctx.root, name: nil})
    ctx = %{ctx | journal: journal, record: legacy}
    before = snapshot(ctx)
    assert {:ok, [^legacy]} = read(ctx, :lookup)
    assert :ok = read(ctx, :fence)
    assert snapshot(ctx) == before
    damage(ctx, :corrupt)
    assert {:error, _} = read(ctx, :fence)
  end

  test "a valid externally added record is rejected rather than silently adopted", ctx do
    extra =
      ctx.record
      |> Map.put("attempt_id", String.duplicate("f", 32))
      |> Map.put("state", "outcome_unknown")

    :ok = Files.write(ctx.root, extra)
    before = snapshot(ctx)
    assert {:error, _} = read(ctx, :lookup_task)
    assert snapshot(ctx) == before
  end

  test "storage repair requires explicit reload and preserves the original generation", ctx do
    bytes = File.read!(path(ctx))
    damage(ctx, :corrupt)
    assert {:error, _} = read(ctx, :lookup_task)
    File.write!(path(ctx), bytes)
    assert {:error, _} = read(ctx, :lookup_task)
    assert :ok = stop_supervised(Journal)
    journal = start_supervised!({Journal, root: ctx.root, name: nil})
    assert {:ok, [record]} = Journal.lookup_task("task", journal)
    assert record == ctx.record
    assert File.read!(path(ctx)) == bytes
  end

  test "full bounded inventory supports narrow and all-selected reads without retention writes",
       ctx do
    for index <- 1..(Files.limits().max_records - 1) do
      request =
        ctx.request
        |> put_in(["params", "task_ir", "task_id"], "other-task")
        |> put_in(["params", "job_id"], "other-parent-#{index}")

      assert {:ok, _} = Journal.begin_dispatch(request, ctx.endpoint, ctx.journal)
    end

    before = snapshot(ctx)
    assert {:ok, [record]} = read(ctx, :lookup_task)
    assert record == ctx.record
    assert {:ok, others} = Journal.lookup_task("other-task", ctx.journal)
    assert length(others) == Files.limits().max_records - 1
    assert :ok = read(ctx, :fence)

    assert {:error, :workflow_dispatch_outcome_unconfirmed} =
             Journal.workflow_replay_fence("other-parent-1", [], ctx.journal)

    assert {:error, :operator_task_dispatch_journal_full} =
             Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)

    assert snapshot(ctx) == before
  end

  defp read(ctx, :lookup), do: Journal.lookup("task", ctx.record["task_digest"], ctx.journal)
  defp read(ctx, :lookup_task), do: Journal.lookup_task("task", ctx.journal)
  defp read(ctx, :fence), do: Journal.workflow_replay_fence("parent", ["task"], ctx.journal)

  defp unrelated(ctx, :lookup), do: Journal.lookup("task", String.duplicate("b", 64), ctx.journal)
  defp unrelated(ctx, :lookup_task), do: Journal.lookup_task("missing", ctx.journal)

  defp unrelated(ctx, :fence),
    do: Journal.workflow_replay_fence("other-parent", ["missing"], ctx.journal)

  defp unrelated(ctx, :begin), do: Journal.begin_dispatch(ctx.request, ctx.endpoint, ctx.journal)

  defp damage(ctx, :deleted), do: File.rm!(path(ctx))
  defp damage(ctx, :corrupt), do: File.write!(path(ctx), "broken")
  defp damage(ctx, :oversized), do: File.write!(path(ctx), String.duplicate("x", 4097))
  defp damage(ctx, :partial), do: File.write!(path(ctx) <> ".next", "interrupted")
  defp damage(ctx, :unexpected), do: File.write!(Path.join(ctx.root, "unexpected"), "unknown")

  defp damage(ctx, :replacement),
    do: Files.write(ctx.root, Map.put(ctx.record, "state", "outcome_unknown"), ctx.record)

  defp damage(ctx, :root_missing), do: File.rename!(ctx.root, ctx.root <> "-moved")

  defp damage(ctx, :root_symlink) do
    damage(ctx, :root_missing)
    File.ln_s!(ctx.root <> "-moved", ctx.root)
  end

  defp damage(ctx, :symlink) do
    File.rename!(path(ctx), path(ctx) <> ".target")
    File.ln_s!(path(ctx) <> ".target", path(ctx))
  end

  defp path(ctx), do: Path.join(ctx.root, ctx.record["attempt_id"] <> ".json")

  defp snapshot(ctx) do
    for root <- [ctx.root, ctx.root <> "-moved"], into: %{} do
      files =
        case File.ls(root) do
          {:ok, names} ->
            Map.new(names, fn name ->
              path = Path.join(root, name)
              {name, {File.lstat!(path).type, File.read(path)}}
            end)

          error ->
            error
        end

      type =
        case File.lstat(root) do
          {:ok, stat} -> {stat.type, stat.mode}
          error -> error
        end

      {root, {type, files}}
    end
  end
end
