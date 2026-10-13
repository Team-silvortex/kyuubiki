defmodule KyuubikiWeb.TestSupport.WorkflowCompletionProbe do
  @moduledoc false

  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator

  def install(job_id, phase \\ :commit_result) do
    token = make_ref()
    pid = Process.whereis(Coordinator)
    state = %{job_id: job_id, owner: self(), token: token, paused?: false, phase: phase}
    :ok = :sys.install(pid, {token, &__MODULE__.observe/3, state})

    ExUnit.Callbacks.on_exit(fn ->
      if Process.alive?(pid) do
        try do
          :sys.remove(pid, token)
        catch
          :exit, _ -> :ok
        end
      end
    end)

    {pid, token}
  end

  def resume({pid, token}), do: send(pid, {token, :continue})

  def observe(
        %{job_id: id, phase: phase} = state,
        {:in, {:"$gen_call", _from, {phase, id, claim, result}}},
        _name
      ) do
    send(state.owner, {:workflow_completion, state.token, self(), claim, result})

    unless state.paused? do
      token = state.token
      ref = Process.monitor(state.owner)

      receive do
        {^token, :continue} -> :ok
        {:DOWN, ^ref, :process, _owner, _reason} -> :ok
      after
        5_000 -> raise "workflow completion probe was not released"
      end

      Process.demonitor(ref, [:flush])
    end

    %{state | paused?: true}
  end

  def observe(state, _event, _name), do: state
end
