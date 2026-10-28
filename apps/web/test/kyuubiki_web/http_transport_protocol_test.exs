defmodule KyuubikiWeb.HttpTransportProtocolTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.HttpTransportSecurity

  @moduletag capture_log: true

  defmodule BodyProbe do
    def init(owner), do: owner

    def call(conn, owner) do
      {:ok, body, conn} = Plug.Conn.read_body(conn, length: 1024, read_timeout: 1000)
      send(owner, {:accepted_body, body})
      Plug.Conn.send_resp(conn, 200, body)
    end
  end

  setup do
    server =
      start_supervised!(
        {Bandit,
         [plug: {BodyProbe, self()}, ip: {127, 0, 0, 1}, port: 0] ++
           HttpTransportSecurity.server_options()}
      )

    {:ok, {_address, port}} = ThousandIsland.listener_info(server)
    %{port: port}
  end

  test "rejects bare LF in a chunk extension before accepting its body", %{port: port} do
    assert_rejected(port, "1;x=\ny\r\na\r\n0\r\n\r\n")
  end

  test "rejects bare CR in a chunk extension before accepting its body", %{port: port} do
    assert_rejected(port, "1;x=\ry\r\na\r\n0\r\n\r\n")
  end

  test "accepts ordinary chunks and valid token or quoted extensions", %{port: port} do
    for chunk <- ["1", "1;x=y", "1;x=\"y\"", "1 ;x=y"] do
      response = request(port, chunk <> "\r\na\r\n0\r\n\r\n")
      assert response =~ "HTTP/1.1 200 OK\r\n"
      assert String.ends_with?(response, "\r\n\r\na")
      assert_receive {:accepted_body, "a"}
    end
  end

  defp assert_rejected(port, body) do
    assert request(port, body) =~ "HTTP/1.1 400 Bad Request\r\n"
    refute_received {:accepted_body, _}
  end

  defp request(port, body) do
    {:ok, socket} =
      :gen_tcp.connect({127, 0, 0, 1}, port, [:binary, active: false, packet: :raw], 1000)

    try do
      :ok =
        :gen_tcp.send(socket, [
          "POST / HTTP/1.1\r\nHost: localhost\r\n",
          "Transfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
          body
        ])

      receive_response(socket, "", System.monotonic_time(:millisecond) + 5000)
    after
      :gen_tcp.close(socket)
    end
  end

  defp receive_response(socket, response, deadline) do
    remaining = deadline - System.monotonic_time(:millisecond)
    assert remaining > 0, "HTTP probe exceeded its response deadline"
    assert byte_size(response) <= 8192, "HTTP probe exceeded its response size limit"

    case :gen_tcp.recv(socket, 0, remaining) do
      {:ok, data} -> receive_response(socket, response <> data, deadline)
      {:error, :closed} -> response
      other -> flunk("HTTP probe failed: #{inspect(other)}")
    end
  end
end
