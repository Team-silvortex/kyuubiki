defmodule KyuubikiWeb.Orchestra.WorkflowCancellationCrashTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.TestSupport.{BeamChild, CancellableSolverAgent}

  @moduletag skip: match?({:win32, _}, :os.type())
  @moduletag timeout: 60_000

  for backend <- ["sqlite", "memory"],
      phase <- ["before_finish", "after_finish"],
      peer <- ["same", "replaced"] do
    test "#{backend}: SIGKILL #{phase} retains exact authority with #{peer} peer", _ctx do
      config = {root, _, _, _, _} = setup_crash(unquote(backend), unquote(phase), unquote(peer))

      with_child(config, "prepare", fn child ->
        describe(root, "original-process", 7)
        id = job_id(root)
        acknowledge(target(id, "in-flight", "original-process", 7), "requested")
        crash(child)
      end)

      refute_receive {:agent_request, _, _, _}, 20

      process =
        if unquote(peer) == "replaced", do: "replacement-process", else: "original-process"

      id = job_id(root)

      settled =
        with_child(config, "recover", fn child ->
          describe(root, process, 9)
          acknowledge(target(id, "untouched", process, 9), "requested")

          if unquote(phase) == "before_finish" do
            status = if unquote(peer) == "replaced", do: "target_not_observed", else: "requested"
            acknowledge(target(id, "in-flight", "original-process", 7), status)
          end

          line = await_line(child, "CANCELLATION_CRASH_RECOVERED ")
          assert await_exit(child) == 0
          line
        end)

      # Another fresh VM must not replay already settled cancellation or any solver.
      with_child(config, "verify", fn child ->
        assert await_line(child, "CANCELLATION_CRASH_RECOVERED ") == settled
        assert await_exit(child) == 0
      end)

      refute_receive {:agent_request, _, _, _}, 30
    end
  end

  for backend <- ["sqlite", "memory"] do
    test "#{backend}: repeated SIGKILL cannot reset or exceed the durable attempt budget" do
      config = {root, _, _, _, _} = setup_crash(unquote(backend), "exhaust_budget", "same")

      with_child(config, "prepare", fn child ->
        describe(root, "original-process", 7)
        acknowledge(target(job_id(root), "in-flight", "original-process", 7), "requested")
        crash(child)
      end)

      id = job_id(root)

      for attempt <- 2..3 do
        with_child(config, "retry_#{attempt}", fn child ->
          await_line(child, "CANCELLATION_RETRY_READY ")

          if attempt == 2 do
            describe(root, "original-process", 9)
            acknowledge(target(id, "untouched", "original-process", 9), "requested")
          end

          acknowledge(target(id, "in-flight", "original-process", 7), "requested")
          crash(child)
        end)
      end

      settled =
        with_child(config, "recover", fn child ->
          line = await_line(child, "CANCELLATION_CRASH_RECOVERED ")
          assert await_exit(child) == 0
          line
        end)

      with_child(config, "verify", fn child ->
        assert await_line(child, "CANCELLATION_CRASH_RECOVERED ") == settled
        assert await_exit(child) == 0
      end)

      refute_receive {:agent_request, _, _, _}, 30
    end
  end

  defp setup_crash(backend, phase, peer) do
    root =
      Path.join(
        System.tmp_dir!(),
        "kyuubiki-cancellation-crash-#{System.pid()}-#{System.unique_integer([:positive])}"
      )

    File.mkdir!(root)
    on_exit(fn -> File.rm_rf!(root) end)
    start_supervised!({CancellableSolverAgent, {self(), "crash-agent"}})
    assert_receive {:cancellable_agent_ready, "crash-agent", port}
    {root, backend, phase, peer, port}
  end

  defp crash(child) do
    line = await_line(child, "CANCELLATION_CRASH_READY ")
    pid = String.trim_leading(line, "CANCELLATION_CRASH_READY ")
    assert {:os_pid, os_pid} = Port.info(child, :os_pid)
    assert Integer.to_string(os_pid) == pid
    assert {_, 0} = System.cmd(System.find_executable("kill"), ["-KILL", pid])
    assert await_exit(child) != 0
  end

  defp describe(root, process, generation) do
    {handler, _} = request("describe_agent")
    id = job_id(root)

    active =
      for name <- ["in-flight", "untouched"] do
        %{
          "job_id" => id,
          "request_id" => name,
          "generation" =>
            if(name == "in-flight" and process == "original-process", do: 7, else: generation),
          "cancel_requested" => false
        }
      end

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "lifecycle" => %{"process_instance_id" => process},
           "solver_control" => %{
             "schema_version" => "kyuubiki.agent-solver-control/v1",
             "available" => true,
             "active" => active
           }
         }
       }}
    )
  end

  defp acknowledge(target, status) do
    {handler, request} = request("cancel_execution")
    assert request["params"] == target

    send(
      handler,
      {:reply,
       %{
         "ok" => true,
         "result" => %{
           "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
           "execution_target" => target,
           "status" => status,
           "cancel_registered" => status == "requested",
           "execution_terminal_confirmed" => false,
           "pending_cancellation_created" => false,
           "operator_package_cleanup_performed" => false,
           "automatic_replay_authorized" => false
         }
       }}
    )
  end

  defp request(method) do
    assert_receive {:agent_request, "crash-agent", handler, %{"method" => ^method} = request},
                   15_000

    {handler, request}
  end

  defp target(id, request, process, generation),
    do: %{
      "job_id" => id,
      "request_id" => request,
      "process_instance_id" => process,
      "generation" => generation
    }

  defp job_id(root),
    do:
      root
      |> Path.join("expected.json")
      |> File.read!()
      |> Jason.decode!()
      |> Map.fetch!("job_id")

  defp with_child({root, backend, phase, peer, port}, mode, callback) do
    child = launch(root, backend, mode, phase, peer, port)

    try do
      callback.(child)
    after
      case Port.info(child, :os_pid) do
        {:os_pid, pid} ->
          System.cmd(System.find_executable("kill"), ["-KILL", Integer.to_string(pid)])
          await_exit(child)

        nil ->
          :ok
      end
    end
  end

  defp launch(root, backend, mode, phase, peer, port) do
    paths = :code.get_path() |> Enum.flat_map(&["-pa", List.to_string(&1) |> Path.expand()])

    args =
      ["--erl", "+S 2:2 +SDcpu 1 +SDio 1"] ++
        paths ++
        [
          "-r",
          "test/support/workflow_commit_fixture.exs",
          "-r",
          "test/support/workflow_cancellation_crash_child.exs",
          "-e",
          "KyuubikiWeb.TestSupport.WorkflowCancellationCrashChild.run(#{inspect(root)}, #{inspect(mode)}, #{inspect(phase)}, #{inspect(peer)}, #{port})"
        ]

    env =
      BeamChild.port_env(%{
        "KYUUBIKI_STORAGE_BACKEND" => backend,
        "KYUUBIKI_DATA_DIR" => root,
        "SQLITE_DATABASE_PATH" => Path.join(root, "analysis.sqlite3"),
        "KYUUBIKI_ORCHESTRA_LEASE_NAME" => "cancellation-crash",
        "KYUUBIKI_ORCHESTRA_INSTANCE_ID" => "cancellation-crash-#{mode}",
        "KYUUBIKI_ORCHESTRA_LEASE_TTL_MS" => "1500",
        "KYUUBIKI_ORCHESTRA_LEASE_HEARTBEAT_MS" => "400",
        "KYUUBIKI_ORCHESTRA_LEASE_RETRY_MS" => "50",
        "KYUUBIKI_WATCHDOG_SCAN_INTERVAL_MS" => "60000"
      })

    Port.open({:spawn_executable, System.find_executable("elixir")}, [
      :binary,
      :exit_status,
      :stderr_to_stdout,
      {:line, 16_384},
      {:args, args},
      {:env, env},
      {:cd, File.cwd!()}
    ])
  end

  defp await_line(port, prefix),
    do: read_line(port, prefix, System.monotonic_time(:millisecond) + 20_000, "")

  defp read_line(port, prefix, deadline, output) do
    remaining = max(deadline - System.monotonic_time(:millisecond), 0)

    receive do
      {^port, {:data, {_kind, line}}} ->
        if String.starts_with?(line, prefix),
          do: line,
          else:
            read_line(port, prefix, deadline, String.slice(output <> line <> "\n", -8_000, 8_000))

      {^port, {:exit_status, status}} ->
        flunk("child exited #{status} before #{prefix}: #{output}")
    after
      remaining -> flunk("child did not reach #{prefix}: #{output}")
    end
  end

  defp await_exit(port) do
    receive do
      {^port, {:exit_status, status}} -> status
      {^port, {:data, _}} -> await_exit(port)
    after
      10_000 -> flunk("child did not exit")
    end
  end
end
