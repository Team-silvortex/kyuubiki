defmodule KyuubikiWeb.TestSupport.CancellableSolverAgent do
  @moduledoc false

  def child_spec({_, name} = args) do
    %{id: {__MODULE__, name}, start: {__MODULE__, :start_link, [args]}, restart: :temporary}
  end

  def start_link({owner, name}) do
    Task.start_link(fn ->
      {:ok, listener} =
        :gen_tcp.listen(0, [:binary, packet: 4, active: false, ip: {127, 0, 0, 1}])

      {:ok, port} = :inet.port(listener)
      send(owner, {:cancellable_agent_ready, name, port})
      accept(listener, owner, name)
    end)
  end

  defp accept(listener, owner, name) do
    case :gen_tcp.accept(listener) do
      {:ok, socket} ->
        handler = spawn_link(fn -> receive_request(owner, name) end)
        :ok = :gen_tcp.controlling_process(socket, handler)
        send(handler, {:socket, socket})
        accept(listener, owner, name)

      {:error, :closed} ->
        :ok
    end
  end

  defp receive_request(owner, name) do
    receive do
      {:socket, socket} ->
        try do
          {:ok, payload} = :gen_tcp.recv(socket, 0, 2_000)
          request = Jason.decode!(payload)
          send(owner, {:agent_request, name, self(), request})

          frame =
            if request["method"] == "cancel_job" do
              %{
                "ok" => true,
                "result" => %{
                  "job_id" => request["params"]["job_id"],
                  "cancel_registered" => true,
                  "execution_terminal_confirmed" => false,
                  "cancelled" => true
                }
              }
            else
              receive do
                {:reply, frame} -> frame
              after
                5_000 ->
                  %{
                    "ok" => false,
                    "error" => %{"code" => "test_timeout", "message" => "test timeout"}
                  }
              end
            end

          :gen_tcp.send(
            socket,
            Jason.encode!(Map.merge(frame, Map.take(request, ["id", "rpc_version"])))
          )
        after
          :gen_tcp.close(socket)
        end
    end
  end
end
