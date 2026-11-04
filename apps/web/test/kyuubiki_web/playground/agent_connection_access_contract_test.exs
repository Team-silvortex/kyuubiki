defmodule KyuubikiWeb.Playground.AgentConnectionAccessContractTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Playground.AgentConnectionPreflight, as: Preflight

  test "connection access accepts only the exact three native policy observations" do
    for {peer, mode, allowed, mutations} <- [
          {"host_loopback", "host_loopback", true, true},
          {"non_loopback", "trusted_lan_control", true, false},
          {"non_loopback", "discovery_only", false, false}
        ] do
      access = %{
        "schema_version" => "kyuubiki.agent-connection-access/v1",
        "scope" => "native_socket_connection",
        "peer_scope" => peer,
        "control_mode" => mode,
        "control_allowed" => allowed,
        "lifecycle_mutations_allowed" => mutations,
        "authenticated" => false,
        "encrypted" => false
      }

      assert Preflight.validate_descriptor(%{"connection_access" => access}) ==
               if(allowed, do: :ok, else: {:error, :connection_control_not_permitted})

      for field <- Map.keys(access) do
        assert_invalid(Map.delete(access, field))
        assert_invalid(Map.put(access, field, nil))
      end

      for {key, value} <- [
            {"schema_version", "future"},
            {"scope", "global"},
            {"peer_scope", "authenticated"},
            {"control_mode", "authenticated"},
            {"control_allowed", not allowed},
            {"lifecycle_mutations_allowed", not mutations},
            {"authenticated", true},
            {"encrypted", true},
            {"credential", "not-authority"}
          ] do
        assert_invalid(Map.put(access, key, value))
      end
    end

    for value <- [nil, true, [], "allowed", %{}], do: assert_invalid(value)

    assert Preflight.validate_descriptor(%{"authenticated" => true}) ==
             {:error, :invalid_connection_access}
  end

  test "only literal loopback new-work targets bypass probing; metadata cannot grant access" do
    requests = [
      %{"method" => "solve_bar_1d"},
      %{"method" => "run_operator_task_ir", "params" => %{"mode" => "execute"}}
    ]

    for request <- requests do
      for host <- ["127.0.0.1", "127.42.0.7", "::1", "::ffff:127.0.0.1", "::ffff:7f2a:7"] do
        refute Preflight.required?(%{host: host}, request), host
      end

      for host <- [
            "localhost",
            "LOCALHOST",
            "localhost.",
            "192.0.2.1",
            "::ffff:192.0.2.1",
            "agent.local",
            "127.0.0.1.example",
            "127.0.0.1 ",
            "[::1]",
            "::",
            nil
          ] do
        assert Preflight.required?(
                 %{
                   host: host,
                   connection_access: %{"control_allowed" => true},
                   control_mode: "standalone",
                   authenticated: true
                 },
                 request
               ),
               inspect(host)
      end
    end
  end

  test "discovery and pinned recovery operations do not acquire new-work eligibility" do
    for method <-
          ~w(ping describe_agent describe_agent_lifecycle fetch_durable_agent_execution
      list_durable_agent_executions cancel_durable_agent_execution cancel_job release_operator_package_job) do
      refute Preflight.required?(%{host: "192.0.2.1"}, %{"method" => method})
    end

    refute Preflight.required?(
             %{host: "192.0.2.1"},
             %{"method" => "run_operator_task_ir", "params" => %{"mode" => "inspect"}}
           )
  end

  defp assert_invalid(access),
    do:
      assert(
        Preflight.validate_descriptor(%{"connection_access" => access}) ==
          {:error, :invalid_connection_access}
      )
end
