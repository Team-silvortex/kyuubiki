defmodule KyuubikiWeb.Playground.AgentJobCancellationTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Playground.{AgentClient, AgentExecutionGate, AgentJobCancellation}

  @first %{id: "cancel-owner-first", host: "127.0.0.1", port: 5091, capacity: 3, token: "private"}
  @second %{id: "cancel-owner-second", host: "127.0.0.1", port: 5092, capacity: 1}

  test "cancellation targets all dispatches of one job without freeing active capacity" do
    for {endpoint, lease} <- [
          {@first, "cancel-one"},
          {@first, "cancel-two"},
          {@second, "cancel-three"}
        ] do
      assert {:ok, _, _} = AgentExecutionGate.acquire([endpoint], lease, 500, "shared-cancel")
      assert :ok = AgentExecutionGate.authorize_dispatch(lease)
    end

    assert {:ok, _, _} =
             AgentExecutionGate.acquire([@first], "cancel-bystander", 500, "other-job")

    assert :ok = AgentExecutionGate.authorize_dispatch("cancel-bystander")
    assert {:ok, targets} = AgentExecutionGate.cancel_job_targets("shared-cancel")
    assert Enum.sort_by(targets.endpoints, & &1.id) == [@first, @second]
    assert AgentExecutionGate.snapshot().active_lease_count == 4

    assert {:error, {:rpc_error, "cancelled", _}} =
             AgentExecutionGate.authorize_dispatch("cancel-one")

    assert :ok = AgentExecutionGate.authorize_dispatch("cancel-bystander")

    for lease <- ["cancel-one", "cancel-two", "cancel-three", "cancel-bystander"],
        do: AgentExecutionGate.release(lease)

    assert {:ok, %{endpoints: []}} = AgentExecutionGate.cancel_job_targets("shared-cancel")
  end

  test "queued cancellation never creates a pending cancellation on an agent" do
    endpoint = %{@second | id: "cancel-queued"}
    assert {:ok, _, _} = AgentExecutionGate.acquire([endpoint], "cancel-queue-holder", 500)

    queued =
      Task.async(fn ->
        AgentExecutionGate.acquire([endpoint], "cancel-queue-waiter", 2_000, "queued-job")
      end)

    await_queue()

    assert {:ok, receipt} =
             AgentJobCancellation.cancel("queued-job", fn _ ->
               flunk("queued work must not send RPC")
             end)

    assert receipt["status"] == "cancelled_before_dispatch"
    assert receipt["queued_cancelled_count"] == 1
    assert {:error, {:rpc_error, "cancelled", _}} = Task.await(queued)
    assert AgentExecutionGate.snapshot().queued_request_count == 0
    assert :ok = AgentExecutionGate.release("cancel-queue-holder")

    assert {:ok, _, _} =
             AgentExecutionGate.acquire([endpoint], "cancel-queue-rerun", 500, "queued-job")

    assert :ok = AgentExecutionGate.authorize_dispatch("cancel-queue-rerun")
    AgentExecutionGate.release("cancel-queue-rerun")
  end

  test "reserved work is stopped locally before it can open an agent connection" do
    assert {:ok, _, _} =
             AgentExecutionGate.acquire([@first], "cancel-reserved", 500, "reserved-job")

    claim_error = {:error, :execution_claim_rejected}
    assert AgentJobCancellation.finish_dispatch("cancel-reserved", claim_error) == claim_error

    assert {:ok, receipt} =
             AgentJobCancellation.cancel("reserved-job", fn _ ->
               flunk("reserved work must not send RPC")
             end)

    assert receipt["reserved_cancelled_count"] == 1

    assert {:error, {:rpc_error, "cancelled", _}} =
             AgentExecutionGate.authorize_dispatch("cancel-reserved")

    assert AgentExecutionGate.snapshot().active_lease_count == 1
    AgentExecutionGate.release("cancel-reserved")
  end

  test "unknown jobs and malformed identities do not contact an arbitrary agent" do
    assert {:ok, receipt} = AgentClient.cancel_job("cancel-no-active-owner")
    assert receipt["status"] == "no_active_dispatch"
    refute receipt["execution_terminal_confirmed"]

    for invalid <- ["", "bad\njob", "bad\u0085job", String.duplicate("x", 257), <<255>>] do
      assert {:error, :invalid_execution_job_id} = AgentExecutionGate.cancel_job_targets(invalid)

      assert {:error, :invalid_execution_job_id} =
               AgentExecutionGate.acquire([@first], "invalid-job-lease", 500, invalid)
    end
  end

  test "partial delivery is explicit and never reports global execution termination" do
    targets = %{
      endpoints: [@first, @second],
      queued_cancelled_count: 0,
      reserved_cancelled_count: 0
    }

    assert {:ok, receipt} =
             AgentJobCancellation.cancel_targets("partial-cancel", targets, fn
               %{id: "cancel-owner-first"} ->
                 {:ok,
                  %{
                    "job_id" => "partial-cancel",
                    "cancel_registered" => true,
                    "operator_package_job_release" => %{
                      "status" => "failed",
                      "error_code" => "cleanup_failed"
                    }
                  }}

               %{id: "cancel-owner-second"} ->
                 {:error, {:agent_transport_failure, :connect, :econnrefused}}
             end)

    assert receipt["status"] == "partially_requested"
    assert receipt["registered_count"] == 1
    refute receipt["execution_terminal_confirmed"]
    assert hd(receipt["targets"])["operator_package_job_release"]["status"] == "failed"
    refute Jason.encode!(receipt) =~ "private"
    assert List.last(receipt["targets"])["error_code"] == "cancellation_delivery_failed"
  end

  test "typed acknowledgements require a matching task identity and known schema" do
    targets = %{endpoints: [@first], queued_cancelled_count: 0, reserved_cancelled_count: 0}

    for result <- [
          %{
            "schema_version" => "kyuubiki.agent-job-cancellation/v1",
            "cancel_registered" => true
          },
          %{"job_id" => "other", "cancel_registered" => true},
          %{"schema_version" => "future/v9", "job_id" => "ack-job", "cancel_registered" => true},
          %{"cancel_registered" => false, "cancelled" => true}
        ] do
      assert {:ok, receipt} =
               AgentJobCancellation.cancel_targets("ack-job", targets, fn _ -> {:ok, result} end)

      assert receipt["status"] == "delivery_failed"
      assert receipt["registered_count"] == 0
    end
  end

  test "timeout cleanup captures the dispatch target before killing its local owner" do
    parent = self()

    task =
      Task.async(fn ->
        {:ok, _, _} =
          AgentExecutionGate.acquire([@first], "cancel-timeout-owner", 500, "timeout-job")

        :ok = AgentExecutionGate.authorize_dispatch("cancel-timeout-owner")
        send(parent, :dispatch_owned)
        receive do: (:never -> :ok)
      end)

    assert_receive :dispatch_owned
    assert {:ok, nil, targets} = AgentJobCancellation.stop_dispatch(task, "timeout-job")
    assert targets.endpoints == [@first]

    assert {:ok, receipt} =
             AgentJobCancellation.cancel_targets("timeout-job", targets, fn endpoint ->
               assert endpoint == @first
               {:ok, %{"job_id" => "timeout-job", "cancel_registered" => true}}
             end)

    assert receipt["status"] == "requested"
    await_no_owner()
    assert {:ok, %{endpoints: []}} = AgentExecutionGate.cancel_job_targets("timeout-job")
  end

  test "cancelled dispatch keeps a native TaskIR failure receipt instead of flattening it" do
    {:ok, _, _} = AgentExecutionGate.acquire([@first], "cancel-native-receipt", 500, "native-job")
    :ok = AgentExecutionGate.authorize_dispatch("cancel-native-receipt")
    {:ok, _} = AgentExecutionGate.cancel_job_targets("native-job")

    failure =
      {:error,
       {:operator_task_rpc_error, "cancelled", "native cancellation",
        %{"task_id" => "native-job", "task_digest" => "bound-digest"}}}

    assert AgentJobCancellation.finish_dispatch("cancel-native-receipt", failure) == failure

    assert {:error, {:rpc_error, "cancelled", _}} =
             AgentJobCancellation.finish_dispatch(
               "cancel-native-receipt",
               {:ok, %{"unexpected" => "late result"}}
             )

    assert {:error, {:rpc_error, "cancelled", _}} =
             AgentJobCancellation.finish_dispatch(
               "cancel-native-receipt",
               {:error, {:agent_transport_failure, :receive, :closed}}
             )

    AgentExecutionGate.release("cancel-native-receipt")
  end

  defp await_queue(attempts \\ 200)
  defp await_queue(0), do: flunk("request did not enter the capacity queue")

  defp await_queue(attempts) do
    if AgentExecutionGate.snapshot().queued_request_count == 0 do
      Process.sleep(5)
      await_queue(attempts - 1)
    end
  end

  defp await_no_owner(attempts \\ 200)
  defp await_no_owner(0), do: flunk("owner DOWN did not remove the execution lease")

  defp await_no_owner(attempts) do
    if AgentExecutionGate.snapshot().active_lease_count != 0 do
      Process.sleep(5)
      await_no_owner(attempts - 1)
    end
  end
end
