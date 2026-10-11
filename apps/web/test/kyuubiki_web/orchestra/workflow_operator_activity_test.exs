defmodule KyuubikiWeb.Orchestra.WorkflowOperatorActivityTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{
    Analysis,
    AnalysisResultStore,
    WorkflowNativeOperatorRuntime,
    WorkflowOperatorRuntime
  }

  alias KyuubikiWeb.Jobs.{Store, Watchdog}
  alias KyuubikiWeb.Playground.AgentExecutionGate

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskExecutionSummary,
    OperatorTaskReadiness,
    WorkflowNodeProgress,
    WorkflowRecoveryCoordinator,
    WorkflowRecoveryEnvelope
  }

  defmodule HeartbeatClient do
    def run_operator_task_ir(task, opts, on_progress) do
      with {:ok, output} <- request(nil, task["input_artifact"], on_progress, opts),
           {:ok, summary} <- OperatorTaskExecutionSummary.build(task) do
        {:ok,
         summary
         |> Map.take(~w(task_id task_digest operator_id program_id))
         |> Map.merge(%{
           "operator_task_ir_status" => "executed",
           "execution_readiness" => OperatorTaskReadiness.local_executed(),
           "result" => output
         })}
      end
    end

    def request(_method, payload, on_progress, opts) do
      owner = :persistent_term.get({__MODULE__, :owner})
      lease = "activity-#{opts[:job_id]}"
      endpoint = %{id: "activity-fixture", host: "127.0.0.1", port: 5093, capacity: 1}
      pause_before(:acquire, owner, opts, on_progress)
      {:ok, _, _} = AgentExecutionGate.acquire([endpoint], lease, 2_000, opts[:job_id])

      try do
        pause_before(:dispatch, owner, opts, on_progress)

        with :ok <- Keyword.get(opts, :before_dispatch, fn -> :ok end).(),
             :ok <- AgentExecutionGate.authorize_dispatch(lease) do
          send(owner, {:operator_started, self(), opts[:job_id]})
          hold(owner, payload, on_progress)
        end
      after
        AgentExecutionGate.release(lease)
      end
    end

    defp pause_before(phase, owner, opts, on_progress) do
      if :persistent_term.get({__MODULE__, :pause}, nil) == phase do
        send(owner, {:operator_boundary, self(), opts[:job_id], phase})

        receive do
          :resume ->
            # Queue/reservation activity is drained after cancellation; it is not admission.
            stage = if phase == :acquire, do: "queued", else: "preprocessing"
            on_progress.(%{"stage" => stage, "progress" => 0.01})
        after
          15_000 -> raise "fixture boundary timed out"
        end
      end
    end

    defp hold(owner, payload, on_progress) do
      receive do
        {:heartbeat, ref} ->
          result = on_progress.(%{"stage" => "solving", "progress" => 0.9, "iteration" => 99})
          send(owner, {:heartbeat, ref, result})
          hold(owner, payload, on_progress)

        :finish ->
          {:ok, payload}
      after
        15_000 -> {:error, :fixture_timeout}
      end
    end
  end

  setup do
    runtime = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])
    native = Application.get_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, [])
    watchdog = Application.get_env(:kyuubiki_web, Watchdog, [])
    Store.reset()
    AnalysisResultStore.reset()
    :persistent_term.put({HeartbeatClient, :owner}, self())

    Application.put_env(
      :kyuubiki_web,
      WorkflowOperatorRuntime,
      Keyword.put(runtime, :solve_runtime_client, HeartbeatClient)
    )

    Application.put_env(
      :kyuubiki_web,
      WorkflowNativeOperatorRuntime,
      Keyword.put(native, :client, HeartbeatClient)
    )

    Application.put_env(
      :kyuubiki_web,
      Watchdog,
      Keyword.merge(watchdog,
        scan_interval_ms: 60_000,
        stale_job_ms: 1_500,
        job_timeout_ms: 60_000
      )
    )

    # These cases control scan_now explicitly. Restart to discard a previously scheduled
    # tick, which could otherwise fail the deliberately stale job before its heartbeat.
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Watchdog)
    {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Watchdog)

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, runtime)
      Application.put_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, native)
      Application.put_env(:kyuubiki_web, Watchdog, watchdog)
      :persistent_term.erase({HeartbeatClient, :owner})
      :persistent_term.erase({HeartbeatClient, :pause})
      Store.reset()
      AnalysisResultStore.reset()
      :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Watchdog)
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Watchdog)
    end)

    :ok
  end

  for {kind, operator_id} <- [
        {"solve", "solve.bar_1d"},
        {"extract", "extract.transport_result_diagnostics"},
        {"transform", "transform.score_transport_quality"}
      ] do
    @tag workflow_operator: {kind, operator_id}
    test "operator heartbeat refreshes liveness without advancing nodes or resetting execution time for #{operator_id}",
         %{workflow_operator: {kind, operator_id}} do
      {id, pid} = start_workflow(kind, operator_id)
      {:ok, initial} = Store.get(id)
      Process.sleep(1_700)
      heartbeat(pid)
      assert %{stalled: 0, timed_out: 0} = Watchdog.scan_now()
      {:ok, alive} = Store.get(id)
      assert alive.status == :solving
      assert alive.progress == initial.progress
      assert alive.iteration == initial.iteration
      assert alive.execution_started_at == initial.execution_started_at
      assert DateTime.compare(alive.updated_at, initial.updated_at) == :gt
      assert {:ok, runtime} = AnalysisResultStore.get(id)
      assert length(runtime["progress_events"]) == 1

      # Heartbeats do not disable the independent stale-work or execution timeout guards.
      Process.sleep(1_700)
      assert %{stalled: 1} = Watchdog.scan_now()
      assert {:ok, %{status: :failed}} = Store.get(id)
      send(pid, :finish)
    end

    @tag workflow_operator: {kind, operator_id}
    test "#{operator_id} throttled activity still checks ownership and cannot touch cancelled or stale claims",
         %{workflow_operator: {kind, operator_id}} do
      {id, pid} = start_workflow(kind, operator_id)
      heartbeat(pid)
      {:ok, before} = Store.get(id)
      {:ok, runtime} = AnalysisResultStore.get(id)
      recovery = runtime[WorkflowRecoveryEnvelope.internal_key()]
      claim = Map.take(recovery, ["generation", "attempt", "owner_session_id"])
      activity = %{"event" => "operator_activity", "node_id" => "solve"}
      assert :ok = WorkflowRecoveryCoordinator.record_progress(id, claim, activity)
      assert {:ok, ^before} = Store.get(id)

      assert {:error, :stale_workflow_execution_claim} =
               WorkflowRecoveryCoordinator.record_progress(
                 id,
                 Map.put(claim, "generation", 0),
                 activity
               )

      assert {:ok, ^before} = Store.get(id)
      assert :ok = WorkflowRecoveryCoordinator.cancel(id)
      assert {:error, _} = WorkflowRecoveryCoordinator.record_progress(id, claim, activity)
      {:ok, cancelled} = Store.get(id)
      heartbeat(pid)
      assert {:ok, ^cancelled} = Store.get(id)
      assert {:ok, targets} = AgentExecutionGate.cancel_job_targets(id)
      assert [%{id: "activity-fixture"}] = targets.endpoints
      ref = Process.monitor(pid)
      send(pid, :finish)
      assert_receive {:DOWN, ^ref, :process, ^pid, _}, 2_000
      assert {:ok, ^cancelled} = Store.get(id)
      assert {:ok, retained} = AnalysisResultStore.get(id)
      assert retained[WorkflowRecoveryEnvelope.internal_key()]["state"] == "cancelled"
      refute "solve" in retained["completed_nodes"]
    end

    for phase <- [:acquire, :dispatch], skip? <- [false, true] do
      @tag workflow_operator: {kind, operator_id}, boundary_phase: phase, skip_error: skip?
      test "cancelled #{operator_id} cannot start after its #{phase} boundary with skip=#{skip?}",
           %{workflow_operator: {kind, operator_id}, boundary_phase: phase, skip_error: skip?} do
        :persistent_term.put({HeartbeatClient, :pause}, phase)
        {id, pid} = start_workflow(kind, operator_id, false, skip?)
        assert {:ok, %{"cancellation" => cancellation}} = Analysis.cancel_job(id)
        assert cancellation["target_count"] == 0
        assert cancellation["registered_count"] == 0
        assert cancellation["reserved_cancelled_count"] == if(phase == :acquire, do: 0, else: 1)

        assert cancellation["status"] ==
                 if(phase == :acquire,
                   do: "no_active_dispatch",
                   else: "cancelled_before_dispatch"
                 )

        {:ok, cancelled} = Store.get(id)
        {:ok, retained} = AnalysisResultStore.get(id)
        ref = Process.monitor(pid)
        send(pid, :resume)
        assert_receive {:DOWN, ^ref, :process, ^pid, _}, 2_000
        refute_receive {:operator_started, _, ^id}, 0
        assert {:ok, ^cancelled} = Store.get(id)
        assert {:ok, ^retained} = AnalysisResultStore.get(id)
        assert AgentExecutionGate.snapshot().active_lease_count == 0
      end
    end

    @tag workflow_operator: {kind, operator_id}
    test "cancelled throttled node completion cannot enter dependent scoring for #{operator_id}",
         %{workflow_operator: {kind, operator_id}} do
      {id, pid} = start_workflow(kind, operator_id, true)

      state =
        :sys.replace_state(WorkflowRecoveryCoordinator, fn state ->
          put_in(
            state,
            [:progress, id, :persisted_at_ms],
            System.monotonic_time(:millisecond) + 60_000
          )
        end)

      refute WorkflowNodeProgress.persist?(
               %{"node_id" => "solve", "completed_nodes" => 2, "total_nodes" => 300},
               state.progress[id]
             )

      assert :ok = WorkflowRecoveryCoordinator.cancel(id)
      assert {:ok, targets} = AgentExecutionGate.cancel_job_targets(id)
      assert [%{id: "activity-fixture"}] = targets.endpoints
      {:ok, cancelled} = Store.get(id)
      {:ok, retained} = AnalysisResultStore.get(id)
      ref = Process.monitor(pid)
      send(pid, :finish)
      assert_receive {:DOWN, ^ref, :process, ^pid, _}, 2_000
      refute_receive {:operator_started, _, ^id}
      assert {:ok, ^cancelled} = Store.get(id)
      assert {:ok, ^retained} = AnalysisResultStore.get(id)
    end

    @tag workflow_operator: {kind, operator_id}
    test "fresh operator heartbeat cannot extend the original execution deadline for #{operator_id}",
         %{workflow_operator: {kind, operator_id}} do
      {id, pid} = start_workflow(kind, operator_id)
      {:ok, original} = Store.get(id)
      Process.sleep(200)
      heartbeat(pid)
      config = Application.get_env(:kyuubiki_web, Watchdog, [])
      Application.put_env(:kyuubiki_web, Watchdog, Keyword.put(config, :job_timeout_ms, 100))
      assert %{stalled: 0, timed_out: 1} = Watchdog.scan_now()
      assert {:ok, finished} = Store.get(id)
      assert finished.status == :failed
      assert finished.execution_started_at == original.execution_started_at
      assert finished.message =~ "watchdog timed out job"
      send(pid, :finish)
    end
  end

  defp heartbeat(pid) do
    ref = make_ref()
    send(pid, {:heartbeat, ref})
    assert_receive {:heartbeat, ^ref, :ok}, 2_000
  end

  defp start_workflow(kind, operator_id, throttled? \\ false, skip? \\ false) do
    model = %{"id" => "model", "artifact_type" => "artifact/result_summary"}
    result = %{"id" => "result", "artifact_type" => "artifact/result_summary"}

    graph = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "research.operator-activity",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [model]},
        %{
          "id" => "solve",
          "kind" => kind,
          "operator_id" => operator_id,
          "inputs" => [model],
          "outputs" => [result]
        },
        %{"id" => "output", "kind" => "output", "inputs" => [result], "outputs" => []}
      ],
      "edges" => [
        %{
          "id" => "a",
          "from" => %{"node" => "input", "port" => "model"},
          "to" => %{"node" => "solve", "port" => "model"},
          "artifact_type" => "artifact/result_summary"
        },
        %{
          "id" => "b",
          "from" => %{"node" => "solve", "port" => "result"},
          "to" => %{"node" => "output", "port" => "result"},
          "artifact_type" => "artifact/result_summary"
        }
      ]
    }

    graph = if throttled?, do: with_dependent_scoring(graph, result), else: graph
    graph = if skip?, do: skip_operator_failure(graph), else: graph

    {:ok, submitted} =
      Analysis.submit_workflow_graph(%{
        "graph" => graph,
        "input_artifacts" => %{"input" => %{"value" => 41}}
      })

    id = submitted["job"]["job_id"]

    pid =
      case :persistent_term.get({HeartbeatClient, :pause}, nil) do
        nil ->
          assert_receive {:operator_started, pid, ^id}, 2_000
          pid

        phase ->
          assert_receive {:operator_boundary, pid, ^id, ^phase}, 2_000
          pid
      end

    on_exit(fn ->
      send(pid, :resume)
      send(pid, :finish)
    end)

    {id, pid}
  end

  defp skip_operator_failure(graph) do
    Map.update!(graph, "nodes", fn nodes ->
      Enum.map(nodes, fn node ->
        if node["id"] == "solve", do: Map.put(node, "on_error", "skip"), else: node
      end)
    end)
  end

  defp with_dependent_scoring(graph, result) do
    quality = %{
      "id" => "quality",
      "kind" => "transform",
      "operator_id" => "transform.score_transport_quality",
      "inputs" => [result],
      "outputs" => [result]
    }

    outputs =
      for n <- 1..296,
          do: %{"id" => "output-#{n}", "kind" => "output", "inputs" => [result], "outputs" => []}

    edges =
      for node <- [%{"id" => "output"} | outputs] do
        %{
          "id" => "quality-#{node["id"]}",
          "from" => %{"node" => "quality", "port" => "result"},
          "to" => %{"node" => node["id"], "port" => "result"},
          "artifact_type" => "artifact/result_summary"
        }
      end

    graph
    |> Map.put("nodes", graph["nodes"] ++ [quality | outputs])
    |> Map.put("edges", [
      hd(graph["edges"]),
      put_in(Enum.at(graph["edges"], 1), ["to", "node"], "quality") | edges
    ])
    |> Map.put("output_nodes", ["output" | Enum.map(outputs, & &1["id"])])
  end
end
