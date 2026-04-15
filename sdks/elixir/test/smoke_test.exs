defmodule KyuubikiSdk.SmokeTest do
  use ExUnit.Case, async: false

  alias KyuubikiSdk.AgentClient
  alias KyuubikiSdk.Session

  setup do
    parent = self()

    {:ok, listener} =
      :gen_tcp.listen(0, [:binary, packet: 0, active: false, reuseaddr: true])

    {:ok, port} = :inet.port(listener)

    acceptor =
      spawn_link(fn ->
        accept_loop(listener, parent)
      end)

    on_exit(fn ->
      Process.exit(acceptor, :kill)
      :gen_tcp.close(listener)
    end)

    {:ok, base_url: "http://127.0.0.1:#{port}"}
  end

  test "agent client runs a study and browses chunks", %{base_url: base_url} do
    session = Session.new(base_url: base_url)

    {:ok, outcome} =
      AgentClient.run_study(session, "truss_2d", %{"nodes" => [], "elements" => []}, timeout: 5_000)

    assert get_in(outcome, [:terminal, "job", "status"]) == "completed"
    assert is_map(outcome.result)

    {:ok, page} = AgentClient.browse_result_chunks(session, "job-smoke", "nodes", offset: 0, limit: 2)
    assert page["returned"] == 2
    assert page["total"] == 3
  end

  defp accept_loop(listener, parent) do
    {:ok, socket} = :gen_tcp.accept(listener)
    handle_socket(socket)
    send(parent, :handled_request)
    accept_loop(listener, parent)
  end

  defp handle_socket(socket) do
    {:ok, request} = :gen_tcp.recv(socket, 0)
    [request_line | _rest] = String.split(request, "\r\n")
    [method, path, _version] = String.split(request_line, " ")

    response =
      case {method, path} do
        {"POST", "/api/v1/fem/truss-2d/jobs"} ->
          json_response(202, %{"job" => %{"job_id" => "job-smoke", "status" => "queued"}})

        {"GET", "/api/v1/jobs/job-smoke"} ->
          json_response(200, %{"job" => %{"job_id" => "job-smoke", "status" => "completed", "progress" => 1.0}})

        {"GET", "/api/v1/results/job-smoke"} ->
          json_response(200, %{
            "job_id" => "job-smoke",
            "result" => %{
              "nodes" => [%{"index" => 0, "id" => "n0"}, %{"index" => 1, "id" => "n1"}, %{"index" => 2, "id" => "n2"}],
              "elements" => [%{"index" => 0, "id" => "e0"}],
              "max_displacement" => 1.0e-6,
              "max_stress" => 7.0e4
            }
          })

        {"GET", "/api/v1/results/job-smoke/chunks/nodes?offset=0&limit=2"} ->
          json_response(200, %{
            "job_id" => "job-smoke",
            "kind" => "nodes",
            "offset" => 0,
            "limit" => 2,
            "returned" => 2,
            "total" => 3,
            "items" => [%{"index" => 0, "id" => "n0"}, %{"index" => 1, "id" => "n1"}]
          })

        _ ->
          json_response(404, %{"error" => "not_found"})
      end

    :ok = :gen_tcp.send(socket, response)
    :gen_tcp.close(socket)
  end

  defp json_response(status, payload) do
    body = Jason.encode!(payload)
    reason =
      case status do
        200 -> "OK"
        202 -> "Accepted"
        404 -> "Not Found"
        _ -> "OK"
      end

    [
      "HTTP/1.1 ",
      Integer.to_string(status),
      " ",
      reason,
      "\r\nServer: kyuubiki-smoke\r\nDate: Tue, 15 Apr 2026 00:00:00 GMT\r\nContent-Type: application/json\r\nContent-Length: ",
      Integer.to_string(byte_size(body)),
      "\r\nConnection: close\r\n\r\n",
      body
    ]
    |> IO.iodata_to_binary()
  end
end
