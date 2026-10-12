defmodule KyuubikiWeb.TestSupport.WorkflowDiagnosticAgent do
  @moduledoc """
  Scripted solver frames with request-bound diagnostic receipts, not a physics oracle.
  Set KYUUBIKI_WORKFLOW_DIAGNOSTIC_AGENT_PORT to execute diagnostics on a real local
  Rust Agent instead. Solver results remain fixtures in both modes.
  """

  import ExUnit.Assertions

  alias KyuubikiWeb.Orchestra.{OperatorTaskExecutionSummary, OperatorTaskReadiness}
  alias KyuubikiWeb.{WorkflowNativeInvocation, WorkflowOperatorRuntime}

  def start!(sessions) do
    owner = self()

    ExUnit.Callbacks.start_supervised!(%{
      id: make_ref(),
      start: {Task, :start_link, [fn -> serve(owner, sessions) end]},
      restart: :temporary
    })
  end

  def solver(method, frames), do: {method, fn _request -> frames end}

  def diagnostic(node_id, operator_id, mutate_receipt \\ &Function.identity/1) do
    {"run_operator_task_ir",
     fn request ->
       task = request["params"]["task_ir"]
       assert request["params"]["mode"] == "execute"
       assert task["node"]["id"] == node_id
       assert task["operator"]["id"] == operator_id
       assert task["operator"]["kind"] == "extract"
       assert task["runtime_hints"]["execution_mode"] == "agent_native"
       assert task["execution_program"]["package_ref"] == nil
       assert :ok = OperatorTaskExecutionSummary.validate_digest(task)
       context = task["orchestration_context"]
       assert is_binary(context["job_id"])
       assert task["task_id"] == "#{context["job_id"]}:#{node_id}:#{operator_id}"

       assert context ==
                WorkflowNativeInvocation.bind_context(
                  context,
                  task["input_artifact"],
                  task["config"]
                )

       frames = diagnostic_frames(request, task)

       Enum.map(frames, fn
         %{"ok" => true, "result" => receipt} = frame ->
           Map.put(frame, "result", mutate_receipt.(receipt))

         frame ->
           frame
       end)
     end}
  end

  def assert_complete(pid) do
    assert_receive {:workflow_diagnostic_agent_complete, ^pid, requests}, 1_000
    requests
  end

  defp serve(owner, sessions) do
    {:ok, listener} =
      :gen_tcp.listen(0, [:binary, packet: 4, active: false, ip: {127, 0, 0, 1}])

    try do
      {:ok, port} = :inet.port(listener)
      send(owner, {:fake_agent_ready, port})

      requests =
        Enum.map(sessions, fn {method, reply} ->
          {:ok, socket} = :gen_tcp.accept(listener, 5_000)

          try do
            {:ok, payload} = :gen_tcp.recv(socket, 0, 1_000)
            request = Jason.decode!(payload)
            assert request["method"] == method

            for frame <- reply.(request) do
              frame = Map.merge(frame, Map.take(request, ["id", "rpc_version"]))
              :ok = :gen_tcp.send(socket, Jason.encode!(frame))
            end

            request
          after
            :gen_tcp.close(socket)
          end
        end)

      send(owner, {:workflow_diagnostic_agent_complete, self(), requests})
    after
      :gen_tcp.close(listener)
    end
  end

  defp diagnostic_frames(request, task) do
    case System.get_env("KYUUBIKI_WORKFLOW_DIAGNOSTIC_AGENT_PORT") do
      nil ->
        # The reference adapter tests orchestration only, never native numerical correctness.
        {:ok, result} =
          WorkflowOperatorRuntime.run_extract_operator(
            task["operator"]["id"],
            task["input_artifact"],
            task["config"]
          )

        {:ok, summary} = OperatorTaskExecutionSummary.build(task)

        receipt =
          summary
          |> Map.take(~w(task_id task_digest operator_id program_id))
          |> Map.merge(%{
            "operator_task_ir_status" => "executed",
            "execution_readiness" => OperatorTaskReadiness.local_executed(),
            "result" => result
          })

        [%{"ok" => true, "result" => receipt}]

      port ->
        {:ok, socket} =
          :gen_tcp.connect(
            {127, 0, 0, 1},
            String.to_integer(port),
            [:binary, packet: 4, active: false],
            1_000
          )

        try do
          :ok = :gen_tcp.send(socket, Jason.encode!(request))
          receive_frames(socket, request, 128)
        after
          :gen_tcp.close(socket)
        end
    end
  end

  defp receive_frames(socket, request, remaining) when remaining > 0 do
    {:ok, payload} = :gen_tcp.recv(socket, 0, 2_000)
    frame = Jason.decode!(payload)
    assert frame["id"] == request["id"]
    assert frame["rpc_version"] == request["rpc_version"]

    if is_boolean(frame["ok"]),
      do: [frame],
      else: [frame | receive_frames(socket, request, remaining - 1)]
  end

  defp receive_frames(_socket, _request, 0), do: flunk("diagnostic Agent frame budget exceeded")
end
