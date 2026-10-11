defmodule KyuubikiWeb.Orchestra.WorkflowNativeResultRecoveryTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorDispatchFiles, as: Files
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskExecutionSummary,
    OperatorTaskIR,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.WorkflowNativeOperatorRuntime, as: Native

  @policy Path.expand("../../../../../schemas/agent-task-result-retention.schema.json", __DIR__)
          |> File.read!()
          |> Jason.decode!()
          |> get_in(["$defs", "policy", "properties"])
          |> Map.new(fn {key, spec} -> {key, spec["const"]} end)
  @operators [
    {"extract", "extract.transport_result_diagnostics"},
    {"extract", "extract.thermal_result_diagnostics"},
    {"extract", "extract.electrostatic_result_diagnostics"},
    {"extract", "extract.magnetostatic_result_diagnostics"},
    {"extract", "extract.thermo_result_diagnostics"},
    {"extract", "extract.stokes_flow_result_diagnostics"},
    {"transform", "transform.score_transport_quality"},
    {"transform", "transform.score_thermal_quality"},
    {"transform", "transform.score_electrostatic_quality"},
    {"transform", "transform.score_magnetostatic_quality"},
    {"transform", "transform.score_cfd_quality"}
  ]

  defmodule Client do
    def run_operator_task_ir(task, opts, _activity) do
      assert_guard = Keyword.get(opts, :before_dispatch, fn -> :ok end)

      with :ok <- assert_guard.() do
        send(self(), {:executed, task})
        {:ok, summary} = OperatorTaskExecutionSummary.build(task)
        {:ok, receipt(summary, %{"computed" => true})}
      end
    end

    def receipt(summary, output) do
      Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
        "operator_task_ir_status" => "executed",
        "execution_readiness" => OperatorTaskReadiness.local_executed(),
        "result" => output
      })
    end
  end

  setup do
    root = Path.join(System.tmp_dir!(), "native-restore-#{System.unique_integer([:positive])}")
    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    endpoint = %{id: "owner", host: "127.0.0.1", port: 1, agent_session_id: "owner-session"}
    %{root: root, journal: journal, endpoint: endpoint}
  end

  for {kind, id} <- @operators do
    test "restores #{id} without dispatching or mutating the journal", ctx do
      spec = spec(unquote(kind), unquote(id))
      record = seed(ctx, spec)
      path = Path.join(ctx.root, record["attempt_id"] <> ".json")
      before = File.read!(path)

      gate = fn ->
        send(self(), :checked_claim)
        :ok
      end

      assert {:ok, %{"retained" => 42}} = run(ctx, spec, before_dispatch: gate)
      assert_receive :checked_claim
      assert_receive :checked_claim
      assert_receive {:probe, endpoint, query}
      assert endpoint == ctx.endpoint
      assert query["attempt_id"] == record["attempt_id"]
      refute_receive {:executed, _}, 0
      refute_receive :checked_claim, 0
      assert File.read!(path) == before
      assert {:ok, [^record]} = Journal.lookup_task(record["task_id"], ctx.journal)
    end
  end

  test "first execution does not adopt an old receipt or require a recovery journal", ctx do
    spec = spec()
    seed(ctx, spec)

    assert {:ok, %{"computed" => true}} =
             run(ctx, spec, recover_native_results: false, journal: :missing)

    assert_receive {:executed, _}
    refute_receive {:probe, _, _}, 0
  end

  test "unexecuted nodes may dispatch after the guard when no successful record exists", ctx do
    spec = spec()

    for state <- [nil, "not_dispatched", "observed_failed", "observed_blocked"] do
      if state, do: seed(ctx, spec, state)
      assert {:ok, %{"computed" => true}} = run(ctx, spec)
      assert_receive {:executed, _}
      refute_receive {:probe, _, _}, 0
    end
  end

  for state <- ~w(dispatch_boundary_unconfirmed outcome_unknown) do
    test "#{state} cannot be promoted by a recoverable success receipt", ctx do
      spec = spec()
      seed(ctx, spec, unquote(state))
      assert {:error, {:native_workflow_recovery_blocked, :dispatch_unconfirmed}} = run(ctx, spec)
      refute_receive {:probe, _, _}, 0
      refute_receive {:executed, _}, 0
    end
  end

  test "a newer success never erases an older unknown dispatch", ctx do
    spec = spec()
    seed(ctx, spec, "outcome_unknown")
    seed(ctx, spec)
    assert {:error, {:native_workflow_recovery_blocked, :dispatch_unconfirmed}} = run(ctx, spec)
    refute_receive {:probe, _, _}, 0
    refute_receive {:executed, _}, 0
  end

  test "an unbound ambiguous exact task may deny restoration but cannot authorize it", ctx do
    spec = spec()
    seed(ctx, spec, "outcome_unknown", nil)
    seed(ctx, spec)
    assert {:error, {:native_workflow_recovery_blocked, :dispatch_unconfirmed}} = run(ctx, spec)
    refute_receive {:probe, _, _}, 0
    refute_receive {:executed, _}, 0
  end

  for field <- ~w(operator_id program_id) do
    test "a contradictory dispatch #{field} cannot authorize restoration", ctx do
      spec = spec()
      record = seed(ctx, spec)
      stop_supervised(Journal)
      path = Path.join(ctx.root, record["attempt_id"] <> ".json")
      File.rm!(path)
      assert :ok = Files.write(ctx.root, Map.put(record, unquote(field), "foreign"))
      ctx = %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}

      assert {:error, {:native_workflow_recovery_blocked, :dispatch_identity_invalid}} =
               run(ctx, spec)

      refute_receive {:probe, _, _}, 0
      refute_receive {:executed, _}, 0
    end
  end

  test "missing selected receipt never falls back to another historical success", ctx do
    spec = spec()
    seed(ctx, spec)
    seed(ctx, spec)
    {:ok, [selected | _]} = Journal.lookup_task(task(spec)["task_id"], ctx.journal)

    probe = fn _, query ->
      send(self(), {:selected_read, query["attempt_id"]})
      assert query["attempt_id"] == selected["attempt_id"]
      {:ok, retained(query) |> Map.put("status", "not_retained") |> Map.put("response", nil)}
    end

    assert {:error, {:native_workflow_recovery_blocked, :receipt_unavailable}} =
             run(ctx, spec, probe: probe)

    assert_receive {:selected_read, _}
    refute_receive {:selected_read, _}, 0
    refute_receive {:executed, _}, 0
  end

  for field <- [:payload, :config, :node, :context] do
    test "changed #{field} cannot reuse or redispatch an old successful node", ctx do
      spec = spec()
      seed(ctx, spec)
      changed = Map.update!(spec, unquote(field), &Map.put(&1, "changed", true))
      # Placement is part of TaskIR even though arbitrary extra node fields are not.
      changed =
        if unquote(field) == :node,
          do: put_in(changed, [:node, "placement_tags"], ["changed"]),
          else: changed

      assert {:error, {:native_workflow_recovery_blocked, :input_changed}} = run(ctx, changed)
      refute_receive {:probe, _, _}, 0
      refute_receive {:executed, _}, 0
    end
  end

  test "foreign parents cannot supply recovery results even with the exact child TaskIR", ctx do
    spec = spec()
    seed(ctx, spec, "observed_executed", "other-parent")
    assert {:ok, %{"computed" => true}} = run(ctx, spec)
    assert_receive {:executed, _}
    refute_receive {:probe, _, _}, 0
  end

  for legacy? <- [false, true] do
    test "unbound success legacy=#{legacy?} cannot authorize node restoration", ctx do
      spec = spec()
      record = seed(ctx, spec, "observed_executed", nil)

      ctx =
        if unquote(legacy?) do
          stop_supervised(Journal)
          File.rm!(Path.join(ctx.root, record["attempt_id"] <> ".json"))

          legacy =
            record
            |> Map.delete("job_id")
            |> Map.put("schema_version", "kyuubiki.operator-task-dispatch-record/v1")

          assert :ok = Files.write(ctx.root, legacy)
          %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}
        else
          ctx
        end

      assert {:ok, %{"computed" => true}} = run(ctx, spec)
      assert_receive {:executed, _}
      refute_receive {:probe, _, _}, 0
    end
  end

  for status <- ~w(pending not_retained result_not_retained attempt_identity_ambiguous) do
    test "#{status} does not turn a missing receipt into replay permission", ctx do
      spec = spec()
      seed(ctx, spec)

      probe = fn _, query ->
        {:ok, retained(query) |> Map.put("status", unquote(status)) |> Map.put("response", nil)}
      end

      assert {:error, {:native_workflow_recovery_blocked, :receipt_unavailable}} =
               run(ctx, spec, probe: probe)

      refute_receive {:executed, _}, 0
    end
  end

  test "an unavailable or changed original endpoint cannot fall back to a peer or execution",
       ctx do
    spec = spec()
    seed(ctx, spec)

    for endpoints <- [[], [%{ctx.endpoint | agent_session_id: "replacement"}]] do
      assert {:error, {:native_workflow_recovery_blocked, :receipt_unavailable}} =
               run(ctx, spec, endpoints: endpoints)

      refute_receive {:probe, _, _}, 0
      refute_receive {:executed, _}, 0
    end

    assert {:error, {:native_workflow_recovery_blocked, :receipt_unavailable}} =
             run(ctx, spec, probe: fn _, _ -> {:error, :timeout} end)

    refute_receive {:executed, _}, 0
  end

  test "malformed receipt identity and non-executed outcomes cannot publish or replay", ctx do
    spec = spec()
    seed(ctx, spec)

    for path <- [~w(request_id), ~w(task_digest), ~w(response id), ~w(response result task_id)] do
      assert {:error, {:native_workflow_recovery_blocked, :receipt_invalid}} =
               run(ctx, spec,
                 probe: fn _, query -> {:ok, put_in(retained(query), path, "foreign")} end
               )
    end

    probe = fn _, query ->
      receipt = retained(query)

      receipt =
        put_in(receipt, ~w(response result operator_task_ir_status), "verified_pending_execution")

      {:ok,
       put_in(receipt, ~w(response result execution_readiness), %{
         "status" => "blocked",
         "ready_to_dispatch" => false
       })}
    end

    assert {:error, {:native_workflow_recovery_blocked, :receipt_unavailable}} =
             run(ctx, spec, probe: probe)

    refute_receive {:executed, _}, 0
  end

  test "journal loss and corrupt generations block restoration without a probe", ctx do
    spec = spec()

    assert {:error, {:native_workflow_recovery_blocked, :journal_unavailable}} =
             run(ctx, spec, journal: :missing)

    stop_supervised(Journal)
    File.mkdir_p!(ctx.root)
    File.write!(Path.join(ctx.root, "corrupt.json"), "damaged")
    ctx = %{ctx | journal: start_supervised!({Journal, root: ctx.root, name: nil})}
    assert {:error, {:native_workflow_recovery_blocked, :journal_unavailable}} = run(ctx, spec)
    refute_receive {:probe, _, _}, 0
    refute_receive {:executed, _}, 0
  end

  test "claim checks fence both before retrieval and after a valid receipt arrives", ctx do
    spec = spec()
    seed(ctx, spec)

    assert {:error, :stale_claim} =
             run(ctx, spec, before_dispatch: fn -> {:error, :stale_claim} end)

    refute_receive {:probe, _, _}, 0
    key = make_ref()
    Process.put(key, 0)

    guard = fn ->
      count = Process.get(key) + 1
      Process.put(key, count)
      if count == 1, do: :ok, else: {:error, :cancelled_after_read}
    end

    assert {:error, :cancelled_after_read} = run(ctx, spec, before_dispatch: guard)
    assert_receive {:probe, _, _}
    refute_receive {:executed, _}, 0
  end

  test "invalid recovery flags and absent parent identity cannot dispatch", ctx do
    for flag <- [nil, 1, "true", []] do
      assert {:error, :invalid_native_workflow_operator_request} =
               run(ctx, spec(), recover_native_results: flag)
    end

    assert {:error, {:native_workflow_recovery_blocked, :dispatch_identity_invalid}} =
             run(ctx, %{spec() | context: %{}})

    refute_receive {:executed, _}, 0
  end

  defp spec(kind \\ "extract", id \\ "extract.thermal_result_diagnostics") do
    %{
      kind: kind,
      id: id,
      payload: %{"nodes" => []},
      config: %{},
      node: %{
        "id" => "native",
        "kind" => kind,
        "operator_id" => id,
        "placement_tags" => ["research"],
        "required_capabilities" => ["task_ir"]
      },
      context: %{"job_id" => "parent-job"}
    }
  end

  defp task(spec) do
    {:ok, task} =
      OperatorTaskIR.build(spec.id, spec.payload, spec.config,
        node: spec.node,
        orchestration_context: spec.context,
        task_id: Native.task_id(spec.context, spec.node, spec.id),
        placement_tags: spec.node["placement_tags"],
        required_capabilities: spec.node["required_capabilities"]
      )

    task
  end

  defp seed(ctx, spec, state \\ "observed_executed", parent \\ "parent-job") do
    task = task(spec)

    request = %{
      "id" => "rpc-#{System.unique_integer([:positive])}",
      "params" => %{"task_ir" => task, "job_id" => parent}
    }

    {:ok, attempt} = Journal.begin_dispatch(request, ctx.endpoint, ctx.journal)
    assert :ok = Journal.finish_dispatch(attempt, state, ctx.journal)
    {:ok, records} = Journal.lookup_task(task["task_id"], ctx.journal)
    Enum.find(records, &(&1["attempt_id"] == attempt))
  end

  defp run(ctx, spec, overrides \\ []) do
    probe = fn endpoint, query ->
      send(self(), {:probe, endpoint, query})
      {:ok, retained(query)}
    end

    opts =
      Keyword.merge(
        [
          client: Client,
          recover_native_results: true,
          journal: ctx.journal,
          endpoints: [ctx.endpoint],
          probe: probe
        ],
        overrides
      )

    Native.run(spec.kind, spec.id, spec.payload, spec.config, spec.node, spec.context, opts)
  end

  defp retained(identity) do
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
        "result" => Client.receipt(identity, %{"retained" => 42})
      }
    })
  end
end
