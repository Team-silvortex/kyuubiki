defmodule KyuubikiWeb.Orchestra.WorkflowNativeResultRestartTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.{
    Analysis,
    AnalysisResultStore,
    WorkflowNativeOperatorRuntime,
    WorkflowOperatorRuntime
  }

  alias KyuubikiWeb.Orchestra.{OperatorTaskExecutionSummary, OperatorTaskReadiness}
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowNativeResultCheckpoint, as: Checkpoint
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Playground.{AgentClient, AgentPool}
  alias KyuubikiWeb.TestSupport.{CancellableSolverAgent, WorkflowCommitFixture}

  defmodule Solver do
    def configure(owner, mode), do: :persistent_term.put(__MODULE__, {owner, mode})
    def clear, do: :persistent_term.erase(__MODULE__)

    def request("solve_bar_1d", payload, _progress, _opts) do
      {owner, mode} = :persistent_term.get(__MODULE__)
      send(owner, {:downstream_solve, mode, payload})

      case mode do
        :success ->
          {:ok, payload}

        :hold ->
          receive do
            :release -> {:ok, payload}
          after
            60_000 -> {:error, :timeout}
          end
      end
    end
  end

  setup do
    WorkflowCommitFixture.setup_coordinator()

    settings =
      for module <- [AgentPool, WorkflowNativeOperatorRuntime, WorkflowOperatorRuntime],
          do: {module, Application.get_env(:kyuubiki_web, module, [])}

    Application.put_env(:kyuubiki_web, WorkflowNativeOperatorRuntime, client: AgentClient)
    Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, solve_runtime_client: Solver)
    Solver.configure(self(), :hold)
    start_supervised!({CancellableSolverAgent, {self(), :native_handoff}})
    assert_receive {:cancellable_agent_ready, :native_handoff, port}

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "native-handoff-agent",
          host: "127.0.0.1",
          port: port,
          methods: ["run_operator_task_ir"],
          capabilities: ["solver_rpc"],
          operator_package_runtime: %{"ready" => false}
        }
      ]
    )

    AgentPool.reload()

    on_exit(fn ->
      Application.ensure_all_started(:kyuubiki_web)
      for {module, value} <- settings, do: Application.put_env(:kyuubiki_web, module, value)
      AgentPool.reload()
      Solver.clear()
    end)

    :ok
  end

  for {kind, operator} <- [
        {"extract", "extract.thermal_result_diagnostics"},
        {"transform", "transform.score_thermal_quality"}
      ],
      fault <- [
        :none,
        :missing_checkpoint,
        :corrupt_checkpoint,
        :missing_journal,
        :replay_restricted
      ] do
    test "#{kind} handoff survives application restart with Agent offline: #{fault}" do
      graph = graph(unquote(kind), unquote(operator))

      graph =
        if unquote(fault) == :replay_restricted,
          do: Map.put(graph, "recovery_policy", %{"retry_safety" => "checkpoint_required"}),
          else: graph

      {:ok, admitted} =
        Analysis.submit_workflow_graph(%{
          "graph" => graph,
          "input_artifacts" => %{"input" => %{}}
        })

      job = admitted["job"]["job_id"]
      assert_receive {:agent_request, :native_handoff, agent, request}, 2_000
      assert request["method"] == "run_operator_task_ir"
      attempt = request["params"]["dispatch_attempt_id"]
      {:ok, summary} = OperatorTaskExecutionSummary.build(request["params"]["task_ir"])
      output = %{"value" => 1.0, "tiny" => 1.0e-30}

      receipt =
        Map.merge(Map.take(summary, ~w(task_id task_digest operator_id program_id)), %{
          "operator_task_ir_status" => "executed",
          "execution_readiness" => OperatorTaskReadiness.local_executed(),
          "result" => output
        })

      send(agent, {:reply, %{"ok" => true, "result" => receipt}})
      assert_receive {:downstream_solve, :hold, ^output}, 2_000
      {:ok, runtime} = AnalysisResultStore.get(job)
      key = Envelope.internal_key()
      assert map_size(runtime[key][Checkpoint.key()]["entries"]) == 1
      assert :ok = Checkpoint.verify(runtime[key], job)
      assert runtime[key][Checkpoint.key()]["entries"][attempt]["receipt_json"] =~ "1.0e-30"
      journal_path = Path.join(Journal.root(), attempt <> ".json")
      journal_bytes = File.read!(journal_path)
      changed = change(runtime, attempt, unquote(fault))
      assert :ok = AnalysisResultStore.put(job, changed)

      # Restart the real application/store, not only the recovery function. The
      # controlled TCP producer is gone, so neither receipt fetch nor replay can help.
      :ok = Application.stop(:kyuubiki_web)
      :ok = stop_supervised({CancellableSolverAgent, :native_handoff})
      if unquote(fault) == :missing_journal, do: File.rm!(journal_path)
      Application.put_env(:kyuubiki_web, AgentPool, endpoints: [])
      Solver.configure(self(), :success)
      assert {:ok, _} = Application.ensure_all_started(:kyuubiki_web)

      if unquote(fault) == :none do
        assert_receive {:downstream_solve, :success, ^output}, 2_000
        result = terminal(job, "completed")
        assert result["artifacts"]["native.payload"] === output
        assert result["recovery"]["generation"] == 2
        assert result["recovery"]["native_result_checkpoint_count"] == 0
        {:ok, final} = AnalysisResultStore.get(job)
        refute Map.has_key?(final[key], Checkpoint.key())
      else
        result = terminal(job, "failed")
        assert result["recovery"]["state"] == "recovery_blocked"
        refute_receive {:downstream_solve, :success, _}, 20
        {:ok, final} = AnalysisResultStore.get(job)
        assert final[key][Checkpoint.key()] == changed[key][Checkpoint.key()]
      end

      refute_receive {:agent_request, _, _, _}, 20

      unless unquote(fault) == :missing_journal,
        do: assert(File.read!(journal_path) == journal_bytes)
    end
  end

  defp change(runtime, attempt, :corrupt_checkpoint),
    do:
      put_in(
        runtime,
        [Envelope.internal_key(), Checkpoint.key(), "entries", attempt, "receipt_json"],
        "{}"
      )

  defp change(runtime, _attempt, :missing_checkpoint),
    do: Map.update!(runtime, Envelope.internal_key(), &Map.delete(&1, Checkpoint.key()))

  defp change(runtime, _attempt, _), do: runtime

  defp graph(kind, operator) do
    port = %{"id" => "payload", "artifact_type" => "payload/json"}

    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "native-result-handoff",
      "nodes" => [
        %{"id" => "input", "kind" => "input", "outputs" => [port]},
        %{
          "id" => "native",
          "kind" => kind,
          "operator_id" => operator,
          "inputs" => [port],
          "outputs" => [port],
          "config" => %{"on_error" => "skip"}
        },
        %{
          "id" => "solve",
          "kind" => "solve",
          "operator_id" => "solve.bar_1d",
          "inputs" => [port],
          "outputs" => [port]
        }
      ],
      "edges" =>
        for {from, to} <- [{"input", "native"}, {"native", "solve"}] do
          %{
            "id" => from <> to,
            "artifact_type" => "payload/json",
            "from" => %{"node" => from, "port" => "payload"},
            "to" => %{"node" => to, "port" => "payload"}
          }
        end
    }
  end

  defp terminal(job, status, remaining \\ 300)
  defp terminal(_, _, 0), do: flunk("native result handoff did not terminate")

  defp terminal(job, status, remaining) do
    case Analysis.fetch_job(job) do
      {:ok, %{"job" => %{"status" => ^status}, "result" => result}} ->
        result

      _ ->
        Process.sleep(10)
        terminal(job, status, remaining - 1)
    end
  end
end
