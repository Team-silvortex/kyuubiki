defmodule KyuubikiWeb.TestSupport.WorkflowCancellationCrashChild do
  @moduledoc false

  import ExUnit.Assertions
  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store, as: Jobs
  alias KyuubikiWeb.Orchestra.WorkflowCancellationDelivery, as: Delivery
  alias KyuubikiWeb.Orchestra.WorkflowCancellationIntent, as: Intent
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @key "_workflow_recovery"
  @registry KyuubikiWeb.Orchestra.WorkflowRunnerRegistry

  def run(root, mode, phase, peer, port) do
    spawn(fn ->
      Process.sleep(35_000)
      System.halt(91)
    end)

    Mix.start()
    Mix.env(:test)

    for path <- ["config/config.exs", "config/runtime.exs"] do
      Application.put_all_env(Config.Reader.read!(path, env: :test))
    end

    endpoint = %{id: "crash-agent", host: "127.0.0.1", port: port, agent_session_id: "session"}
    Application.put_env(:kyuubiki_web, AgentPool, discovery: :static, endpoints: [endpoint])

    Application.put_env(
      :kyuubiki_web,
      :operator_dispatch_journal_test_root,
      Path.join(root, "journal")
    )

    assert {:ok, _} = Application.ensure_all_started(:kyuubiki_web)

    case mode do
      "prepare" -> prepare(root, phase, endpoint)
      "retry_2" -> retry_crash(root, 2)
      "retry_3" -> retry_crash(root, 3)
      _ -> recover(root, phase, peer)
    end
  end

  def observe({owner, id, phase} = config, {:in, {:"$gen_call", _, message}}, _) do
    pause? =
      case {phase, message} do
        {phase,
         {:cancellation_intent, ^id, {:sweep, _, {:finish, %{id: {_, "in-flight"}}, "requested"}}}}
        when phase in ["before_finish", "exhaust_budget"] ->
          true

        {"after_finish",
         {:cancellation_intent, ^id, {:sweep, _, {:reserve, _, {_, "untouched"}}}}} ->
          true

        _ ->
          false
      end

    if pause? do
      send(owner, :cancellation_crash_paused)
      # The parent kills the entire VM; no shutdown callback may finish the receipt.
      receive do
        :unexpected_resume -> raise "crash probe must not resume"
      after
        30_000 -> System.halt(92)
      end
    end

    config
  end

  def observe(config, _, _), do: config

  defp prepare(root, phase, endpoint) do
    wait(fn -> Coordinator.snapshot()["lease"]["status"] == "owner" end)
    wait(fn -> :sys.get_state(Coordinator).recovery_runs > 0 and idle?() end)
    f = Fixture.terminal_job(:failed)

    snapshot = %{
      requests:
        Enum.map(["in-flight", "untouched"], &%{endpoint: endpoint, request_id: &1, job_id: f.id}),
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    assert {:ok, recovery} = Intent.attach(f.runtime[@key], f.job, snapshot)

    assert :ok =
             AnalysisResultStore.compare_and_swap(
               f.id,
               f.runtime,
               Map.put(f.runtime, @key, recovery)
             )

    File.write!(
      Path.join(root, "expected.json"),
      Jason.encode!(%{
        "job_id" => f.id,
        "generation" => recovery["generation"],
        "attempt" => recovery["attempt"],
        "intent_id" => recovery[Intent.key()]["intent_id"]
      }),
      [:sync]
    )

    :ok = :sys.install(Coordinator, {&__MODULE__.observe/3, {self(), f.id, phase}})
    assert {:ok, _} = Delivery.kick([f.id])
    await_crash(f.id, phase, 1)
  end

  defp retry_crash(root, attempt) do
    expected = root |> Path.join("expected.json") |> File.read!() |> Jason.decode!()
    id = expected["job_id"]
    :ok = :sys.install(Coordinator, {&__MODULE__.observe/3, {self(), id, "exhaust_budget"}})
    IO.puts("CANCELLATION_RETRY_READY #{System.pid()}")
    await_crash(id, "exhaust_budget", attempt)
  end

  defp await_crash(id, phase, attempt) do
    receive do
      :cancellation_crash_paused ->
        assert {:ok, runtime} = AnalysisResultStore.get(id)
        assert [first, untouched] = runtime[@key][Intent.key()]["entries"]
        assert first["attempts"] == attempt

        assert first["status"] ==
                 if(phase == "after_finish", do: "requested", else: "dispatching")

        assert first["execution_target"] == target(id, "in-flight", "original-process", 7)
        assert untouched["attempts"] == if(attempt == 1, do: 0, else: 1)
        assert untouched["status"] == if(attempt == 1, do: "pending", else: "requested")

        expected_target =
          if attempt == 1, do: nil, else: target(id, "untouched", "original-process", 9)

        assert untouched["execution_target"] == expected_target
        IO.puts("CANCELLATION_CRASH_READY #{System.pid()}")
        Process.sleep(:infinity)
    after
      15_000 -> raise "cancellation did not reach the crash boundary"
    end
  end

  defp recover(root, phase, peer) do
    expected = root |> Path.join("expected.json") |> File.read!() |> Jason.decode!()
    id = expected["job_id"]
    wait(fn -> Coordinator.snapshot()["lease"]["status"] == "owner" end)

    wait(fn ->
      with {:ok, runtime} <- AnalysisResultStore.get(id),
           false <- Intent.pending?(runtime[@key][Intent.key()]),
           true <- :sys.get_state(Coordinator).recovery_runs > 0 do
        idle?()
      else
        _ -> false
      end
    end)

    assert {:ok, job} = Jobs.get(id)
    assert job.status == :failed
    assert {:ok, runtime} = AnalysisResultStore.get(id)
    recovery = runtime[@key]
    assert :ok = Intent.verify(recovery, job)
    assert recovery["state"] == "failed"
    assert recovery["generation"] == expected["generation"]
    assert recovery["attempt"] == expected["attempt"]
    assert recovery[Intent.key()]["intent_id"] == expected["intent_id"]
    assert [first, untouched] = recovery[Intent.key()]["entries"]

    {expected_status, attempts, reason} =
      case {phase, peer} do
        {"exhaust_budget", _} -> {"exhausted", 3, "retry_budget_exhausted"}
        {"before_finish", "replaced"} -> {"target_not_observed", 2, nil}
        {"before_finish", _} -> {"requested", 2, nil}
        {"after_finish", _} -> {"requested", 1, nil}
      end

    assert first["status"] == expected_status
    assert first["attempts"] == attempts
    assert first["reason"] == reason
    assert first["execution_target"] == target(id, "in-flight", "original-process", 7)
    assert untouched["status"] == "requested"
    assert untouched["attempts"] == 1
    process = if peer == "replaced", do: "replacement-process", else: "original-process"
    assert untouched["execution_target"] == target(id, "untouched", process, 9)

    assert Intent.summary(recovery)["status"] ==
             if(phase == "exhaust_budget", do: "unresolved", else: "acknowledged")

    refute Intent.summary(recovery)["execution_terminal_confirmed"]
    assert :ok = Application.stop(:kyuubiki_web)
    IO.puts("CANCELLATION_CRASH_RECOVERED " <> Jason.encode!(recovery[Intent.key()]))
  end

  defp target(id, request, process, generation),
    do: %{
      "job_id" => id,
      "request_id" => request,
      "process_instance_id" => process,
      "generation" => generation
    }

  defp idle?, do: Registry.lookup(@registry, {Delivery, :sweep}) == []
  defp wait(check, remaining \\ 1_500)
  defp wait(_, 0), do: raise("cancellation recovery timed out")

  defp wait(check, remaining) do
    unless check.() do
      Process.sleep(10)
      wait(check, remaining - 1)
    end
  end
end
