defmodule KyuubikiWeb.Playground.AgentRpcTransportBoundaryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.Playground.AgentExecutionGate
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  setup do
    original_pool = Application.get_env(:kyuubiki_web, AgentPool, [])
    original_client = Application.get_env(:kyuubiki_web, AgentClient, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, original_pool)
      Application.put_env(:kyuubiki_web, AgentClient, original_client)
      AgentPool.reload()
    end)

    :ok
  end

  test "queue callback failure returns an error before reserving capacity" do
    configure_endpoint(%{id: "queue-callback", host: "127.0.0.1", port: 59_991})

    assert {:error, {:progress_callback_failed, :error, "queue sink failed"}} =
             AgentClient.request_with_agent(
               "solve_bar_1d",
               %{},
               fn _progress -> raise "queue sink failed" end,
               job_id: "queue-callback-job"
             )

    assert %{active_lease_count: 0, queued_request_count: 0} =
             AgentExecutionGate.snapshot()

    assert [endpoint] = AgentPool.endpoints()
    assert endpoint.consecutive_failures == 0
  end

  test "dispatch callback failure releases acquired capacity" do
    configure_endpoint(%{id: "dispatch-callback", host: "127.0.0.1", port: 59_992})

    callback = fn
      %{"stage" => "queued"} -> :ok
      %{"stage" => "preprocessing"} -> raise "dispatch sink failed"
    end

    assert {:error, {:progress_callback_failed, :error, "dispatch sink failed"}} =
             AgentClient.request_with_agent(
               "solve_bar_1d",
               %{},
               callback,
               job_id: "dispatch-callback-job"
             )

    assert %{active_lease_count: 0, queued_request_count: 0} =
             AgentExecutionGate.snapshot()

    assert [endpoint] = AgentPool.endpoints()
    assert endpoint.consecutive_failures == 0
  end

  test "remote progress callback failure does not degrade a healthy agent" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{
          "event" => "progress",
          "progress" => %{"stage" => "solving", "progress" => 0.5}
        }
      ])

    port = await_fake_agent_port()
    configure_endpoint(%{id: "remote-callback", host: "127.0.0.1", port: port})

    assert {:error, {:progress_callback_failed, :error, "remote sink failed"}} =
             AgentClient.solve_bar_1d(%{}, fn _progress -> raise "remote sink failed" end)

    assert [endpoint] = AgentPool.endpoints()
    assert endpoint.consecutive_failures == 0
    assert endpoint.last_failure_reason == nil
  end

  test "explicit callback errors are observable and remain local" do
    configure_endpoint(%{id: "returned-callback", host: "127.0.0.1", port: 59_993})

    assert {:error, {:progress_callback_failed, :returned_error, ":persistence_unavailable"}} =
             AgentClient.request_with_agent(
               "solve_bar_1d",
               %{},
               fn _progress -> {:error, :persistence_unavailable} end,
               job_id: "returned-callback-job"
             )

    assert %{active_lease_count: 0} = AgentExecutionGate.snapshot()
  end

  test "unencodable requests fail before endpoint selection" do
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [])
    AgentPool.reload()

    assert {:error, {:request_encoding_failed, detail}} =
             AgentClient.request("custom_method", %{"caller" => self()})

    assert detail =~ "Jason.Encoder"
  end

  test "malformed progress payload is a protocol failure" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{"event" => "progress", "progress" => "not-a-map"}
      ])

    port = await_fake_agent_port()
    configure_endpoint(%{id: "malformed-progress", host: "127.0.0.1", port: port})

    assert {:error, {:all_agents_failed, [receipt]}} = AgentClient.ping()
    assert receipt.failure_stage == "protocol"
    assert receipt.reason_code == "agent_protocol_failure"
    assert receipt.reason =~ "invalid_progress_payload"

    assert [endpoint] = AgentPool.endpoints()
    assert endpoint.consecutive_failures == 1
  end

  test "oversized RPC frames are rejected before JSON allocation" do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link([
        %{"ok" => true, "result" => %{"payload" => String.duplicate("x", 2_048)}}
      ])

    port = await_fake_agent_port()
    configure_endpoint(%{id: "oversized-frame", host: "127.0.0.1", port: port})
    Application.put_env(:kyuubiki_web, AgentClient, max_rpc_frame_bytes: 512)

    assert {:error, {:all_agents_failed, [receipt]}} = AgentClient.ping()
    assert receipt.failure_stage == "receive"
    assert receipt.reason =~ "emsgsize"
  end

  defp configure_endpoint(endpoint) do
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: [endpoint])
    AgentPool.reload()
  end

  defp await_fake_agent_port do
    receive do
      {:fake_agent_ready, port} -> port
    after
      1_000 -> flunk("timed out waiting for fake agent port")
    end
  end
end
