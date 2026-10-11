defmodule KyuubikiWeb.Playground.TaskDigestRoutingTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Playground.{AgentClient, AgentPool, AgentPoolRouter}
  alias KyuubikiWeb.TaskDigestV2
  alias KyuubikiWeb.TestSupport.FakePlaygroundAgent

  setup do
    original = Application.get_env(:kyuubiki_web, AgentPool, [])

    on_exit(fn ->
      Application.put_env(:kyuubiki_web, AgentPool, original)
      AgentPool.reload()
    end)
  end

  defp task do
    {:ok, task} =
      OperatorTaskIR.build("transform.rank_material_candidates", %{"candidates" => %{}})

    {:ok, task} = OperatorTaskIR.seal_task_digest(task, TaskDigestV2.encoding())
    task
  end

  defp endpoint(id, capabilities \\ []) do
    %{
      id: id,
      host: "127.0.0.1",
      port: 59_999,
      capabilities: capabilities,
      methods: ["run_operator_task_ir"]
    }
  end

  defp configure(endpoints) do
    Application.put_env(:kyuubiki_web, AgentPool, endpoints: endpoints)
    AgentPool.reload()
  end

  test "encoding adds an immutable derived requirement without rewriting signed runtime hints" do
    task = task()
    original = task["runtime_hints"]["required_capabilities"]

    assert OperatorTaskIR.required_agent_capabilities(task) ==
             original ++ [TaskDigestV2.encoding()]

    opts = OperatorTaskIR.agent_routing_opts(task)
    assert opts[:required_capabilities] == original ++ [TaskDigestV2.encoding()]
    assert task["runtime_hints"]["required_capabilities"] == original
    assert :ok = KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary.validate_digest(task)

    for hints <- [[], [TaskDigestV2.encoding()], ["other", TaskDigestV2.encoding()]] do
      task = put_in(task, ["runtime_hints", "required_capabilities"], hints)

      assert Enum.count(
               OperatorTaskIR.required_agent_capabilities(task),
               &(&1 == TaskDigestV2.encoding())
             ) == 1
    end
  end

  test "unknown old and role-only peers are not typed digest fallbacks" do
    typed =
      endpoint("typed", [
        %{id: TaskDigestV2.encoding(), role: "task_digest", methods: [], tags: []}
      ])

    unknown = %{id: "unknown", host: "127.0.0.1", port: 59_998}
    old = endpoint("old")
    role_only = Map.put(old, :role, TaskDigestV2.encoding())

    for peers <- [[unknown, old, role_only], [typed, unknown, old, role_only]] do
      expected = if typed in peers, do: ["typed"], else: []

      assert Enum.map(
               AgentPoolRouter.route(peers, "run_operator_task_ir",
                 required_capabilities: [TaskDigestV2.encoding()]
               ),
               & &1.id
             ) == expected
    end

    assert AgentPoolRouter.route([unknown], "run_operator_task_ir",
             required_capabilities: ["solver_rpc"]
           ) != []

    no_method = Map.put(typed, :methods, [])

    assert AgentPoolRouter.route([no_method], "run_operator_task_ir",
             required_capabilities: [TaskDigestV2.encoding()]
           ) == []
  end

  test "legacy absent and explicit fixed15 encoding retain existing routing" do
    {:ok, legacy} = OperatorTaskIR.build("transform.rank_material_candidates", %{})
    {:ok, explicit} = OperatorTaskIR.seal_task_digest(legacy, "kyuubiki.task-digest/fixed15-v1")

    assert OperatorTaskIR.agent_routing_opts(legacy) ==
             OperatorTaskIR.agent_routing_opts(explicit)

    refute TaskDigestV2.encoding() in OperatorTaskIR.required_agent_capabilities(legacy)

    assert OperatorTaskIR.enforce_digest_routing(
             "run_operator_task_ir",
             %{"task_ir" => legacy},
             []
           ) == []
  end

  test "generic routing leaves malformed encoding to admission without crashing" do
    for integrity <- [nil, false, "invalid", [], %{}] do
      assert OperatorTaskIR.enforce_digest_routing(
               "run_operator_task_ir",
               %{"task_ir" => %{"integrity" => integrity}},
               []
             ) == []
    end

    malformed = put_in(task(), ["runtime_hints"], "invalid")

    assert OperatorTaskIR.enforce_digest_routing(
             "run_operator_task_ir",
             %{"task_ir" => malformed},
             []
           )[:required_capabilities] == [TaskDigestV2.encoding()]
  end

  test "generic task RPC cannot override the derived capability with empty caller requirements" do
    task = task()
    configure([%{id: "unknown", host: "127.0.0.1", port: 59_998}])

    for opts <- [[], [required_capabilities: []], [required_capabilities: ["caller"]]] do
      assert {:error, {:no_matching_agent, reason}} =
               AgentClient.request_with_agent(
                 "run_operator_task_ir",
                 %{"task_ir" => task},
                 fn _ -> :ok end,
                 opts
               )

      assert TaskDigestV2.encoding() in reason.required_capabilities
      assert "workflow_transform_runtime" in reason.required_capabilities
    end
  end

  test "dedicated client blocks typed tasks before any request when only an old peer exists" do
    {:ok, _} =
      FakePlaygroundAgent.start_link({:capture, self(), [%{"ok" => true, "result" => %{}}]})

    assert_receive {:fake_agent_ready, port}
    old = endpoint("old", ["workflow_transform_runtime"]) |> Map.put(:port, port)
    configure([old])

    for mode <- [:preflight, :execute] do
      assert {:error, {:no_matching_agent, reason}} =
               AgentClient.run_operator_task_ir(task(),
                 mode: mode,
                 required_capabilities: [],
                 task_digest_encoding: "kyuubiki.task-digest/fixed15-v1"
               )

      assert TaskDigestV2.encoding() in reason.required_capabilities
    end

    refute_receive {:fake_agent_request, _}, 50
  end

  test "a mixed pool sends the original typed task only to the explicit capable peer" do
    {:ok, _} =
      FakePlaygroundAgent.start_link(
        {:capture, self(), [%{"ok" => true, "result" => %{"peer" => "typed"}}]}
      )

    assert_receive {:fake_agent_ready, port}

    modern =
      endpoint("modern", ["workflow_transform_runtime", TaskDigestV2.encoding()])
      |> Map.put(:port, port)

    configure([endpoint("old", ["workflow_transform_runtime"]), modern])
    task = task()
    assert {:ok, %{"peer" => "typed"}} = AgentClient.run_operator_task_ir(task)
    assert_receive {:fake_agent_request, request}
    assert request["params"]["task_ir"] == task
  end

  test "unavailable typed peer does not transport-failover to an old peer" do
    {:ok, _} =
      FakePlaygroundAgent.start_link({:capture, self(), [%{"ok" => true, "result" => %{}}]})

    assert_receive {:fake_agent_ready, port}
    # Reserve then close an ephemeral listener so the capable endpoint cannot accept a request.
    {:ok, listener} = :gen_tcp.listen(0, [:binary, active: false, ip: {127, 0, 0, 1}])
    {:ok, {_, closed_port}} = :inet.sockname(listener)
    :ok = :gen_tcp.close(listener)

    modern =
      endpoint("modern", ["workflow_transform_runtime", TaskDigestV2.encoding()])
      |> Map.put(:port, closed_port)

    old = endpoint("old", ["workflow_transform_runtime"]) |> Map.put(:port, port)
    configure([modern, old])
    assert {:error, {:all_agents_failed, [_]}} = AgentClient.run_operator_task_ir(task())
    refute_receive {:fake_agent_request, _}, 50
  end
end
