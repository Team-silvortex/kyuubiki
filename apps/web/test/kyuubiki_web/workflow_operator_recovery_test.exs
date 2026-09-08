defmodule KyuubikiWeb.WorkflowOperatorRecoveryTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.DistributedRecovery
  alias KyuubikiWeb.WorkflowOperatorRuntime

  defmodule RecordingClient do
    def request(method, payload, _progress, opts) do
      send(self(), {:dispatched, method, opts})
      {:ok, payload}
    end
  end

  setup do
    original = Application.get_env(:kyuubiki_web, WorkflowOperatorRuntime, [])

    Application.put_env(
      :kyuubiki_web,
      WorkflowOperatorRuntime,
      Keyword.put(original, :solve_runtime_client, RecordingClient)
    )

    on_exit(fn -> Application.put_env(:kyuubiki_web, WorkflowOperatorRuntime, original) end)
    :ok
  end

  test "workflow checkpoint-required node reaches transport recovery without weakening" do
    opts = dispatch(%{"retry_safety" => "checkpoint_required"})
    assert opts[:retry_safety] == "checkpoint_required"
    receipt = receipt(opts)
    refute receipt.retryable
    assert receipt.next_action == "checkpoint_before_retry"
  end

  test "workflow verified checkpoint and execution identity reach transport recovery" do
    checkpoint = %{
      "operator_task_batch_checkpoint_verification_contract" =>
        "kyuubiki.operator_task_batch_checkpoint_verification/v1",
      "status" => "verified",
      "checkpoint_digest" => String.duplicate("a", 64)
    }

    opts =
      dispatch(%{
        "retry_safety" => "checkpointed",
        "replay_checkpoint" => checkpoint,
        "orchestration_context" => %{"job_id" => "retained-research-job"}
      })

    assert opts[:retry_safety] == "checkpointed"
    assert opts[:replay_checkpoint] == checkpoint
    assert opts[:job_id] == "retained-research-job"
    assert receipt(opts).checkpoint_digest == checkpoint["checkpoint_digest"]
    assert receipt(opts).retryable
  end

  test "workflow invalid checkpoint assertion cannot inherit solver default idempotence" do
    opts = dispatch(%{"retry_safety" => "checkpointed"})
    refute receipt(opts).retryable
    assert receipt(opts).retry_safety == "checkpoint_required"
  end

  test "absent workflow retry policy retains the pure solver default" do
    opts = dispatch(%{})
    refute Keyword.has_key?(opts, :retry_safety)
    assert receipt(opts).retryable
  end

  test "policy alias is forwarded but cannot override an explicit invalid canonical policy" do
    alias_opts = dispatch(%{"replay_safety" => "checkpoint_required"})
    assert alias_opts[:retry_safety] == "checkpoint_required"
    refute receipt(alias_opts).retryable

    opts = dispatch(%{"retry_safety" => nil, "replay_safety" => "idempotent"})
    assert Keyword.has_key?(opts, :retry_safety)
    assert opts[:retry_safety] == nil
    refute receipt(opts).retryable
  end

  defp dispatch(node) do
    assert {:ok, %{}} =
             WorkflowOperatorRuntime.run_solve_operator("solve.thermal_plane_quad_2d", %{}, node)

    assert_receive {:dispatched, "solve_thermal_plane_quad_2d", opts}
    opts
  end

  defp receipt(opts) do
    DistributedRecovery.failure_receipt(
      %{id: "interrupted-agent"},
      "solve_thermal_plane_quad_2d",
      {:agent_transport_failure, :receive, :closed},
      opts,
      1,
      1
    )
  end
end
