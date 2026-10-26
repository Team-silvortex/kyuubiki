defmodule KyuubikiWeb.Playground.AgentRpcEnvelopeTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.AgentRpcTransport, as: Transport
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  test "success cannot discard an error or invent an absent result" do
    for frame <- [
          %{
            "ok" => true,
            "result" => %{"value" => 42},
            "error" => %{"code" => "failed", "message" => "SECRET failure"}
          },
          %{"ok" => true, "result" => %{}, "error" => false},
          %{"ok" => true, "result" => nil},
          %{"ok" => true}
        ] do
      assert_protocol_failure(request([frame]))
    end
  end

  test "failure cannot discard a result or accept an invalid error" do
    valid = %{"ok" => false, "error" => %{"code" => "failed", "message" => "worker failed"}}

    for frame <- [
          Map.put(valid, "result", %{"value" => 42}),
          Map.put(valid, "result", false),
          Map.put(valid, "error", %{}),
          Map.put(valid, "error", nil),
          Map.put(valid, "error", []),
          put_in(valid, ~w(error code), 1),
          put_in(valid, ~w(error code), " \t"),
          put_in(valid, ~w(error message), "\n"),
          put_in(valid, ~w(error message), false)
        ] do
      assert_protocol_failure(request([frame]))
    end
  end

  test "terminal frames cannot carry event or progress fields" do
    base = %{"ok" => true, "result" => %{"value" => 42}}

    for {key, value} <- [
          {"event", "unknown"},
          {"event", nil},
          {"progress", %{}},
          {"progress", nil}
        ] do
      assert_protocol_failure(request([Map.put(base, key, value)]))
    end
  end

  test "mixed progress frames fail before any callback" do
    parent = self()

    for event <- ["progress", "heartbeat"],
        {key, value} <- [{"ok", true}, {"ok", nil}, {"result", nil}, {"error", nil}] do
      frame = %{
        "event" => event,
        "progress" => %{"stage" => "solving", "progress" => 0.5},
        key => value
      }

      result = request([frame], fn progress -> send(parent, {:unexpected_progress, progress}) end)
      refute_received {:unexpected_progress, _}
      assert_protocol_failure(result)
    end
  end

  test "valid terminal payloads and optional null or absent counterpart fields remain supported" do
    for payload <- [
          false,
          0,
          1.25,
          "value",
          [],
          %{},
          %{"nested" => [nil, %{"extension" => true}]}
        ],
        null_error <- [false, true] do
      frame = %{"ok" => true, "result" => payload, "trace_extension" => "opaque"}
      frame = if null_error, do: Map.put(frame, "error", nil), else: frame
      assert request([frame]) == {:ok, payload}
    end

    for null_result <- [false, true] do
      frame = %{
        "ok" => false,
        "error" => %{
          "code" => "worker_failed",
          "message" => "worker failed",
          "details" => %{"extension" => [nil]}
        }
      }

      frame = if null_result, do: Map.put(frame, "result", nil), else: frame
      assert request([frame]) == {:error, {:rpc_error, "worker_failed", "worker failed"}}
    end
  end

  test "valid progress and heartbeat arrive in order before the terminal result" do
    parent = self()
    progress = %{"stage" => "solving", "progress" => 0.25}
    heartbeat = %{"stage" => "solving", "progress" => 0.5}

    frames = [
      %{"event" => "progress", "progress" => progress},
      %{"event" => "heartbeat", "progress" => heartbeat},
      %{"ok" => true, "result" => %{"value" => 42}, "error" => nil}
    ]

    assert request(frames, fn value -> send(parent, {:progress, value}) end) ==
             {:ok, %{"value" => 42}}

    for expected <- [progress, heartbeat] do
      assert_received {:progress, actual}
      assert actual == expected
    end

    refute_received {:progress, _}
  end

  test "structured TaskIR failures keep their details after envelope validation" do
    details = %{"task_id" => "task", "reason_code" => "failed", "extension" => [nil, 1.25]}

    frame = %{
      "ok" => false,
      "result" => nil,
      "error" => %{
        "code" => "failed",
        "message" => "worker failed",
        "details" => %{"operator_task_failure_receipt" => details}
      }
    }

    assert request([frame], fn _ -> :ok end, "run_operator_task_ir") ==
             {:error, {:operator_task_rpc_error, "failed", "worker failed", details}}

    assert_protocol_failure(
      request([Map.put(frame, "result", %{})], fn _ -> :ok end, "run_operator_task_ir")
    )
  end

  defp request(frames, callback \\ fn _ -> :ok end, method \\ "ping") do
    {:ok, _pid} = FakePlaygroundAgent.start_link(frames)
    assert_receive {:fake_agent_ready, port}, 1_000
    endpoint = %{host: "127.0.0.1", port: port}
    query = %{"rpc_version" => 1, "id" => "query", "method" => method, "params" => %{}}
    Transport.request(endpoint, "query", query, callback, [])
  end

  defp assert_protocol_failure(result) do
    assert {:error, {:agent_transport_failure, :protocol, {:invalid_response, reason}}} = result
    assert is_atom(reason)
    refute inspect(result) =~ "SECRET"
  end
end
