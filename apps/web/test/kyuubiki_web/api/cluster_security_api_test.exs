defmodule KyuubikiWeb.Api.ClusterSecurityApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  test "protects cluster registration routes with a dedicated cluster token" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "cluster-secret",
      cluster_api_token: "cluster-only-secret",
      protect_reads?: false
    )

    unauthorized_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-a",
          "host" => "127.0.0.1",
          "port" => 5001,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert unauthorized_conn.status == 401

    authorized_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-a",
          "host" => "127.0.0.1",
          "port" => 5001,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-a")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert authorized_conn.status == 201
    assert Jason.decode!(authorized_conn.resp_body)["agent"]["id"] == "remote-a"
  end

  test "falls back to control-plane token for cluster routes when no dedicated cluster token is configured" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-b",
          "host" => "127.0.0.1",
          "port" => 5002,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "shared-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-b")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["agent"]["id"] == "remote-b"
  end

  test "rejects stale cluster timestamps on protected cluster routes" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_timestamp_window_ms: 1_000,
      protect_reads?: false
    )

    stale_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-c",
          "host" => "127.0.0.1",
          "port" => 5003,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-c")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond) - 10_000)
      )
      |> Router.call(@opts)

    assert stale_conn.status == 401
    assert Jason.decode!(stale_conn.resp_body)["error"] == "stale_cluster_request"
  end

  test "rejects missing cluster timestamps on protected cluster routes" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-ts-missing",
          "host" => "127.0.0.1",
          "port" => 5006,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-ts-missing")
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "stale_cluster_request"
  end

  test "allows protected cluster registration when agent and cluster IDs are allowlisted" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_allowed_agent_ids: MapSet.new(["remote-d"]),
      cluster_allowed_cluster_ids: MapSet.new(["lan-a"]),
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-d",
          "host" => "127.0.0.1",
          "port" => 5004,
          "role" => "solver",
          "cluster_id" => "lan-a"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-d")
      |> put_req_header("x-kyuubiki-cluster-id", "lan-a")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 201
    assert Jason.decode!(conn.resp_body)["agent"]["id"] == "remote-d"
  end

  test "rejects protected cluster registration when agent ID is not allowlisted" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_allowed_agent_ids: MapSet.new(["remote-allowed"]),
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-denied",
          "host" => "127.0.0.1",
          "port" => 5005,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-denied")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "invalid_cluster_identity"
  end

  test "rejects protected cluster heartbeat when cluster ID is not allowlisted" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_allowed_agent_ids: MapSet.new(["remote-e"]),
      cluster_allowed_cluster_ids: MapSet.new(["lan-ok"]),
      protect_reads?: false
    )

    register_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-e",
          "host" => "127.0.0.1",
          "port" => 5006,
          "role" => "solver",
          "cluster_id" => "lan-ok"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-e")
      |> put_req_header("x-kyuubiki-cluster-id", "lan-ok")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert register_conn.status == 201

    heartbeat_conn =
      :post
      |> conn(
        "/api/v1/agents/remote-e/heartbeat",
        Jason.encode!(%{"cluster_id" => "lan-other"})
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-e")
      |> put_req_header("x-kyuubiki-cluster-id", "lan-other")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert heartbeat_conn.status == 401
    assert Jason.decode!(heartbeat_conn.resp_body)["error"] == "invalid_cluster_identity"
  end

  test "rejects protected cluster registration when fingerprint is required but missing" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_require_fingerprint?: true,
      protect_reads?: false
    )

    conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-fp-a",
          "host" => "127.0.0.1",
          "port" => 5007,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-fp-a")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert conn.status == 401
    assert Jason.decode!(conn.resp_body)["error"] == "invalid_cluster_identity"
  end

  test "rejects protected cluster heartbeat when fingerprint does not match the registered agent" do
    Application.put_env(:kyuubiki_web, KyuubikiWeb.Security,
      api_token: "shared-secret",
      cluster_api_token: "cluster-only-secret",
      cluster_require_fingerprint?: true,
      protect_reads?: false
    )

    register_conn =
      :post
      |> conn(
        "/api/v1/agents/register",
        Jason.encode!(%{
          "id" => "remote-fp-b",
          "host" => "127.0.0.1",
          "port" => 5008,
          "role" => "solver"
        })
      )
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-fp-b")
      |> put_req_header("x-kyuubiki-agent-fingerprint", "fp-good")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert register_conn.status == 201

    heartbeat_conn =
      :post
      |> conn("/api/v1/agents/remote-fp-b/heartbeat", Jason.encode!(%{}))
      |> put_req_header("content-type", "application/json")
      |> put_req_header("x-kyuubiki-token", "cluster-only-secret")
      |> put_req_header("x-kyuubiki-agent-id", "remote-fp-b")
      |> put_req_header("x-kyuubiki-agent-fingerprint", "fp-bad")
      |> put_req_header(
        "x-kyuubiki-cluster-ts",
        Integer.to_string(System.system_time(:millisecond))
      )
      |> Router.call(@opts)

    assert heartbeat_conn.status == 401
    assert Jason.decode!(heartbeat_conn.resp_body)["error"] == "invalid_cluster_identity"
  end
end
