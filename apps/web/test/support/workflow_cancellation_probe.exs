defmodule KyuubikiWeb.TestSupport.WorkflowCancellationProbe do
  @moduledoc false

  def install(server, match, occurrence \\ 1) do
    pid = GenServer.whereis(server)
    token = make_ref()
    state = %{owner: self(), token: token, match: match, remaining: occurrence}
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

  def observe(%{remaining: count} = state, {:in, {:"$gen_call", _, message}}, _)
      when count > 0 do
    if state.match.(message) do
      if count == 1 do
        token = state.token
        ref = Process.monitor(state.owner)
        send(state.owner, {:cancellation_paused, token, message})

        receive do
          {^token, :continue} -> :ok
          {:DOWN, ^ref, :process, _, _} -> :ok
        after
          25_000 -> raise "cancellation probe was not released"
        end

        Process.demonitor(ref, [:flush])
      end

      %{state | remaining: count - 1}
    else
      state
    end
  end

  def observe(state, _, _), do: state
end
