defmodule KyuubikiWeb.Orchestra.WorkflowNativeDispatchPipelineTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{Analysis, AnalysisResultStore, WorkflowNativeOperatorRuntime}
  alias KyuubikiWeb.Jobs.Watchdog
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowNativeDispatchEvidence, as: Evidence
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.{OperatorTaskExecutionSummary, OperatorTaskReadiness}
  alias KyuubikiWeb.Playground.{AgentClient, AgentPool}

  alias KyuubikiWeb.TestSupport.{
    CancellableSolverAgent,
    ControlledSolverAgent,
    WorkflowCommitFixture
  }

  setup do
    WorkflowCommitFixture.setup_coordinator()
    root = Path.join(System.tmp_dir!(), "native-pipeline-#{System.unique_integer([:positive])}")
    :ok = Supervisor.terminate_child(KyuubikiWeb.Supervisor, Journal)
    start_supervised!({Journal, root: root})
    previous = Application.get_env(:kyuubiki_web, AgentPool, [])
    native = Application.get_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, [])
    Application.put_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, client: AgentClient)

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, previous)
      Application.put_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, native)
      AgentPool.reload()
      {:ok, _} = Supervisor.restart_child(KyuubikiWeb.Supervisor, Journal)
      File.rm_rf!(root)
    end)

    start_supervised!({ControlledSolverAgent, {self(), %{"stage" => "solving"}}})
    assert_receive {:fake_agent_ready, port}

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "native-evidence-agent",
          host: "127.0.0.1",
          port: port,
          methods: ["run_operator_task_ir"],
          capabilities: ["solver_rpc"],
          operator_package_runtime: %{"ready" => false}
        }
      ]
    )

    AgentPool.reload()
    %{root: root}
  end

  for outcome <- [:complete, :journal_lost] do
    test "real coordinator to TCP dispatch persists evidence before Agent work: #{outcome}",
         ctx do
      assert {:ok, admitted} =
               Analysis.submit_workflow_graph(%{
                 "graph" => graph(),
                 "input_artifacts" => %{"input" => %{}}
               })

      job = admitted["job"]["job_id"]
      assert_receive {:solver_request, agent, request}, 2_000
      assert request["method"] == "run_operator_task_ir"
      assert request["params"]["job_id"] == job
      attempt = request["params"]["dispatch_attempt_id"]
      {:ok, runtime} = AnalysisResultStore.get(job)
      recovery = runtime[Envelope.internal_key()]
      entry = recovery[Evidence.key()]["entries"][attempt]
      assert is_map(entry)
      assert entry["producer_claim"]["generation"] == recovery["generation"]
      {:ok, summary} = OperatorTaskExecutionSummary.build(request["params"]["task_ir"])

      assert Map.take(entry, ~w(task_id task_digest operator_id program_id)) ==
               Map.take(summary, ~w(task_id task_digest operator_id program_id))

      assert :ok = Evidence.verify(recovery, job)

      if unquote(outcome) == :journal_lost do
        assert :ok = stop_supervised(Journal)
        File.rm_rf!(ctx.root)
        start_supervised!({Journal, root: ctx.root})
      end

      receipt =
        Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
          "operator_task_ir_status" => "executed",
          "execution_readiness" => OperatorTaskReadiness.local_executed(),
          "result" => %{"controlled" => 42}
        })

      send(agent, {:reply, %{"ok" => true, "result" => receipt}})

      status = if unquote(outcome) == :complete, do: "completed", else: "failed"
      result = terminal(job, status)
      {:ok, final} = AnalysisResultStore.get(job)

      if unquote(outcome) == :complete do
        assert result["recovery"]["state"] == "completed"
        assert result["artifacts"]["diagnose.payload"] == %{"controlled" => 42}
        refute Map.has_key?(final[Envelope.internal_key()], Evidence.key())
      else
        assert result["recovery"]["state"] == "recovery_blocked"
        assert result["recovery"]["generation"] == 1
        refute Map.has_key?(result["artifacts"], "diagnose.payload")
        assert final[Envelope.internal_key()][Evidence.key()] == recovery[Evidence.key()]
        refute File.exists?(ctx.root)
      end

      refute_receive {:solver_request, _, _}, 20
      assert [%{consecutive_failures: 0}] = AgentPool.endpoints()
    end
  end

  for outcome <- [:disconnect, :invalid_receipt, :rpc_error], skip? <- [false, true] do
    @tag skip_node_error: skip?
    test "unconfirmed #{outcome} blocks the parent and peer replay with skip=#{skip?}", ctx do
      start_supervised!(
        Supervisor.child_spec({ControlledSolverAgent, {self(), %{"stage" => "peer"}}},
          id: :unconfirmed_peer
        )
      )

      assert_receive {:fake_agent_ready, port}
      config = Application.get_env(:kyuubiki_web, AgentPool)
      [first] = config[:endpoints]
      peer = %{first | id: "unused-native-peer", port: port}

      Application.put_env(
        :kyuubiki_web,
        AgentPool,
        Keyword.put(config, :endpoints, [first, peer])
      )

      AgentPool.reload()

      graph =
        update_in(graph(), ["nodes"], fn nodes ->
          Enum.map(nodes, fn node ->
            if node["id"] == "diagnose" and not ctx.skip_node_error,
              do: Map.put(node, "config", %{}),
              else: node
          end)
        end)

      assert {:ok, admitted} =
               Analysis.submit_workflow_graph(%{
                 "graph" => graph,
                 "input_artifacts" => %{"input" => %{}}
               })

      job = admitted["job"]["job_id"]
      assert_receive {:solver_request, agent, request}, 2_000
      task_id = request["params"]["task_ir"]["task_id"]
      {:ok, before} = AnalysisResultStore.get(job)
      evidence = before[Envelope.internal_key()][Evidence.key()]

      reply =
        case unquote(outcome) do
          :disconnect ->
            :disconnect

          :invalid_receipt ->
            {:reply, %{"ok" => true, "result" => %{}}}

          :rpc_error ->
            {:reply,
             %{"ok" => false, "error" => %{"code" => "failed", "message" => "no receipt"}}}
        end

      send(agent, reply)
      refute_receive {:solver_request, _, _}, 50
      result = terminal(job, ["completed", "failed"])
      assert result["recovery"]["state"] == "recovery_blocked"
      refute Map.has_key?(result["artifacts"], "diagnose.payload")
      {:ok, final} = AnalysisResultStore.get(job)
      assert final[Envelope.internal_key()][Evidence.key()] == evidence
      assert {:ok, [%{"state" => "outcome_unknown"}]} = Journal.lookup_task(task_id)
      refute_receive {:solver_request, _, _}, 50

      [first, peer] = AgentPool.endpoints()
      assert peer.consecutive_failures == 0
      assert first.consecutive_failures == if(unquote(outcome) == :rpc_error, do: 0, else: 1)

      assert :ok = stop_supervised(Journal)
      start_supervised!({Journal, root: ctx.root})
      WorkflowCommitFixture.restart_coordinator()
      assert Coordinator.recover_now()["recovered"] == 0
      {:ok, restarted} = AnalysisResultStore.get(job)
      assert restarted[Envelope.internal_key()]["state"] == "recovery_blocked"
      assert restarted[Envelope.internal_key()]["generation"] == 1
      assert restarted[Envelope.internal_key()][Evidence.key()] == evidence
      refute_receive {:solver_request, _, _}, 20
    end
  end

  test "workflow deadline retains an in-flight native attempt after killing its transport", ctx do
    watchdog = Application.get_env(:kyuubiki_web, Watchdog, [])

    Application.put_env(
      :kyuubiki_web,
      Watchdog,
      Keyword.put(watchdog, :workflow_timeout_ms, 1_000)
    )

    on_exit(fn -> Application.put_env(:kyuubiki_web, Watchdog, watchdog) end)

    start_supervised!({CancellableSolverAgent, {self(), "deadline-agent"}})
    assert_receive {:cancellable_agent_ready, "deadline-agent", port}
    config = Application.get_env(:kyuubiki_web, AgentPool)
    [endpoint] = config[:endpoints]
    endpoint = %{endpoint | id: "deadline-agent", port: port}
    Application.put_env(:kyuubiki_web, AgentPool, Keyword.put(config, :endpoints, [endpoint]))
    AgentPool.reload()

    assert {:ok, admitted} =
             Analysis.submit_workflow_graph(%{
               "graph" => graph(),
               "input_artifacts" => %{"input" => %{}}
             })

    job = admitted["job"]["job_id"]
    assert_receive {:agent_request, "deadline-agent", agent, request}, 2_000
    task_id = request["params"]["task_ir"]["task_id"]
    {:ok, before} = AnalysisResultStore.get(job)
    result = terminal(job, "failed")
    assert result["recovery"]["state"] == "recovery_blocked"
    {:ok, final} = AnalysisResultStore.get(job)

    assert final[Envelope.internal_key()][Evidence.key()] ==
             before[Envelope.internal_key()][Evidence.key()]

    assert final[Envelope.internal_key()]["envelope"] ==
             before[Envelope.internal_key()]["envelope"]

    refute Map.has_key?(result["artifacts"], "diagnose.payload")
    assert {:ok, [record]} = Journal.lookup_task(task_id)
    assert record["state"] == "dispatch_boundary_unconfirmed"
    assert_receive {:agent_request, "deadline-agent", _, cancellation}, 2_000
    assert cancellation["method"] == "cancel_job"
    assert cancellation["params"]["job_id"] == job
    send(agent, {:reply, %{"ok" => true, "result" => %{}}})

    assert :ok = stop_supervised(Journal)
    start_supervised!({Journal, root: ctx.root})
    WorkflowCommitFixture.restart_coordinator()
    assert Coordinator.recover_now()["recovered"] == 0
    {:ok, restarted} = AnalysisResultStore.get(job)
    assert restarted[Envelope.internal_key()] == final[Envelope.internal_key()]
    refute_receive {:solver_request, _, _}, 30
    refute_receive {:agent_request, _, _, _}, 30
    assert [%{consecutive_failures: 0}] = AgentPool.endpoints()
  end

  for reason <- [:stall, :execution_timeout] do
    test "periodic watchdog #{reason} retains evidence and cancels only the active native task",
         ctx do
      watchdog = Application.get_env(:kyuubiki_web, Watchdog, [])
      on_exit(fn -> Application.put_env(:kyuubiki_web, Watchdog, watchdog) end)
      start_supervised!({CancellableSolverAgent, {self(), "watchdog-agent"}})
      assert_receive {:cancellable_agent_ready, "watchdog-agent", port}
      config = Application.get_env(:kyuubiki_web, AgentPool)
      [endpoint] = config[:endpoints]

      endpoint =
        endpoint
        |> Map.merge(%{id: "watchdog-agent", port: port, capacity: 2})
        |> Map.put(:methods, ["run_operator_task_ir", "solve_bar_1d"])

      Application.put_env(:kyuubiki_web, AgentPool, Keyword.put(config, :endpoints, [endpoint]))
      AgentPool.reload()

      assert {:ok, admitted} =
               Analysis.submit_workflow_graph(%{
                 "graph" => graph(),
                 "input_artifacts" => %{"input" => %{}}
               })

      job = admitted["job"]["job_id"]
      assert_receive {:agent_request, "watchdog-agent", agent, request}, 2_000

      bystander =
        Task.async(fn ->
          AgentClient.request("solve_bar_1d", %{}, fn _ -> :ok end, job_id: "watchdog-bystander")
        end)

      assert_receive {:agent_request, "watchdog-agent", other_agent, other_request}, 2_000
      assert other_request["method"] == "solve_bar_1d"
      task_id = request["params"]["task_ir"]["task_id"]
      {:ok, before} = AnalysisResultStore.get(job)
      key = if unquote(reason) == :stall, do: :stale_job_ms, else: :job_timeout_ms
      Application.put_env(:kyuubiki_web, Watchdog, Keyword.put(watchdog, key, 1))
      Process.sleep(5)
      assert %{available: true} = stats = Watchdog.scan_now()
      assert stats.stalled + stats.timed_out == 1
      {:ok, final} = AnalysisResultStore.get(job)
      assert final[Envelope.internal_key()]["state"] == "recovery_blocked"

      assert final[Envelope.internal_key()][Evidence.key()] ==
               before[Envelope.internal_key()][Evidence.key()]

      assert_receive {:agent_request, "watchdog-agent", _, cancel}, 2_000
      assert cancel["method"] == "cancel_job"
      assert cancel["params"]["job_id"] == job
      assert Task.yield(bystander, 0) == nil
      send(other_agent, {:reply, %{"ok" => true, "result" => %{"unrelated" => true}}})
      assert {:ok, %{"unrelated" => true}} = Task.await(bystander, 2_000)
      assert {:ok, [%{"state" => "dispatch_boundary_unconfirmed"}]} = Journal.lookup_task(task_id)
      send(agent, {:reply, %{"ok" => true, "result" => %{}}})
      assert :ok = stop_supervised(Journal)
      start_supervised!({Journal, root: ctx.root})
      WorkflowCommitFixture.restart_coordinator()
      assert Coordinator.recover_now()["recovered"] == 0
      {:ok, restarted} = AnalysisResultStore.get(job)
      assert restarted[Envelope.internal_key()] == final[Envelope.internal_key()]
      refute_receive {:agent_request, _, _, _}, 30
      refute_receive {:solver_request, _, _}, 30
      assert [%{consecutive_failures: 0}] = AgentPool.endpoints()
    end
  end

  defp graph do
    port = %{"id" => "payload", "artifact_type" => "payload/json"}

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "native-evidence-pipeline",
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [port]},
        %{
          "id" => "diagnose",
          "kind" => "extract",
          "operator_id" => "extract.thermal_result_diagnostics",
          "inputs" => [port],
          "outputs" => [port],
          "config" => %{"on_error" => "skip"}
        }
      ],
      "edges" => [
        %{
          "id" => "input-diagnose",
          "artifact_type" => "payload/json",
          "from" => %{"node" => "input", "port" => "payload"},
          "to" => %{"node" => "diagnose", "port" => "payload"}
        }
      ]
    }
  end

  defp terminal(job, status, remaining \\ 300)
  defp terminal(_job, _status, 0), do: flunk("native pipeline did not terminate")

  defp terminal(job, status, remaining) do
    case Analysis.fetch_job(job) do
      {:ok, %{"job" => %{"status" => actual}, "result" => result}}
      when actual in ["completed", "failed"] ->
        assert actual in List.wrap(status)
        result

      _ ->
        Process.sleep(10)
        terminal(job, status, remaining - 1)
    end
  end
end
