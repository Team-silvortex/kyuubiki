defmodule KyuubikiWeb.TestSupport.BeamChildTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.TestSupport.BeamChild

  for noise <- [false, true] do
    @tag timeout: 30_000
    test "child flags are isolated and application environment is preserved (diagnostics: #{noise})" do
      verify_child_environment(unquote(noise))
    end
  end

  defp verify_child_environment(noise) do
    # Pollute only this disposable VM, never the concurrently running test VM.
    code = ~S"""
    flags = ~w(ERL_FLAGS ERL_AFLAGS ERL_ZFLAGS ELIXIR_ERL_OPTIONS)
    Enum.each(flags, &System.put_env(&1, "+S 3:3 +SDcpu 2:2 +SDio 3"))
    inherited = Map.new(flags, &{&1, System.get_env(&1)})

    probe = ~S'''
    if System.get_env("KYUUBIKI_TEST_NOISE") == "true" do
      IO.puts("probe stdout diagnostic")
      IO.puts(:stderr, "probe stderr diagnostic")
    end
    counts = Enum.map([:schedulers_online, :dirty_cpu_schedulers_online, :dirty_io_schedulers],
      &:erlang.system_info/1)
    inherited = Enum.any?(~w(ERL_FLAGS ERL_AFLAGS ERL_ZFLAGS ELIXIR_ERL_OPTIONS),
      &(System.get_env(&1) != nil))
    IO.puts("BEAM_CHILD #{hd(System.argv())} #{Enum.join(counts, ":")}:#{inherited}:#{System.get_env("KYUUBIKI_TEST_MARKER")}")
    '''

    for {label, env} <- [
      {"INHERITED", [{~c"KYUUBIKI_TEST_MARKER", ~c"kept"}]},
      {"ISOLATED", KyuubikiWeb.TestSupport.BeamChild.port_env(%{"KYUUBIKI_TEST_MARKER" => "kept"})}
    ] do
      port = Port.open({:spawn_executable, System.find_executable("elixir")}, [
        :binary, :exit_status, :stderr_to_stdout,
        {:args, ["--erl", "+S 1:1 +SDcpu 1 +SDio 2", "-e", probe, "--", label]}, {:env, env}
      ])
      deadline = System.monotonic_time(:millisecond) + 5_000
      collect = fn collect, output ->
        remaining = max(deadline - System.monotonic_time(:millisecond), 0)
        receive do
          {^port, {:data, bytes}} -> collect.(collect, output <> bytes)
          {^port, {:exit_status, 0}} -> output
          {^port, {:exit_status, status}} -> raise "probe exited #{status}: #{output}"
        after
          remaining -> raise "probe timed out: #{output}"
        end
      end
      IO.write(collect.(collect, ""))
    end

    true = inherited == Map.new(flags, &{&1, System.get_env(&1)})
    IO.puts("BEAM_CHILD PARENT_UNCHANGED")
    """

    port =
      Port.open({:spawn_executable, System.find_executable("elixir")}, [
        :binary,
        :exit_status,
        :stderr_to_stdout,
        {:args,
         ["--erl", "+S 1:1 +SDcpu 1 +SDio 1", "-r", "test/support/beam_child.exs", "-e", code]},
        {:env, BeamChild.port_env(%{"KYUUBIKI_TEST_NOISE" => to_string(noise)})},
        {:cd, File.cwd!()}
      ])

    try do
      {output, status} = await_exit(port, "", System.monotonic_time(:millisecond) + 20_000)
      assert status == 0, output

      # Startup diagnostics are not protocol records; still reject missing,
      # duplicated or incorrect records and preserve the full output on failure.
      records =
        output
        |> String.split(~r/\r?\n/)
        |> Enum.filter(&String.starts_with?(&1, "BEAM_CHILD "))

      assert records == [
               "BEAM_CHILD INHERITED 3:2:3:true:kept",
               "BEAM_CHILD ISOLATED 1:1:2:false:kept",
               "BEAM_CHILD PARENT_UNCHANGED"
             ],
             output

      if noise do
        assert output =~ "probe stdout diagnostic"
        assert output =~ "probe stderr diagnostic"
      end
    after
      if Port.info(port), do: Port.close(port)
    end
  end

  defp await_exit(port, output, deadline) do
    remaining = max(deadline - System.monotonic_time(:millisecond), 0)

    receive do
      {^port, {:data, bytes}} -> await_exit(port, output <> bytes, deadline)
      {^port, {:exit_status, status}} -> {output, status}
    after
      remaining -> flunk("BEAM environment child timed out: #{output}")
    end
  end
end
