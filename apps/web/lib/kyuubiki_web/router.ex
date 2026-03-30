defmodule KyuubikiWeb.Router do
  @moduledoc false

  use Plug.Router

  alias KyuubikiWeb.Playground

  plug(Plug.Logger)
  plug(Plug.Parsers, parsers: [:json], pass: ["application/json"], json_decoder: Jason)

  plug(:match)
  plug(:dispatch)

  get "/" do
    respond_json(conn, 200, %{
      "service" => "kyuubiki-orchestrator",
      "ui" => "http://127.0.0.1:3000",
      "status" => "ok"
    })
  end

  get "/api/health" do
    respond_json(conn, 200, %{
      "service" => "kyuubiki-orchestrator",
      "status" => "ok",
      "transport" => %{
        "http" => 4000,
        "solver_agent_tcp" => 5001
      }
    })
  end

  post "/api/v1/fem/axial-bar/jobs" do
    case Playground.run(conn.body_params) do
      {:ok, payload} ->
        respond_json(conn, 200, payload)

      {:error, reason} ->
        respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  post "/api/playground/run" do
    case Playground.run(conn.body_params) do
      {:ok, payload} -> respond_json(conn, 200, payload)
      {:error, reason} -> respond_json(conn, 422, %{"error" => inspect(reason)})
    end
  end

  match _ do
    Plug.Conn.send_resp(conn, 404, "not found")
  end

  defp respond_json(conn, status, payload) do
    conn
    |> Plug.Conn.put_resp_content_type("application/json")
    |> Plug.Conn.send_resp(status, Jason.encode!(payload))
  end
end
