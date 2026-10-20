defmodule KyuubikiWeb.Orchestra.WorkflowCompletionCrashTest do
  use ExUnit.Case, async: false

  # This is an OS-process crash test, not an application/coordinator restart test.
  @moduletag skip: match?({:win32, _}, :os.type())
  @moduletag timeout: 60_000

  for backend <- ["sqlite", "memory"], phase <- ["before_stage", "after_stage"] do
    test "#{backend}: SIGKILL #{phase} obeys the durable publication boundary" do
      root =
        Path.join(
          System.tmp_dir!(),
          "kyuubiki-completion-crash-#{System.pid()}-#{System.unique_integer([:positive])}"
        )

      File.mkdir_p!(root)
      on_exit(fn -> File.rm_rf!(root) end)

      with_child(root, unquote(backend), unquote(phase), fn first ->
        line = await_line(first, "CRASH_READY ")
        pid = String.trim_leading(line, "CRASH_READY ")
        assert Integer.to_string(Port.info(first, :os_pid) |> elem(1)) == pid
        assert {_, 0} = System.cmd(System.find_executable("kill"), ["-KILL", pid])
        assert await_exit(first) != 0
      end)

      expected_status = if unquote(phase) == "after_stage", do: "completed", else: "failed"

      for _ <- 1..2 do
        with_child(root, unquote(backend), "recover", fn restarted ->
          assert await_line(restarted, "CRASH_RECOVERED ") == "CRASH_RECOVERED #{expected_status}"
          assert await_exit(restarted) == 0
        end)
      end

      assert File.read!(Path.join(root, "solver-invocations")) == "solve\n"
    end
  end

  defp with_child(root, backend, mode, callback) do
    port = launch(root, backend, mode)

    try do
      callback.(port)
    after
      # Clean up while the owning port still exists, including failed assertions.
      case Port.info(port, :os_pid) do
        {:os_pid, pid} ->
          System.cmd(System.find_executable("kill"), ["-KILL", Integer.to_string(pid)])
          await_exit(port)

        nil ->
          :ok
      end
    end
  end

  defp launch(root, backend, mode) do
    paths = :code.get_path() |> Enum.flat_map(&["-pa", List.to_string(&1) |> Path.expand()])

    args =
      ["--erl", "+S 2:2 +SDcpu 1 +SDio 1"] ++
        paths ++
        [
          "-r",
          "test/support/storage_outage_workflow.exs",
          "-r",
          "test/support/workflow_completion_crash_child.exs",
          "-e",
          "KyuubikiWeb.TestSupport.WorkflowCompletionCrashChild.run(#{inspect(root)}, #{inspect(mode)})"
        ]

    env =
      %{
        "KYUUBIKI_STORAGE_BACKEND" => backend,
        "KYUUBIKI_DATA_DIR" => root,
        "SQLITE_DATABASE_PATH" => Path.join(root, "analysis.sqlite3"),
        "KYUUBIKI_ORCHESTRA_LEASE_NAME" => "completion-crash",
        "KYUUBIKI_ORCHESTRA_INSTANCE_ID" => "crash-#{mode}",
        "KYUUBIKI_ORCHESTRA_LEASE_TTL_MS" => "1500",
        "KYUUBIKI_ORCHESTRA_LEASE_HEARTBEAT_MS" => "400",
        "KYUUBIKI_ORCHESTRA_LEASE_RETRY_MS" => "50",
        "KYUUBIKI_WATCHDOG_SCAN_INTERVAL_MS" => "60000",
        "KYUUBIKI_WORKFLOW_TIMEOUT_MS" => "60000"
      }
      |> KyuubikiWeb.TestSupport.BeamChild.port_env()

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
