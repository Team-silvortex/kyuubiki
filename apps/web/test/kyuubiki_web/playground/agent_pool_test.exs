defmodule KyuubikiWeb.Playground.AgentPoolTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentPool

  setup do
    original_config = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, original_config)
      AgentPool.reload()
    end)

    :ok
  end

  test "rotates endpoints in round-robin order" do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{id: "agent-a", host: "127.0.0.1", port: 5101},
        %{id: "agent-b", host: "127.0.0.1", port: 5102},
        %{id: "agent-c", host: "127.0.0.1", port: 5103}
      ]
    )

    assert :ok = AgentPool.reload()

    assert Enum.map(AgentPool.checkout_endpoints(), & &1.id) == ["agent-a", "agent-b", "agent-c"]
    assert Enum.map(AgentPool.checkout_endpoints(), & &1.id) == ["agent-b", "agent-c", "agent-a"]
    assert Enum.map(AgentPool.checkout_endpoints(), & &1.id) == ["agent-c", "agent-a", "agent-b"]
  end
end
