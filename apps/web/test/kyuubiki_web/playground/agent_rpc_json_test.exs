defmodule KyuubikiWeb.Playground.AgentRpcJsonTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentRpcTransport, as: Transport
  alias KyuubikiWeb.Playground.AgentRpcJson

  test "duplicate envelope keys are rejected even when their values agree" do
    for body <- [
          ~S("ok":true,"ok":true,"result":{}),
          ~S("ok":true,"result":{},"result":{"value":42}),
          ~S("ok":true,"result":{},"error":null,"error":{"code":"failed","message":"SECRET"}),
          ~S("ok":true,"result":{},"id":"query"),
          ~S("ok":true,"result":{},"rpc_version":1)
        ] do
      assert_duplicate(request(frame(body)))
    end
  end

  test "duplicate decoded keys cannot hide in nested arrays or Unicode escapes" do
    for body <- [
          ~S("ok":true,"result":{"items":[{"value":1,"value":2}]}),
          ~S("ok":true,"result":{"value":1,"val\u0075e":2}),
          ~S("ok":true,"result":{"a/b":1,"a\/b":2}),
          ~S("ok":true,"result":{},"extension":{"SECRET":1,"SECRET":2})
        ] do
      assert_duplicate(request(frame(body)))
    end
  end

  test "error codes and failure detail duplicates cannot become a normal RPC failure" do
    for body <- [
          ~S("ok":false,"error":{"code":"agent_at_capacity","code":"failed","message":"SECRET"}),
          ~S("ok":false,"error":{"code":"failed","message":"SECRET","details":{"operator_task_failure_receipt":{"task_id":"first","task_id":"second"}}})
        ] do
      assert_duplicate(request(frame(body), method: "run_operator_task_ir"))
    end
  end

  test "duplicate progress data is rejected before callback delivery" do
    for body <- [
          ~S("event":"progress","event":"heartbeat","progress":{"stage":"solving"}),
          ~S("event":"heartbeat","progress":{"stage":"solving","stage":"completed"}),
          ~S("event":"progress","progress":{"metadata":[{"count":1,"count":2}]})
        ] do
      result = request(frame(body))
      refute_received {:progress, _}
      assert_duplicate(result)
    end
  end

  test "invalid JSON diagnostics do not retain or echo the original frame" do
    raw =
      ~S({"rpc_version":1,"id":"query","ok":true,"result":{"SECRET_CREDENTIAL":"SECRET_VALUE"},)

    assert request(raw) ==
             {:error, {:agent_transport_failure, :protocol, {:invalid_response, :invalid_json}}}
  end

  test "unique keys keep their scope, decoded spelling and opaque string contents" do
    raw =
      ~S({"items":[{"value":1},{"value":2}],"val\u0075e":3,"a\/b":4,"__struct__":"opaque","\u00e9":5,"e\u0301":6,"text":"{\"value\":1,\"value\":2}","empty":[null,{},[]]})

    expected = Jason.decode!(raw)
    assert map_size(expected) == 8
    assert request(frame(~s("ok":true,"result":#{raw}))) == {:ok, expected}
  end

  test "numeric payloads preserve native integer precision and exact float bits" do
    for raw <- [
          "-0.0",
          "5e-324",
          "-5e-324",
          "2.2250738585072014e-308",
          "1.7976931348623157e308",
          "1.0000000000000002",
          "9007199254740993",
          "18446744073709551616",
          "-18446744073709551617"
        ] do
      expected = Jason.decode!(raw)
      assert {:ok, actual} = request(frame(~s("ok":true,"result":[#{raw}])))
      assert [number] = actual
      assert number === expected

      if is_float(number), do: assert(<<number::float-64>> == <<expected::float-64>>)
    end
  end

  test "wide payloads retain array order and independently scoped object keys" do
    payload = %{
      "values" => Enum.to_list(1..20_000),
      "objects" => Enum.map(1..2_000, &%{"index" => &1, "value" => [&1, nil]}),
      "fields" => Map.new(1..2_000, &{"field-#{&1}", &1})
    }

    assert {:ok, ^payload} = request(frame(~s("ok":true,"result":#{Jason.encode!(payload)})))
  end

  test "syntax errors and escaped duplicate keys expose only fixed diagnostic codes" do
    for raw <- ["{", ~S({"SECRET":1,}), ~S({"SECRET":"\uD800"}), "[1e999]"] do
      assert AgentRpcJson.decode(raw) == {:error, :invalid_json}
    end

    for raw <- [
          ~S({"\uD83D\uDE00":1,"\ud83d\ude00":2}),
          ~S({"\"":1,"\u0022":2})
        ] do
      assert AgentRpcJson.decode(raw) == {:error, :duplicate_json_key}
    end
  end

  defp frame(body), do: ~s({"rpc_version":1,"id":"query",#{body}})

  defp assert_duplicate(result) do
    assert result ==
             {:error,
              {:agent_transport_failure, :protocol, {:invalid_response, :duplicate_json_key}}}

    refute inspect(result) =~ "SECRET"
  end

  defp request(raw, opts \\ []) do
    {:ok, listener} = :gen_tcp.listen(0, [:binary, packet: 4, active: false, ip: {127, 0, 0, 1}])
    on_exit(fn -> :gen_tcp.close(listener) end)
    {:ok, port} = :inet.port(listener)

    agent =
      Task.async(fn ->
        {:ok, socket} = :gen_tcp.accept(listener, 2_000)

        try do
          {:ok, _query} = :gen_tcp.recv(socket, 0, 2_000)
          :gen_tcp.send(socket, raw)
        after
          :gen_tcp.close(socket)
        end
      end)

    parent = self()

    query = %{
      "rpc_version" => 1,
      "id" => "query",
      "method" => Keyword.get(opts, :method, "ping"),
      "params" => %{}
    }

    result =
      Transport.request(
        %{host: "127.0.0.1", port: port},
        "query",
        query,
        fn progress -> send(parent, {:progress, progress}) end,
        opts
      )

    assert Task.await(agent, 2_000) == :ok
    result
  end
end
