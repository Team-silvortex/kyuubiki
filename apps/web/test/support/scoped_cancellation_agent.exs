defmodule KyuubikiWeb.TestSupport.ScopedCancellationAgent do
  @moduledoc false

  def child_spec({_, name} = args),
    do: %{id: {__MODULE__, name}, start: {__MODULE__, :start_link, [args]}, restart: :temporary}

  def start_link({owner, name}) do
    Task.start_link(fn ->
      {:ok, state} = Agent.start_link(fn -> %{next: 1, active: %{}} end)

      {:ok, listener} =
        :gen_tcp.listen(0, [:binary, packet: 4, active: false, ip: {127, 0, 0, 1}])

      {:ok, port} = :inet.port(listener)
      send(owner, {:cancellable_agent_ready, name, port})
      accept(listener, owner, name, state)
    end)
  end

  defp accept(listener, owner, name, state) do
    case :gen_tcp.accept(listener) do
      {:ok, socket} ->
        handler = spawn_link(fn -> handle(owner, name, state) end)
        :ok = :gen_tcp.controlling_process(socket, handler)
        send(handler, {:socket, socket})
        accept(listener, owner, name, state)

      {:error, :closed} ->
        :ok
    end
  end

  defp handle(owner, name, state) do
    receive do
      {:socket, socket} ->
        try do
          {:ok, payload} = :gen_tcp.recv(socket, 0, 2_000)
          request = Jason.decode!(payload)
          frame = reply(request, owner, name, state)

          :gen_tcp.send(
            socket,
            Jason.encode!(Map.merge(frame, Map.take(request, ~w(id rpc_version))))
          )
        after
          :gen_tcp.close(socket)
        end
    end
  end

  defp reply(%{"method" => "describe_agent"}, _owner, name, state) do
    active = Agent.get(state, &Map.values(&1.active))

    %{
      "ok" => true,
      "result" => %{
        "lifecycle" => %{"process_instance_id" => name <> "-process"},
        "solver_control" => %{
          "schema_version" => "kyuubiki.agent-solver-control/v1",
          "available" => true,
          "active" => active
        }
      }
    }
  end

  defp reply(%{"method" => "cancel_execution", "params" => target} = request, owner, name, state) do
    matched =
      Agent.get_and_update(state, fn current ->
        id = target["request_id"]
        entry = current.active[id]

        matches =
          is_map(entry) and target["process_instance_id"] == name <> "-process" and
            Map.take(entry, ~w(request_id generation job_id)) ==
              Map.delete(target, "process_instance_id")

        next =
          if matches, do: put_in(current, [:active, id, "cancel_requested"], true), else: current

        {matches, next}
      end)

    send(owner, {:agent_request, name, self(), request})

    %{
      "ok" => true,
      "result" => %{
        "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
        "execution_target" => target,
        "status" => if(matched, do: "requested", else: "target_not_observed"),
        "cancel_registered" => matched,
        "execution_terminal_confirmed" => false,
        "pending_cancellation_created" => false,
        "operator_package_cleanup_performed" => false,
        "automatic_replay_authorized" => false
      }
    }
  end

  defp reply(%{"method" => "cancel_job"} = request, owner, name, _state) do
    send(owner, {:agent_request, name, self(), request})

    %{
      "ok" => true,
      "result" => %{"cancel_registered" => true, "job_id" => request["params"]["job_id"]}
    }
  end

  defp reply(request, owner, name, state) do
    Agent.update(state, fn current ->
      entry = %{
        "request_id" => request["id"],
        "job_id" => request["params"]["job_id"],
        "generation" => current.next,
        "cancel_requested" => false
      }

      %{current | next: current.next + 1, active: Map.put(current.active, request["id"], entry)}
    end)

    send(owner, {:agent_request, name, self(), request})

    frame =
      receive do
        {:reply, frame} -> frame
      after
        10_000 ->
          %{"ok" => false, "error" => %{"code" => "test_timeout", "message" => "test timeout"}}
      end

    Agent.update(state, &%{&1 | active: Map.delete(&1.active, request["id"])})
    frame
  end
end
