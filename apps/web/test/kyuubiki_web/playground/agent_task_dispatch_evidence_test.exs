defmodule KyuubikiWeb.Playground.AgentTaskDispatchEvidenceTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.{OperatorDispatchJournal, OperatorTaskIR}
  alias KyuubikiWeb.Playground.{AgentExecutionGate, AgentRpcTransport, AgentTaskDispatch}

  for failure <- [:error, :bad_reply, :raise, :throw, :exit, :invalid] do
    test "#{failure} durable callback prevents even opening the Agent connection" do
      id = "dispatch-evidence-#{System.unique_integer([:positive])}"

      {:ok, task} =
        OperatorTaskIR.build("extract.thermal_result_diagnostics", %{}, %{}, task_id: id)

      {:ok, listener} =
        :gen_tcp.listen(0, [:binary, packet: 4, active: false, ip: {127, 0, 0, 1}])

      on_exit(fn -> :gen_tcp.close(listener) end)
      {:ok, port} = :inet.port(listener)
      endpoint = %{id: id, host: "127.0.0.1", port: port, capacity: 1}
      {:ok, _, _} = AgentExecutionGate.acquire([endpoint], id, 1_000, id)

      request = %{
        "rpc_version" => 1,
        "id" => id,
        "method" => "run_operator_task_ir",
        "params" => %{"task_ir" => task, "mode" => "execute", "job_id" => id}
      }

      try do
        assert {:error, :operator_task_dispatch_evidence_unavailable} =
                 AgentTaskDispatch.request(endpoint, id, request, fn _ -> :ok end,
                   request_timeout_ms: 50,
                   on_dispatch_reserved: callback(unquote(failure))
                 )

        if unquote(failure) != :invalid, do: assert_receive({:persist_attempt, _})
        assert {:error, :timeout} = :gen_tcp.accept(listener, 20)

        assert {:ok, [%{"state" => "dispatch_boundary_unconfirmed"}]} =
                 OperatorDispatchJournal.lookup_task(id)

        assert AgentRpcTransport.local_failure?(:operator_task_dispatch_evidence_unavailable)
      after
        AgentExecutionGate.release(id)
      end
    end
  end

  defp callback(:invalid), do: false

  defp callback(failure) do
    fn attempt ->
      send(self(), {:persist_attempt, attempt})

      case failure do
        :error -> {:error, :storage_unavailable}
        :bad_reply -> {:ok, "SECRET"}
        :raise -> raise "SECRET"
        :throw -> throw("SECRET")
        :exit -> exit("SECRET")
      end
    end
  end
end
