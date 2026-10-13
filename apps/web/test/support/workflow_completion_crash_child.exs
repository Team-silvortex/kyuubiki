defmodule KyuubikiWeb.TestSupport.WorkflowCompletionCrashChild do
  @moduledoc false

  alias KyuubikiWeb.{AnalysisResultStore, WorkflowOperatorRuntime}
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.TestSupport.StorageOutageWorkflow, as: Workflow

  @key "_workflow_recovery"

  def run(root, mode) do
    Mix.start()
    Mix.env(:test)

    for path <- ["config/config.exs", "config/runtime.exs"] do
      Application.put_all_env(Config.Reader.read!(path, env: :test))
    end

    Application.put_env(
      :kyuubiki_web,
      :operator_dispatch_journal_test_root,
      Path.join(root, "journal")
    )

    Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, solve_runtime_client: __MODULE__)
    :persistent_term.put({__MODULE__, :root}, root)
    {:ok, _} = Application.ensure_all_started(:kyuubiki_web)

    if mode == "recover", do: recover(root), else: prepare(root, mode)
  end

  def request("solve_bar_1d", _payload, _progress, _opts) do
    root = :persistent_term.get({__MODULE__, :root})
    File.write!(Path.join(root, "solver-invocations"), "solve\n", [:append, :sync])
    {:ok, %{"computed" => true, "precision" => [1.0, 1.234567890123456e-12]}}
  end

  def observe(owner, {:in, {:"$gen_call", _from, {phase, id, claim, result}}}, _name)
      when phase in [:stage_result, :commit_result] do
    if phase == :persistent_term.get({__MODULE__, :pause_phase}) do
      send(owner, {:publication_paused, id, claim, result})
      # Only the parent test's SIGKILL should end this VM at the selected boundary.
      receive do
        :unexpected_resume -> raise "crash probe must not resume"
      after
        30_000 -> System.halt(91)
      end
    end

    owner
  end

  def observe(owner, _event, _name), do: owner

  defp prepare(root, mode) do
    wait(fn -> Coordinator.snapshot()["lease"]["status"] == "owner" end)
    phase = if mode == "before_stage", do: :stage_result, else: :commit_result
    :persistent_term.put({__MODULE__, :pause_phase}, phase)
    :ok = :sys.install(Coordinator, {&__MODULE__.observe/3, self()})
    {:ok, submitted} = Workflow.submit("checkpoint_required")
    id = submitted["job"]["job_id"]

    receive do
      {:publication_paused, ^id, claim, result} ->
        {:ok, runtime} = AnalysisResultStore.get(id)
        pending? = Map.has_key?(runtime[@key], "pending_completion")
        true = pending? == (phase == :commit_result)
        {:ok, job} = Store.get(id)
        false = job.status == :completed

        File.write!(
          Path.join(root, "expected.json"),
          Jason.encode!(%{
            "job_id" => id,
            "claim" => claim,
            "result" => result,
            "pending" => pending?
          }),
          [:sync]
        )

        IO.puts("CRASH_READY #{System.pid()}")
        Process.sleep(:infinity)
    after
      10_000 -> raise "workflow did not reach publication boundary"
    end
  end

  defp recover(root) do
    expected = root |> Path.join("expected.json") |> File.read!() |> Jason.decode!()
    id = expected["job_id"]
    status = if expected["pending"], do: :completed, else: :failed
    wait(fn -> match?({:ok, %{status: ^status}}, Store.get(id)) end)
    {:ok, final} = AnalysisResultStore.get(id)
    true = final[@key]["generation"] == expected["claim"]["generation"]
    true = final[@key]["attempt"] == expected["claim"]["attempt"]

    if expected["pending"] do
      true = Map.take(final, Map.keys(expected["result"])) === expected["result"]
      false = Map.has_key?(final[@key], "pending_completion")
      false = Map.has_key?(final[@key], "envelope")
    else
      "recovery_blocked" = final[@key]["state"]
    end

    "solve\n" = File.read!(Path.join(root, "solver-invocations"))
    :ok = Application.stop(:kyuubiki_web)
    IO.puts("CRASH_RECOVERED #{status}")
  end

  defp wait(check, remaining \\ 1_000)
  defp wait(_check, 0), do: raise("recovery timed out")

  defp wait(check, remaining) do
    unless check.() do
      Process.sleep(10)
      wait(check, remaining - 1)
    end
  end
end
