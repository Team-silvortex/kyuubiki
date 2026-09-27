defmodule KyuubikiWeb.Api.StorageOutageHealthTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.Jobs.Watchdog

  setup do
    Outage.setup()
  end

  test "health remains reachable but degraded while storage is unavailable" do
    Outage.without(Outage.store_child(), fn ->
      assert %{available: false} = Watchdog.scan_now()
      conn = conn(:get, "/api/health") |> Router.call(Router.init([]))
      assert conn.status == 200
      payload = Jason.decode!(conn.resp_body)
      assert payload["status"] == "degraded"
      assert payload["watchdog"]["available"] == false
      assert payload["watchdog"]["active_jobs"] == nil
      assert payload["watchdog"]["reason"] == "analysis_store_unavailable"
    end)

    conn = conn(:get, "/api/health") |> Router.call(Router.init([]))
    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "degraded"
    assert payload["watchdog"]["available"] == true
    assert payload["watchdog"]["last_scan"]["available"] == false

    assert %{available: true} = Watchdog.scan_now()
    conn = conn(:get, "/api/health") |> Router.call(Router.init([]))
    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "ok"
    assert payload["watchdog"]["available"] == true
  end
end
