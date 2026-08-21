defmodule KyuubikiWeb.Orchestra.DistributedRecoveryOperationalProbe do
  @moduledoc """
  Executes one recovery scenario against externally managed Rust Agents.

  Host lifecycle and fault injection stay in the native qualification runner;
  this module only exercises Orchestra routing and replay policy.
  """

  alias KyuubikiWeb.Orchestra.{OperatorTaskBatchRun, OperatorTaskIR}
  alias KyuubikiWeb.Playground.{AgentClient, AgentPool}

  @schema_version "kyuubiki.distributed-task-recovery-operational-probe/v1"
  @scenarios ~w(idempotent side_effect_blocked checkpointed)

  @spec run_from_env!() :: map()
  def run_from_env! do
    scenario = fetch_scenario!()

    primary =
      endpoint(
        "remote-primary",
        fetch_env!("KYUUBIKI_QUAL_PRIMARY_HOST"),
        fetch_port!("KYUUBIKI_QUAL_PRIMARY_PORT")
      )

    fallback = endpoint("local-fallback", "127.0.0.1", fetch_port!("KYUUBIKI_QUAL_FALLBACK_PORT"))
    report = run!(scenario, primary, fallback)
    output = fetch_env!("KYUUBIKI_QUAL_REPORT_PATH")
    File.write!(output, Jason.encode_to_iodata!(report, pretty: true))
    report
  end

  @spec run!(String.t(), map(), map()) :: map()
  def run!(scenario, primary, fallback)
      when scenario in @scenarios and is_map(primary) and is_map(fallback) do
    original_pool = Application.get_env(:kyuubiki_web, AgentPool, [])
    original_client = Application.get_env(:kyuubiki_web, AgentClient, [])

    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [primary, fallback])

    Application.put_env(:kyuubiki_web, AgentClient,
      connect_timeout_ms: 500,
      recv_timeout_ms: 2_000,
      request_timeout_ms: 15_000,
      queue_timeout_ms: 5_000
    )

    try do
      AgentPool.reload()
      reset_progress()
      execute_scenario!(scenario)
    after
      Application.put_env(:kyuubiki_web, AgentPool, original_pool)
      Application.put_env(:kyuubiki_web, AgentClient, original_client)
      AgentPool.reload()
      reset_progress()
    end
  end

  defp execute_scenario!("idempotent") do
    job_id = "distributed-recovery-idempotent"

    {:ok, result, endpoint} =
      AgentClient.request_with_agent(
        "solve_bar_1d",
        %{
          length: 1.0,
          area: 1.0,
          youngs_modulus: 1_000.0,
          elements: 64,
          tip_force: 10.0
        },
        progress_handler(),
        job_id: job_id,
        request_timeout_ms: 15_000
      )

    recovery = required_recovery!()

    build_report("idempotent", job_id, %{
      "outcome" => "fallback_completed",
      "fallback_agent_id" => endpoint.id,
      "result_max_stress" => result["max_stress"],
      "result_tip_displacement" => result["tip_displacement"],
      "recovery" => recovery
    })
  end

  defp execute_scenario!("side_effect_blocked") do
    job_id = "distributed-recovery-side-effect"
    task_ir = export_task_ir!()

    {:error, {:agent_retry_blocked, receipt}} =
      AgentClient.run_operator_task_ir(
        task_ir,
        [mode: :execute, job_id: job_id, request_timeout_ms: 15_000],
        progress_handler()
      )

    build_report("side_effect_blocked", job_id, %{
      "outcome" => "replay_blocked",
      "task_digest" => task_ir["task_digest"],
      "recovery" => receipt
    })
  end

  defp execute_scenario!("checkpointed") do
    job_id = "distributed-recovery-checkpointed"
    task_ir = export_task_ir!()
    checkpoint = verified_checkpoint!(task_ir)

    {:ok, result} =
      AgentClient.run_operator_task_ir(
        task_ir,
        [
          mode: :execute,
          job_id: job_id,
          request_timeout_ms: 15_000,
          retry_safety: :checkpointed,
          replay_checkpoint: checkpoint
        ],
        progress_handler()
      )

    recovery = required_recovery!()

    build_report("checkpointed", job_id, %{
      "outcome" => "checkpoint_authorized_fallback",
      "task_digest" => task_ir["task_digest"],
      "checkpoint_digest" => checkpoint["checkpoint_digest"],
      "result_status" => result["operator_task_ir_status"],
      "recovery" => recovery
    })
  end

  defp build_report(scenario, job_id, observations) do
    %{
      "schema_version" => @schema_version,
      "status" => "pass",
      "scenario" => scenario,
      "job_id" => job_id,
      "progress_event_count" => Process.get(:distributed_recovery_progress_count, 0),
      "observations" => observations
    }
  end

  defp progress_handler do
    fn progress ->
      Process.put(
        :distributed_recovery_progress_count,
        Process.get(:distributed_recovery_progress_count, 0) + 1
      )

      case progress do
        %{"recovery" => recovery} ->
          Process.put(:distributed_recovery_receipt, recovery)

        _ ->
          :ok
      end
    end
  end

  defp required_recovery! do
    Process.get(:distributed_recovery_receipt) ||
      raise "recovery progress receipt was not emitted"
  end

  defp reset_progress do
    Process.delete(:distributed_recovery_progress_count)
    Process.delete(:distributed_recovery_receipt)
  end

  defp export_task_ir! do
    {:ok, task_ir} =
      OperatorTaskIR.build(
        "export.summary_json",
        %{"summary" => %{"status" => "ready"}},
        %{},
        placement_tags: []
      )

    task_ir
  end

  defp verified_checkpoint!(task_ir) do
    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "operator_id" => task_ir["operator"]["id"],
      "tasks" => [%{"case_id" => "operational-recovery", "task_ir" => task_ir}]
    }

    checkpoint = OperatorTaskBatchRun.checkpoint(batch)
    {:ok, verification} = OperatorTaskBatchRun.verify_checkpoint(batch, checkpoint)
    verification
  end

  defp endpoint(id, host, port), do: %{id: id, host: host, port: port}

  defp fetch_scenario! do
    scenario = fetch_env!("KYUUBIKI_QUAL_SCENARIO")
    if scenario in @scenarios, do: scenario, else: raise("unsupported recovery scenario")
  end

  defp fetch_port!(name) do
    case Integer.parse(fetch_env!(name)) do
      {port, ""} when port in 1..65_535 -> port
      _ -> raise "#{name} must be a valid TCP port"
    end
  end

  defp fetch_env!(name), do: System.fetch_env!(name)
end
