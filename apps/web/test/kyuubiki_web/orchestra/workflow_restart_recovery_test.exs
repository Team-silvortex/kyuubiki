defmodule KyuubikiWeb.Orchestra.WorkflowRestartRecoveryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.WorkflowOperatorRuntime

  defmodule RestartRuntimeClient do
    @state_key {__MODULE__, :state}

    def configure(owner, mode)
        when is_pid(owner) and mode in [:hold, :succeed, :crash_then_succeed] do
      :persistent_term.put(@state_key, %{owner: owner, mode: mode})
    end

    def clear, do: :persistent_term.erase(@state_key)

    def request("solve_bar_1d", payload, _on_progress, opts) do
      %{owner: owner, mode: mode} = :persistent_term.get(@state_key)
      send(owner, {:restart_runtime_request, Keyword.get(opts, :job_id), mode})

      case mode do
        :hold ->
          receive do
            :release_restart_runtime -> {:ok, Map.put(payload, "recovered", false)}
          after
            60_000 -> {:error, :restart_runtime_hold_timeout}
          end

        :succeed ->
          {:ok, Map.put(payload, "recovered", true)}

        :crash_then_succeed ->
          configure(owner, :succeed)
          exit(:injected_workflow_runner_loss)
      end
    end
  end

  setup do
    original_runtime = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])

    Store.reset()
    AnalysisResultStore.reset()
    configure_runtime_client(original_runtime)
    RestartRuntimeClient.configure(self(), :hold)

    on_exit(fn ->
      ensure_application_started!()
      Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original_runtime)
      RestartRuntimeClient.clear()
      Store.reset()
      AnalysisResultStore.reset()
    end)

    :ok
  end

  test "replays an idempotent in-flight workflow after the Orchestra application restarts" do
    {:ok, payload} = submit_workflow(idempotent_graph())
    job_id = payload["job"]["job_id"]

    assert_receive {:restart_runtime_request, ^job_id, :hold}, 2_000
    assert {:ok, %{status: :solving}} = Store.get(job_id)

    assert {:error, :active_workflow_result_is_read_only} =
             Analysis.update_result(job_id, %{"forged" => true})

    assert {:error, :active_workflow_result_is_read_only} = Analysis.delete_result(job_id)

    restart_application(:succeed)

    assert_receive {:restart_runtime_request, ^job_id, :succeed}, 2_000
    result = wait_for_terminal_result(job_id, "completed")

    assert result["artifacts"]["output.result"]["recovered"] == true
    assert result["recovery"]["state"] == "completed"
    assert result["recovery"]["generation"] == 2
    assert result["recovery"]["attempt"] == 2
    assert result["recovery"]["envelope_retained"] == false

    assert Enum.any?(result["recovery"]["history"], fn event ->
             event["event"] == "claimed" and event["reason"] == "process_restart"
           end)
  end

  test "blocks an in-flight side-effect workflow after restart without a checkpoint" do
    graph =
      idempotent_graph()
      |> Map.put("id", "workflow.restart-side-effect-test")
      |> Map.put("recovery_policy", %{"retry_safety" => "checkpoint_required"})

    {:ok, payload} = submit_workflow(graph)
    job_id = payload["job"]["job_id"]

    assert_receive {:restart_runtime_request, ^job_id, :hold}, 2_000
    restart_application(:succeed)

    result = wait_for_terminal_result(job_id, "failed")
    refute_receive {:restart_runtime_request, ^job_id, :succeed}, 200

    assert result["recovery"]["state"] == "recovery_blocked"
    assert result["recovery"]["retry_safety"] == "checkpoint_required"
    assert result["recovery"]["generation"] == 1
    assert result["recovery"]["attempt"] == 1
    assert result["recovery"]["envelope_retained"] == true

    assert {:ok, job} = Store.get(job_id)
    assert job.message =~ "workflow recovery blocked"
    assert job.message =~ "checkpoint_required"
  end

  test "blocks a digest-tampered execution envelope before restart replay" do
    {:ok, payload} = submit_workflow(idempotent_graph())
    job_id = payload["job"]["job_id"]

    assert_receive {:restart_runtime_request, ^job_id, :hold}, 2_000
    assert {:ok, runtime} = AnalysisResultStore.get(job_id)

    internal_key = KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope.internal_key()

    tampered =
      put_in(runtime, [internal_key, "envelope", "input_artifacts", "input", "value"], 99)

    :ok = AnalysisResultStore.put(job_id, tampered)

    restart_application(:succeed)

    result = wait_for_terminal_result(job_id, "failed")
    refute_receive {:restart_runtime_request, ^job_id, :succeed}, 200
    assert result["recovery"]["state"] == "recovery_blocked"

    assert {:ok, job} = Store.get(job_id)
    assert job.message =~ "workflow_recovery_digest_mismatch"
  end

  @tag capture_log: true
  test "reclaims an idempotent workflow after its supervised runner is lost" do
    RestartRuntimeClient.configure(self(), :crash_then_succeed)
    {:ok, payload} = submit_workflow(idempotent_graph())
    job_id = payload["job"]["job_id"]

    assert_receive {:restart_runtime_request, ^job_id, :crash_then_succeed}, 2_000
    assert_receive {:restart_runtime_request, ^job_id, :succeed}, 2_000

    result = wait_for_terminal_result(job_id, "completed")
    assert result["recovery"]["attempt"] == 2
    assert result["recovery"]["generation"] == 2

    assert Enum.any?(result["recovery"]["history"], fn event ->
             event["event"] == "claimed" and event["reason"] == "runner_loss"
           end)

    wait_for_runner_exit(job_id)
    Process.sleep(20)

    assert {:ok, %{"job" => %{"status" => "completed"}, "result" => stable_result}} =
             Analysis.fetch_job(job_id)

    assert stable_result["recovery"]["state"] == "completed"
  end

  test "fails submission when the workflow runner supervisor is unavailable" do
    result =
      without_task_supervisor(fn ->
        submit_workflow(idempotent_graph())
      end)

    assert {:error, {:workflow_runner_start_failed, :workflow_runner_supervisor_unavailable}} =
             result

    [job] = Store.list()
    assert job.status == :failed
    assert job.message =~ "workflow runner start failed"

    assert {:ok, runtime} = Analysis.fetch_job(job.job_id)
    assert runtime["result"]["recovery"]["state"] == "failed"
    assert runtime["result"]["recovery"]["envelope_retained"] == false
  end

  defp submit_workflow(graph) do
    Analysis.submit_workflow_graph(%{
      "graph" => graph,
      "input_artifacts" => %{"input" => %{"value" => 41}}
    })
  end

  defp idempotent_graph do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.restart-idempotent-test",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{
          "id" => "input",
          "kind" => "input",
          "outputs" => [%{"id" => "model", "artifact_type" => "model/bar_1d"}]
        },
        %{
          "id" => "solve",
          "kind" => "solve",
          "operator_id" => "solve.bar_1d",
          "inputs" => [%{"id" => "model", "artifact_type" => "model/bar_1d"}],
          "outputs" => [%{"id" => "result", "artifact_type" => "result/bar_1d"}]
        },
        %{
          "id" => "output",
          "kind" => "output",
          "inputs" => [%{"id" => "result", "artifact_type" => "result/bar_1d"}],
          "outputs" => []
        }
      ],
      "edges" => [
        %{
          "id" => "e0",
          "from" => %{"node" => "input", "port" => "model"},
          "to" => %{"node" => "solve", "port" => "model"},
          "artifact_type" => "model/bar_1d"
        },
        %{
          "id" => "e1",
          "from" => %{"node" => "solve", "port" => "result"},
          "to" => %{"node" => "output", "port" => "result"},
          "artifact_type" => "result/bar_1d"
        }
      ]
    }
  end

  defp restart_application(next_mode) do
    :ok = Application.stop(:kyuubiki_web)
    RestartRuntimeClient.configure(self(), next_mode)
    ensure_application_started!()
  end

  defp without_task_supervisor(callback) do
    :ok =
      Supervisor.terminate_child(
        KyuubikiWeb.Supervisor,
        KyuubikiWeb.TaskSupervisor
      )

    try do
      callback.()
    after
      case Supervisor.restart_child(KyuubikiWeb.Supervisor, KyuubikiWeb.TaskSupervisor) do
        {:ok, _pid} -> :ok
        {:ok, _pid, _info} -> :ok
        {:error, :running} -> :ok
        {:error, reason} -> raise "failed to restart workflow Task.Supervisor: #{inspect(reason)}"
      end
    end
  end

  defp ensure_application_started! do
    case Application.ensure_all_started(:kyuubiki_web) do
      {:ok, _started} -> :ok
      {:error, {:already_started, :kyuubiki_web}} -> :ok
      {:error, reason} -> raise "failed to start kyuubiki_web: #{inspect(reason)}"
    end
  end

  defp configure_runtime_client(original_runtime) do
    updated = Keyword.put(original_runtime, :solve_runtime_client, RestartRuntimeClient)
    Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, updated)
  end

  defp wait_for_terminal_result(job_id, expected_status, attempts \\ 300)

  defp wait_for_terminal_result(_job_id, expected_status, 0) do
    flunk("timed out waiting for restarted workflow status #{expected_status}")
  end

  defp wait_for_terminal_result(job_id, expected_status, attempts) do
    case Analysis.fetch_job(job_id) do
      {:ok, %{"job" => %{"status" => ^expected_status}, "result" => result}} ->
        result

      _ ->
        Process.sleep(10)
        wait_for_terminal_result(job_id, expected_status, attempts - 1)
    end
  end

  defp wait_for_runner_exit(job_id, attempts \\ 100)

  defp wait_for_runner_exit(_job_id, 0), do: flunk("timed out waiting for workflow runner exit")

  defp wait_for_runner_exit(job_id, attempts) do
    case KyuubikiWeb.Orchestra.WorkflowJobRunner.running(job_id) do
      :error ->
        :ok

      {:ok, _pid} ->
        Process.sleep(5)
        wait_for_runner_exit(job_id, attempts - 1)
    end
  end
end
