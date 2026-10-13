defmodule KyuubikiWeb.Orchestra.WorkflowNativeAttemptRecoveryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Orchestra.OperatorTaskReadiness
  alias KyuubikiWeb.Orchestra.WorkflowNativeResultRecovery, as: Recovery

  @policy Path.expand("../../../../../schemas/agent-task-result-retention.schema.json", __DIR__)
          |> File.read!()
          |> Jason.decode!()
          |> get_in(["$defs", "policy", "properties"])
          |> Map.new(fn {key, spec} -> {key, spec["const"]} end)

  setup do
    root = Path.join(System.tmp_dir!(), "native-attempt-#{System.unique_integer([:positive])}")
    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    {:ok, task} = OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{})
    {:ok, summary} = OperatorTaskExecutionSummary.build(task)

    %{
      root: root,
      journal: journal,
      task: task,
      summary: summary,
      endpoint: %{id: "owner", host: "127.0.0.1", port: 1, agent_session_id: "session"}
    }
  end

  for states <- [
        ~w(observed_executed observed_executed),
        ~w(observed_executed observed_failed),
        ~w(observed_failed observed_executed),
        ~w(observed_executed observed_blocked),
        ~w(observed_blocked observed_executed)
      ] do
    test "ambiguous #{Enum.join(states, "/")} attempts cannot select a convenient success", ctx do
      for state <- unquote(states), do: seed(ctx, state)
      {:ok, before} = Journal.lookup_task(ctx.task["task_id"], ctx.journal)

      assert {:error, {:native_workflow_recovery_blocked, :dispatch_attempt_ambiguous}} = run(ctx)
      refute_receive {:probe, _}, 0
      assert {:ok, ^before} = Journal.lookup_task(ctx.task["task_id"], ctx.journal)
    end
  end

  test "ambiguity remains blocked after journal restart without rewriting old attempts", ctx do
    seed(ctx, "observed_executed")
    seed(ctx, "observed_failed")
    before = files(ctx)
    assert :ok = stop_supervised(Journal)
    ctx = %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}
    assert {:error, {:native_workflow_recovery_blocked, :dispatch_attempt_ambiguous}} = run(ctx)
    refute_receive {:probe, _}, 0
    assert files(ctx) == before
  end

  test "unbound and foreign successful attempts cannot resolve an owned ambiguity", ctx do
    seed(ctx, "observed_executed")
    seed(ctx, "observed_failed")
    seed(ctx, "observed_executed", "foreign-parent")
    seed(ctx, "observed_executed", nil)
    assert {:error, {:native_workflow_recovery_blocked, :dispatch_attempt_ambiguous}} = run(ctx)
    refute_receive {:probe, _}, 0
  end

  test "a single success with non-dispatched or foreign peers remains recoverable", ctx do
    selected = seed(ctx, "observed_executed")
    seed(ctx, "not_dispatched")
    seed(ctx, "observed_executed", "foreign-parent")
    seed(ctx, "observed_failed", "foreign-parent")
    seed(ctx, "observed_executed", nil)
    before = files(ctx)
    assert {:ok, %{"retained" => 42}} = run(ctx)
    assert_receive {:probe, %{"attempt_id" => ^selected}}
    refute_receive {:probe, _}, 0
    assert files(ctx) == before
  end

  for state <- ~w(observed_failed observed_blocked not_dispatched) do
    test "selected success changing to #{state} during retrieval cannot publish", ctx do
      attempt = seed(ctx, "observed_executed")
      {:ok, [original]} = Journal.lookup_task(ctx.task["task_id"], ctx.journal)

      probe = fn query ->
        assert query["attempt_id"] == attempt
        # Inject a consistent replacement outside the API; selection must still change.
        :ok = Files.write(ctx.root, Map.put(original, "state", unquote(state)), original)

        :sys.replace_state(ctx.journal, fn journal ->
          put_in(journal, [:records, attempt, "state"], unquote(state))
        end)

        {:ok, retained(query)}
      end

      assert {:error, {:native_workflow_recovery_blocked, :dispatch_observation_changed}} =
               run(ctx, probe)

      assert_receive {:probe, %{"attempt_id" => ^attempt}}
      refute_receive {:probe, _}, 0
    end
  end

  test "a rejected late completion cannot poison a valid unique recovery", ctx do
    attempt = seed(ctx, "observed_executed")
    before = files(ctx)

    probe = fn query ->
      assert {:error, :operator_task_dispatch_transition_rejected} =
               Journal.finish_dispatch(attempt, "observed_failed", ctx.journal)

      assert :ok = Journal.finish_dispatch(attempt, "observed_executed", ctx.journal)
      {:ok, retained(query)}
    end

    assert {:ok, %{"retained" => 42}} = run(ctx, probe)
    assert_receive {:probe, %{"attempt_id" => ^attempt}}
    refute_receive {:probe, _}, 0
    assert files(ctx) == before
  end

  for fault <- [:deleted, :replacement, :partial], phase <- [:before_probe, :during_probe] do
    test "#{fault} disk evidence #{phase} blocks cached successful native reuse", ctx do
      attempt = seed(ctx, "observed_executed")
      {:ok, [record]} = Journal.lookup_task(ctx.task["task_id"], ctx.journal)
      path = Path.join(ctx.root, attempt <> ".json")

      damage = fn ->
        case unquote(fault) do
          :deleted ->
            File.rm!(path)

          :replacement ->
            assert :ok =
                     Files.write(ctx.root, Map.put(record, "state", "outcome_unknown"), record)

          :partial ->
            File.write!(path <> ".next", "interrupted")
        end
      end

      if unquote(phase) == :before_probe, do: damage.()

      probe = fn query ->
        if unquote(phase) == :during_probe, do: damage.()
        {:ok, retained(query)}
      end

      assert {:error, {:native_workflow_recovery_blocked, :journal_unavailable}} = run(ctx, probe)
      if unquote(phase) == :during_probe, do: assert_receive({:probe, _})
      refute_receive {:probe, _}, 0
      assert {:error, _} = Journal.lookup_task(ctx.task["task_id"], ctx.journal)

      case unquote(fault) do
        :deleted ->
          refute File.exists?(path)

        :replacement ->
          assert {:ok, stored} = Files.load(ctx.root)
          assert stored[attempt]["state"] == "outcome_unknown"

        :partial ->
          assert File.read!(path <> ".next") == "interrupted"
      end
    end
  end

  for {state, reason} <- [
        {"observed_executed", :dispatch_attempt_ambiguous},
        {"observed_failed", :dispatch_attempt_ambiguous},
        {"dispatch_boundary_unconfirmed", :dispatch_unconfirmed},
        {"outcome_unknown", :dispatch_unconfirmed}
      ] do
    test "new #{state} attempt during retrieval blocks stale publication", ctx do
      original = seed(ctx, "observed_executed")

      probe = fn query ->
        assert query["attempt_id"] == original
        seed(ctx, unquote(state))
        {:ok, retained(query)}
      end

      assert {:error, {:native_workflow_recovery_blocked, unquote(reason)}} = run(ctx, probe)
      assert_receive {:probe, %{"attempt_id" => ^original}}
      refute_receive {:probe, _}, 0
    end
  end

  test "journal loss after a valid receipt arrives cannot publish or authorize fresh execution",
       ctx do
    seed(ctx, "observed_executed")

    probe = fn query ->
      GenServer.stop(ctx.journal, :normal)
      {:ok, retained(query)}
    end

    assert {:error, {:native_workflow_recovery_blocked, :journal_unavailable}} = run(ctx, probe)
    assert_receive {:probe, _}
    refute_receive {:probe, _}, 0
  end

  test "selected attempt eviction during receipt retrieval cannot authorize stale reuse", ctx do
    original = seed(ctx, "observed_executed")
    {:ok, [record]} = Journal.lookup_task(ctx.task["task_id"], ctx.journal)
    assert :ok = stop_supervised(Journal)
    old = Map.merge(record, %{"created_at_ms" => 0, "updated_at_ms" => 0})
    assert :ok = Files.write(ctx.root, old, record)
    ctx = %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}

    probe = fn query ->
      for index <- 1..Journal.policy().terminal_retention do
        other = %{ctx | task: Map.put(ctx.task, "task_id", "unrelated-#{index}")}
        seed(other, "observed_failed", "other-parent")
      end

      assert {:ok, []} = Journal.lookup_task(ctx.task["task_id"], ctx.journal)
      {:ok, retained(query)}
    end

    assert {:error, {:native_workflow_recovery_blocked, :dispatch_observation_changed}} =
             run(ctx, probe)

    assert_receive {:probe, %{"attempt_id" => ^original}}
    refute_receive {:probe, _}, 0
    refute File.exists?(Path.join(ctx.root, original <> ".json"))
  end

  test "an unrelated parent attempt arriving during retrieval cannot poison a valid recovery",
       ctx do
    original = seed(ctx, "observed_executed")

    probe = fn query ->
      seed(ctx, "outcome_unknown", "foreign-parent")
      {:ok, retained(query)}
    end

    assert {:ok, %{"retained" => 42}} = run(ctx, probe)
    assert_receive {:probe, %{"attempt_id" => ^original}}
    refute_receive {:probe, _}, 0
  end

  defp seed(ctx, state, parent \\ "parent") do
    request = %{
      "id" => "request-#{System.unique_integer([:positive])}",
      "params" => %{"task_ir" => ctx.task, "job_id" => parent}
    }

    {:ok, attempt} = Journal.begin_dispatch(request, ctx.endpoint, ctx.journal)
    assert :ok = Journal.finish_dispatch(attempt, state, ctx.journal)
    attempt
  end

  defp run(ctx, probe \\ &default_probe/1) do
    Recovery.resolve(ctx.task, ctx.summary, %{"job_id" => "parent"},
      recover_native_results: true,
      journal: ctx.journal,
      endpoints: [ctx.endpoint],
      probe: fn _, query ->
        send(self(), {:probe, query})
        probe.(query)
      end
    )
  end

  defp default_probe(query), do: {:ok, retained(query)}

  defp files(ctx) do
    for path <- Path.wildcard(Path.join(ctx.root, "*.json")),
        into: %{},
        do: {Path.basename(path), File.read!(path)}
  end

  defp retained(identity) do
    result =
      Map.merge(Map.take(identity, ~w(task_id task_digest operator_id program_id)), %{
        "operator_task_ir_status" => "executed",
        "execution_readiness" => OperatorTaskReadiness.local_executed(),
        "result" => %{"retained" => 42}
      })

    Map.merge(identity, %{
      "schema_version" => "kyuubiki.agent-task-result-retention/v1",
      "process_instance_id" => "original-boot",
      "generation" => 1,
      "status" => "receipt_retained",
      "automatic_replay_authorized" => false,
      "retention_policy" => @policy,
      "response" => %{
        "rpc_version" => 1,
        "id" => identity["request_id"],
        "ok" => true,
        "result" => result
      }
    })
  end
end
