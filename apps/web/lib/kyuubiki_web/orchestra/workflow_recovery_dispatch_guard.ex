defmodule KyuubikiWeb.Orchestra.WorkflowRecoveryDispatchGuard do
  @moduledoc "Checks replay admission without promoting observations into execution authority."

  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowSolveCheckpoint
  alias KyuubikiWeb.Orchestra.WorkflowBranchCheckpoint
  alias KyuubikiWeb.Orchestra.WorkflowNativeResultCheckpoint
  alias KyuubikiWeb.Orchestra.WorkflowNativeDispatchEvidence, as: Evidence
  alias KyuubikiWeb.WorkflowNativeOperatorRuntime, as: Native

  def claim(job_id, recovery, session, reason, max_attempts, journal \\ Journal) do
    with :ok <- dispatch_reason(recovery, reason),
         :ok <- attempt_available(recovery, max_attempts),
         {:ok, claimed, claim} <- Envelope.claim(recovery, session, reason),
         :ok <- verify_solve_checkpoints(claimed, job_id),
         :ok <- verify_branch_checkpoints(claimed, job_id),
         :ok <- verify_native_result_checkpoints(claimed, job_id),
         :ok <- verify_native_dispatch(job_id, claimed, journal) do
      {:ok, claimed, claim}
    end
  end

  defp verify_native_result_checkpoints(recovery, job_id) do
    case WorkflowNativeResultCheckpoint.verify(recovery, job_id) do
      :ok -> :ok
      _ -> {:error, :workflow_native_result_checkpoint_invalid}
    end
  end

  defp verify_branch_checkpoints(recovery, job_id) do
    case WorkflowBranchCheckpoint.verify(recovery, job_id) do
      :ok -> :ok
      _ -> {:error, :workflow_branch_checkpoint_invalid}
    end
  end

  defp verify_solve_checkpoints(recovery, job_id) do
    case WorkflowSolveCheckpoint.verify(recovery, job_id) do
      :ok -> :ok
      _ -> {:error, :workflow_solve_checkpoint_invalid}
    end
  end

  def next_action(:workflow_dispatch_outcome_unconfirmed),
    do: "inspect_original_dispatch_before_explicit_resubmit"

  def next_action({:workflow_dispatch_journal_unavailable, _reason}),
    do: "restore_dispatch_journal_before_recovery"

  def next_action(:workflow_solve_checkpoint_invalid),
    do: "inspect_solve_checkpoint_before_explicit_resubmit"

  def next_action(:workflow_branch_checkpoint_invalid),
    do: "inspect_branch_checkpoint_before_explicit_resubmit"

  def next_action(:workflow_native_dispatch_evidence_invalid),
    do: "inspect_native_dispatch_evidence_before_explicit_resubmit"

  def next_action(:workflow_native_result_checkpoint_invalid),
    do: "inspect_native_result_checkpoint_before_explicit_resubmit"

  def next_action(:workflow_completion_checkpoint_invalid),
    do: "inspect_completion_checkpoint_without_reexecuting_workflow"

  def next_action(_reason), do: "supply_verified_checkpoint_or_resubmit"

  # Also used before ordinary failure cleanup; it does not claim or replay work.
  def verify_native_dispatch(job_id, recovery, journal \\ Journal) do
    with {:ok, envelope} <- Envelope.execution(recovery) do
      context = envelope["orchestration_context"]

      if context["job_id"] == job_id do
        with {:ok, tasks} <- native_tasks(envelope["graph"], context) do
          check_journal(job_id, tasks, recovery, journal)
        end
      else
        {:error, :workflow_recovery_identity_mismatch}
      end
    end
  end

  defp native_tasks(graph, context) do
    nodes = Map.get(graph, "nodes", [])

    if is_list(nodes) and Enum.all?(nodes, &is_map/1) do
      native = Enum.filter(nodes, &Native.supports?(&1["kind"], &1["operator_id"]))

      if Enum.all?(native, &(is_binary(&1["id"]) and byte_size(&1["id"]) > 0)),
        do: {:ok, Enum.map(native, &Native.task_id(context, &1, &1["operator_id"]))},
        else: {:error, :invalid_workflow_execution_envelope}
    else
      {:error, :invalid_workflow_execution_envelope}
    end
  end

  defp check_journal(_job_id, [], _recovery, _journal), do: :ok

  defp check_journal(job_id, tasks, recovery, journal) do
    if Evidence.verify(recovery, job_id) == :ok do
      check_expected(job_id, tasks, Evidence.expected(recovery), journal)
    else
      {:error, :workflow_native_dispatch_evidence_invalid}
    end
  end

  defp check_expected(job_id, tasks, expected, journal) do
    case Journal.workflow_replay_fence(job_id, tasks, expected, journal) do
      :ok -> :ok
      {:error, :workflow_dispatch_outcome_unconfirmed} = blocked -> blocked
      {:error, reason} -> {:error, {:workflow_dispatch_journal_unavailable, reason}}
    end
  end

  defp dispatch_reason(%{"state" => "pending", "generation" => 0, "attempt" => 0}, :initial),
    do: :ok

  defp dispatch_reason(_recovery, :initial), do: {:error, :workflow_already_dispatched}
  defp dispatch_reason(_recovery, _reason), do: :ok

  defp attempt_available(%{"attempt" => attempt}, max_attempts)
       when is_integer(attempt) and attempt < max_attempts,
       do: :ok

  defp attempt_available(_recovery, _max_attempts),
    do: {:error, :workflow_recovery_attempts_exhausted}
end
