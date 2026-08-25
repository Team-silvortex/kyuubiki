defmodule KyuubikiWeb.Orchestra.LeaseStoreTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Analysis
  alias KyuubikiWeb.Jobs.Store
  alias KyuubikiWeb.Orchestra.LeaseMemoryBackend
  alias KyuubikiWeb.Orchestra.LeaseStore
  alias KyuubikiWeb.Orchestra.WorkflowRecoveryCoordinator

  test "SQL lease rejects a second owner and fences the old token after expiry" do
    lease_name = unique_lease_name("sql-lifecycle")

    assert {:ok, first} = LeaseStore.acquire(lease_name, "orchestra-a", 5_000)
    assert first.fencing_token == 1
    assert :protected == LeaseStore.with_lease(first, fn -> :protected end)
    assert {:ok, exclusive} = LeaseStore.renew(first, 5_000)

    assert {:error, {:lease_held, held}} =
             LeaseStore.acquire(lease_name, "orchestra-b", 5_000)

    assert held.owner_instance_id == "orchestra-a"
    assert {:ok, expiring} = LeaseStore.renew(exclusive, 500)
    assert expiring.fencing_token == first.fencing_token

    Process.sleep(700)

    assert {:ok, takeover} = LeaseStore.acquire(lease_name, "orchestra-b", 500)
    assert takeover.fencing_token == first.fencing_token + 1
    assert {:error, :orchestra_lease_lost} = LeaseStore.renew(expiring, 100)

    assert {:error, :orchestra_lease_lost} =
             LeaseStore.with_lease(expiring, fn -> flunk("stale lease callback executed") end)

    assert :ok = LeaseStore.release(takeover)
    assert {:ok, reacquired} = LeaseStore.acquire(lease_name, "orchestra-b", 500)
    assert reacquired.fencing_token == takeover.fencing_token + 1
    assert :ok = LeaseStore.release(reacquired)
  end

  test "SQL lease permits exactly one concurrent expired-lease takeover" do
    lease_name = unique_lease_name("sql-race")
    assert {:ok, first} = LeaseStore.acquire(lease_name, "orchestra-original", 40)
    Process.sleep(70)

    results =
      1..8
      |> Enum.map(fn candidate ->
        Task.async(fn ->
          result = LeaseStore.acquire(lease_name, "orchestra-#{candidate}", 1_000)
          {candidate, result}
        end)
      end)
      |> Task.await_many(5_000)

    winners = for {candidate, {:ok, lease}} <- results, do: {candidate, lease}
    held = for {_candidate, {:error, {:lease_held, _lease}}} <- results, do: :held

    assert [{winner, lease}] = winners
    assert length(held) == 7
    assert lease.owner_instance_id == "orchestra-#{winner}"
    assert lease.fencing_token == first.fencing_token + 1
    assert :ok = LeaseStore.release(lease)
  end

  test "memory lease follows the same ownership and fencing contract" do
    if is_nil(Process.whereis(LeaseMemoryBackend)) do
      start_supervised!({LeaseMemoryBackend, []})
    end

    lease_name = unique_lease_name("memory")

    assert {:ok, first} = LeaseMemoryBackend.acquire(lease_name, "memory-a", 40)

    assert {:error, {:lease_held, _lease}} =
             LeaseMemoryBackend.acquire(lease_name, "memory-b", 40)

    Process.sleep(70)
    assert {:ok, takeover} = LeaseMemoryBackend.acquire(lease_name, "memory-b", 200)
    assert takeover.fencing_token == first.fencing_token + 1

    assert {:error, :orchestra_lease_lost} =
             LeaseMemoryBackend.with_lease(first, fn -> :stale_write end)

    assert :ok = LeaseMemoryBackend.release(takeover)
    assert {:ok, reacquired} = LeaseMemoryBackend.acquire(lease_name, "memory-b", 200)
    assert reacquired.fencing_token == takeover.fencing_token + 1
    assert :ok = LeaseMemoryBackend.release(reacquired)
  end

  test "memory lease callbacks can inspect ownership without deadlocking" do
    ensure_memory_lease_backend()
    lease_name = unique_lease_name("memory-reentrant-read")
    assert {:ok, lease} = LeaseMemoryBackend.acquire(lease_name, "memory-reader", 1_000)

    assert {:ok, current} =
             LeaseMemoryBackend.with_lease(lease, fn ->
               LeaseMemoryBackend.current(lease_name)
             end)

    assert current.fencing_token == lease.fencing_token
    assert :ok = LeaseMemoryBackend.release(lease)
  end

  test "memory lease callback does not block unrelated lease names" do
    ensure_memory_lease_backend()
    guarded_name = unique_lease_name("memory-guarded")
    unrelated_name = unique_lease_name("memory-unrelated")
    assert {:ok, guarded} = LeaseMemoryBackend.acquire(guarded_name, "memory-a", 1_000)
    parent = self()

    guarded_task =
      Task.async(fn ->
        LeaseMemoryBackend.with_lease(guarded, fn ->
          send(parent, {:memory_guard_entered, self()})

          receive do
            :finish_memory_guard -> :protected
          after
            1_000 -> :timed_out
          end
        end)
      end)

    assert_receive {:memory_guard_entered, guard_pid}, 500

    unrelated_task =
      Task.async(fn ->
        LeaseMemoryBackend.acquire(unrelated_name, "memory-b", 1_000)
      end)

    assert {:ok, unrelated} = Task.await(unrelated_task, 500)
    send(guard_pid, :finish_memory_guard)
    assert :protected = Task.await(guarded_task, 500)
    assert :ok = LeaseMemoryBackend.release(guarded)
    assert :ok = LeaseMemoryBackend.release(unrelated)
  end

  test "memory lease fencing blocks takeover until a guarded callback exits" do
    ensure_memory_lease_backend()
    lease_name = unique_lease_name("memory-guarded-takeover")
    assert {:ok, lease} = LeaseMemoryBackend.acquire(lease_name, "memory-a", 30)
    parent = self()

    guarded_task =
      Task.async(fn ->
        LeaseMemoryBackend.with_lease(lease, fn ->
          send(parent, {:takeover_guard_entered, self()})

          receive do
            :finish_takeover_guard -> :protected
          after
            1_000 -> :timed_out
          end
        end)
      end)

    assert_receive {:takeover_guard_entered, guard_pid}, 500
    Process.sleep(50)

    takeover_task =
      Task.async(fn ->
        LeaseMemoryBackend.acquire(lease_name, "memory-b", 1_000)
      end)

    assert Task.yield(takeover_task, 50) == nil
    send(guard_pid, :finish_takeover_guard)
    assert :protected = Task.await(guarded_task, 500)
    assert {:ok, takeover} = Task.await(takeover_task, 1_000)
    assert takeover.fencing_token == lease.fencing_token + 1
    assert :ok = LeaseMemoryBackend.release(takeover)
  end

  test "workflow coordinator demotes immediately when its persisted lease is lost" do
    snapshot = WorkflowRecoveryCoordinator.snapshot()
    assert snapshot["lease"]["status"] == "owner"
    assert {:ok, token} = LeaseStore.current(snapshot["lease"]["lease_name"])
    assert :ok = LeaseStore.release(token)

    assert {:error, :orchestra_lease_lost} =
             WorkflowRecoveryCoordinator.initialize(
               "lease-loss-probe",
               %{},
               %{},
               %{},
               %{}
             )

    assert WorkflowRecoveryCoordinator.snapshot()["lease"]["status"] == "standby"
    send(Process.whereis(WorkflowRecoveryCoordinator), :acquire_lease)
    assert wait_for_coordinator_owner()
  end

  test "standby workflow submission rolls back its newly created queue record" do
    existing_job_ids = Store.list() |> Enum.map(& &1.job_id) |> MapSet.new()
    snapshot = WorkflowRecoveryCoordinator.snapshot()
    assert {:ok, token} = LeaseStore.current(snapshot["lease"]["lease_name"])
    assert :ok = LeaseStore.release(token)

    assert {:error, :orchestra_lease_lost} =
             Analysis.submit_workflow_graph(%{
               "graph" => %{
                 "schema_version" => "kyuubiki.workflow-graph/v1",
                 "id" => "workflow.standby-rollback-test",
                 "nodes" => [],
                 "edges" => []
               },
               "input_artifacts" => %{}
             })

    assert Store.list() |> Enum.map(& &1.job_id) |> MapSet.new() == existing_job_ids
    assert WorkflowRecoveryCoordinator.snapshot()["lease"]["status"] == "standby"
    send(Process.whereis(WorkflowRecoveryCoordinator), :acquire_lease)
    assert wait_for_coordinator_owner()
  end

  test "lease store rejects malformed fencing tokens without touching storage" do
    assert {:error, :invalid_lease_request} = LeaseStore.renew(%{}, 100)
    assert {:error, :invalid_lease_request} = LeaseStore.release(%{})

    assert {:error, :invalid_lease_request} =
             LeaseStore.with_lease(%{}, fn -> flunk("invalid lease callback executed") end)

    malformed = %{lease_name: 1, owner_instance_id: "memory", fencing_token: "stale"}
    assert {:error, :invalid_lease_request} = LeaseMemoryBackend.renew(malformed, 100)
    assert {:error, :invalid_lease_request} = LeaseMemoryBackend.release(malformed)

    assert {:error, :invalid_lease_request} =
             LeaseMemoryBackend.with_lease(malformed, fn ->
               flunk("invalid memory lease callback executed")
             end)
  end

  defp unique_lease_name(prefix) do
    suffix = :crypto.strong_rand_bytes(8) |> Base.url_encode64(padding: false)
    "#{prefix}-#{System.pid()}-#{suffix}"
  end

  defp ensure_memory_lease_backend do
    if is_nil(Process.whereis(LeaseMemoryBackend)) do
      start_supervised!({LeaseMemoryBackend, []})
    end
  end

  defp wait_for_coordinator_owner(attempts \\ 100)

  defp wait_for_coordinator_owner(0), do: false

  defp wait_for_coordinator_owner(attempts) do
    if WorkflowRecoveryCoordinator.snapshot()["lease"]["status"] == "owner" do
      true
    else
      Process.sleep(10)
      wait_for_coordinator_owner(attempts - 1)
    end
  end
end
