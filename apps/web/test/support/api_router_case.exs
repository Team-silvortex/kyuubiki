defmodule KyuubikiWeb.TestSupport.ApiRouterCase do
  @moduledoc false

  use ExUnit.CaseTemplate

  using do
    quote do
      use ExUnit.Case, async: false

      import Plug.Conn
      import Plug.Test
      import KyuubikiWeb.TestSupport.WorkflowApi

      alias KyuubikiWeb.AnalysisResultStore
      alias KyuubikiWeb.Jobs.Store
      alias KyuubikiWeb.Library
      alias KyuubikiWeb.Playground.AgentPool
      alias KyuubikiWeb.Playground.AgentRegistry
      alias KyuubikiWeb.Router
      alias KyuubikiWeb.SecurityEvents.Store, as: SecurityEventStore
      alias KyuubikiWeb.TestSupport.FakePlaygroundAgent
      alias KyuubikiWeb.TestSupport.WorkflowApi

      @opts Router.init([])
    end
  end

  setup do
    KyuubikiWeb.Jobs.Store.reset()
    KyuubikiWeb.AnalysisResultStore.reset()
    KyuubikiWeb.Library.reset()
    KyuubikiWeb.SecurityEvents.Store.reset()

    Enum.each(KyuubikiWeb.Playground.AgentRegistry.agents(), fn agent ->
      KyuubikiWeb.Playground.AgentRegistry.unregister(agent.id)
    end)

    original_config = Application.get_env(:kyuubiki_web, KyuubikiWeb.Playground.AgentPool, [])
    original_security = Application.get_env(:kyuubiki_web, KyuubikiWeb.Security, [])

    on_exit(fn ->
      Enum.each(KyuubikiWeb.Playground.AgentRegistry.agents(), fn agent ->
        KyuubikiWeb.Playground.AgentRegistry.unregister(agent.id)
      end)

      Application.put_env(:kyuubiki_web, KyuubikiWeb.Playground.AgentPool, original_config)
      Application.put_env(:kyuubiki_web, KyuubikiWeb.Security, original_security)
      KyuubikiWeb.Playground.AgentPool.reload()
    end)

    :ok
  end
end
