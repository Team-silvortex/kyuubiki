defmodule KyuubikiWeb.Orchestra.WorkflowTerminalReconciliationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.AnalysisResultStore
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator, as: Coordinator
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryScan, as: Scan
  alias KyuubikiWeb.TestSupport.StorageOutageFixture, as: Outage
  alias KyuubikiWeb.TestSupport.WorkflowCommitFixture, as: Fixture

  @reason_prefix "workflow recovery blocked: "
  @message_limit 4096

  setup do
    Outage.setup()
  end

  for terminal <- ["failed", "recovery_blocked"],
      {label, history} <- [
        missing: :missing,
        null: nil,
        object: %{"message" => "not an event list"},
        text: "not an event list",
        number: 42,
        boolean: false,
        empty: [],
        invalid_last: [%{"message" => "stale diagnostic"}, nil],
        invalid_fields: [%{"message" => %{}, "reason" => 42}],
        empty_message: [%{"message" => ""}],
        blank_message: [%{"message" => " \n\t"}],
        empty_reason: [%{"reason" => ""}],
        blank_reason: [%{"reason" => " \n\t"}],
        oversized_message: [%{"message" => String.duplicate("m", @message_limit + 1)}],
        oversized_utf8: [
          %{"message" => String.duplicate("\u00e9", div(@message_limit, 2) + 1)}
        ],
        oversized_reason: [
          %{"reason" => String.duplicate("r", @message_limit - byte_size(@reason_prefix) + 1)}
        ]
      ] do
    @tag :direct
    test "#{terminal} reconciles #{label} diagnostics without rewriting evidence" do
      terminal = unquote(terminal)
      fixture = terminal_runtime(terminal, unquote(Macro.escape(history)))

      assert :ok = Scan.reconcile_terminal(fixture.id, terminal, recovery(fixture))
      assert_failed(fixture, fallback(terminal))
    end
  end

  for {label, entry, expected} <-
        [
          message: {%{"message" => "solver failed", "reason" => "secondary"}, "solver failed"},
          reason: {%{"reason" => "checkpoint invalid"}, @reason_prefix <> "checkpoint invalid"},
          invalid_message_reason: {
            %{"message" => [], "reason" => "checkpoint invalid"},
            @reason_prefix <> "checkpoint invalid"
          },
          blank_message_reason: {
            %{"message" => " \n\t", "reason" => "checkpoint invalid"},
            @reason_prefix <> "checkpoint invalid"
          },
          oversized_message_reason: {
            %{"message" => String.duplicate("m", @message_limit + 1), "reason" => "bounded"},
            @reason_prefix <> "bounded"
          },
          exact_message: {
            %{"message" => String.duplicate("m", @message_limit)},
            String.duplicate("m", @message_limit)
          },
          exact_reason: {
            %{"reason" => String.duplicate("r", @message_limit - byte_size(@reason_prefix))},
            @reason_prefix <> String.duplicate("r", @message_limit - byte_size(@reason_prefix))
          },
          exact_utf8: {
            %{"message" => String.duplicate("\u00e9", div(@message_limit, 2))},
            String.duplicate("\u00e9", div(@message_limit, 2))
          }
        ]
        |> Enum.map(fn {label, {entry, expected}} -> {label, entry, expected} end) do
    @tag :direct
    test "preserves usable #{label} diagnostics" do
      fixture = terminal_runtime("failed", [unquote(Macro.escape(entry))])
      assert :ok = Scan.reconcile_terminal(fixture.id, "failed", recovery(fixture))
      assert_failed(fixture, unquote(expected))
    end
  end

  for terminal <- ["completed", "cancelled"] do
    @tag :direct
    test "#{terminal} reconciliation does not depend on diagnostic history" do
      terminal = unquote(terminal)
      fixture = terminal_runtime(terminal, %{"message" => "malformed"})
      assert :ok = Scan.reconcile_terminal(fixture.id, terminal, recovery(fixture))
      assert {:ok, job} = Store.get(fixture.id)
      assert Atom.to_string(job.status) == terminal
      assert job.progress == 1.0
      assert_runtime_unchanged(fixture)
    end
  end

  for trigger <- [:scan, :restart],
      {label, history} <- [
        malformed: %{"message" => "not an event list"},
        oversized: [%{"message" => String.duplicate("m", @message_limit + 1)}]
      ] do
    test "#{trigger} isolates #{label} diagnostics and reaches a later healthy job" do
      healthy = terminal_runtime("completed", [])

      Outage.eventually(fn ->
        DateTime.compare(DateTime.utc_now(), healthy.job.updated_at) == :gt
      end)

      broken = terminal_runtime("failed", unquote(Macro.escape(history)))
      assert [first, second] = Store.list()
      assert {first.job_id, second.job_id} == {broken.id, healthy.id}

      if unquote(trigger) == :restart do
        Fixture.restart_coordinator()
      else
        pid = Process.whereis(Coordinator)

        assert %{"status" => "completed", "active_jobs" => 2, "skipped" => 2} =
                 Coordinator.recover_now()

        assert Process.whereis(Coordinator) == pid
      end

      pid = Process.whereis(Coordinator)
      assert %{"lease" => %{"status" => "owner"}, "tracked_jobs" => []} = Coordinator.snapshot()
      assert_failed(broken, fallback("failed"))
      assert {:ok, %{status: :completed, progress: 1.0}} = Store.get(healthy.id)
      assert_runtime_unchanged(healthy)
      assert %{"status" => "completed", "active_jobs" => 0} = Coordinator.recover_now()
      assert Process.whereis(Coordinator) == pid
      assert_failed(broken, fallback("failed"))
    end
  end

  defp terminal_runtime(terminal, history) do
    fixture = Fixture.claimed_job()
    recovery = Envelope.transition(recovery(fixture), terminal)

    recovery =
      if history == :missing,
        do: Map.delete(recovery, "history"),
        else: Map.put(recovery, "history", history)

    runtime = Map.put(fixture.runtime, Envelope.internal_key(), recovery)
    assert :ok = AnalysisResultStore.put(fixture.id, runtime)
    %{fixture | runtime: runtime}
  end

  defp recovery(fixture), do: fixture.runtime[Envelope.internal_key()]
  defp fallback(terminal), do: "workflow execution ended in #{terminal} state"

  defp assert_failed(fixture, message) do
    assert {:ok, job} = Store.get(fixture.id)
    assert job.status == :failed
    assert job.progress == 1.0
    assert job.message == message
    assert_runtime_unchanged(fixture)
  end

  defp assert_runtime_unchanged(fixture) do
    assert {:ok, runtime} = AnalysisResultStore.get(fixture.id)
    assert runtime == fixture.runtime
  end
end
