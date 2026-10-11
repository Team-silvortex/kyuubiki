defmodule KyuubikiWeb.WorkflowNativeOperatorRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskExecutionSummary,
    OperatorTaskIR,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.{
    WorkflowNativeInvocation,
    WorkflowNativeOperatorRuntime,
    WorkflowOperatorCatalog
  }

  @operators [
    {"extract", "extract.transport_result_diagnostics"},
    {"extract", "extract.thermal_result_diagnostics"},
    {"extract", "extract.electrostatic_result_diagnostics"},
    {"extract", "extract.magnetostatic_result_diagnostics"},
    {"extract", "extract.thermo_result_diagnostics"},
    {"extract", "extract.stokes_flow_result_diagnostics"},
    {"transform", "transform.score_transport_quality"},
    {"transform", "transform.score_thermal_quality"},
    {"transform", "transform.score_electrostatic_quality"},
    {"transform", "transform.score_magnetostatic_quality"},
    {"transform", "transform.score_cfd_quality"}
  ]

  defmodule Client do
    def run_operator_task_ir(task, opts, on_progress \\ fn _ -> :ok end) do
      with :ok <- Keyword.get(opts, :before_dispatch, fn -> :ok end).() do
        execute(task, opts, on_progress)
      end
    end

    defp execute(task, opts, on_progress) do
      send(self(), {:task, task, opts})
      on_progress.(%{"stage" => "result_diagnostics", "completed_steps" => 64})
      {:ok, summary} = OperatorTaskExecutionSummary.build(task)

      {:ok,
       summary
       |> Map.take(~w(task_id task_digest operator_id program_id))
       |> Map.merge(%{
         "operator_task_ir_status" => "executed",
         "execution_readiness" => OperatorTaskReadiness.local_executed(),
         "result" => %{"ready" => true}
       })}
    end
  end

  defmodule WrongIdentity do
    def run_operator_task_ir(task, opts, on_progress) do
      {:ok, receipt} = Client.run_operator_task_ir(task, opts, on_progress)
      {:ok, Map.put(receipt, "task_id", "different-task")}
    end
  end

  defmodule Pending do
    def run_operator_task_ir(task, opts, on_progress) do
      {:ok, receipt} = Client.run_operator_task_ir(task, opts, on_progress)

      {:ok,
       receipt
       |> Map.put("operator_task_ir_status", "verified_pending_execution")
       |> Map.put("execution_readiness", %{"status" => "blocked", "ready_to_dispatch" => false})}
    end
  end

  defp run(client, config, kind, id, opts \\ []) do
    WorkflowNativeOperatorRuntime.run(
      kind,
      id,
      %{"nodes" => []},
      config,
      %{
        "id" => "diagnose",
        "kind" => kind,
        "operator_id" => id,
        "inputs" => [],
        "outputs" => [],
        "placement_tags" => ["research"],
        "required_capabilities" => ["task_ir"]
      },
      %{"job_id" => "parent-job", "orch_id" => "orch-a", "control_mode" => "orch_managed"},
      Keyword.put(opts, :client, client)
    )
  end

  test "native catalog declares an unbundled engine entrypoint, not a package fetch" do
    for {kind, id} <- @operators do
      assert WorkflowNativeOperatorRuntime.supports?(kind, id)
      {:ok, %{"operator" => operator}} = WorkflowOperatorCatalog.fetch(id)
      assert operator["execution"]["execution_mode"] == "agent_native"
      assert operator["execution"]["agent_fetchable"] == false
      assert operator["execution"]["package_ref"] == nil
      assert operator["execution"]["source_ref"] == "builtin://engine/#{id}"

      refute WorkflowNativeOperatorRuntime.supports?(
               if(kind == "extract", do: "transform", else: "extract"),
               id
             )

      refute WorkflowNativeOperatorRuntime.supports?(kind, id <> "_unknown")
    end

    refute WorkflowNativeOperatorRuntime.supports?(
             "transform",
             "extract.transport_result_diagnostics"
           )

    refute WorkflowNativeOperatorRuntime.supports?("extract", "extract.unknown")
  end

  test "explicit typed workflow tasks are sealed without normalizing configuration" do
    encoding = KyuubikiWeb.TaskDigestV2.encoding()
    config = %{"witness" => [1, 1.0, -0.0, 0.0, 1.0e-30]}

    assert {:ok, _} =
             run(Client, config, "transform", "transform.score_thermal_quality",
               native_task_digest_encoding: encoding
             )

    assert_receive {:task, task, _opts}
    assert task["config"] === config
    assert task["integrity"]["task_digest_encoding"] == encoding
    assert {:ok, digest} = OperatorTaskIR.compute_task_digest_checked(task)
    assert task["integrity"]["task_digest"] == digest
    assert encoding in OperatorTaskIR.required_agent_capabilities(task)
  end

  test "invalid workflow digest encoding cannot reach dispatch" do
    for encoding <- [nil, false, "unknown"] do
      assert {:error, _} =
               run(Client, %{}, "transform", "transform.score_thermal_quality",
                 native_task_digest_encoding: encoding
               )

      refute_receive {:task, _, _}
    end
  end

  test "TaskIR binds original node, parent job, authority context and placement without copying physics" do
    for {kind, id} <- @operators do
      config = %{"enabled_terms" => ["explicit-term"]}
      assert {:ok, %{"ready" => true}} = run(Client, config, kind, id)
      assert_receive {:task, task, [mode: :execute, job_id: "parent-job"]}
      assert :ok = OperatorTaskExecutionSummary.validate_digest(task)
      assert task["integrity"]["task_digest"] == OperatorTaskIR.compute_task_digest(task)
      assert task["node"]["id"] == "diagnose"
      assert task["task_id"] == "parent-job:diagnose:#{id}"
      assert task["operator"]["id"] == id
      assert task["operator"]["kind"] == kind
      assert task["input_artifact"] == %{"nodes" => []}
      assert task["config"] == config
      assert task["orchestration_context"]["orch_id"] == "orch-a"

      context = %{
        "job_id" => "parent-job",
        "orch_id" => "orch-a",
        "control_mode" => "orch_managed"
      }

      assert task["orchestration_context"] ==
               WorkflowNativeInvocation.bind_context(context, %{"nodes" => []}, config)

      assert task["runtime_hints"]["placement_tags"] == ["research"]
      assert task["runtime_hints"]["required_capabilities"] == ["task_ir"]
    end
  end

  test "native invocation binding preserves numeric payloads and overrides caller claims" do
    payload = %{"nodes" => [], "tiny" => [1.0e-300, -0.0]}
    config = %{"threshold" => 2.0e-300}
    context = %{"job_id" => "parent-job", "workflow_native_invocation_sha256" => "forged"}

    for {kind, id} <- @operators do
      assert {:ok, %{"ready" => true}} =
               WorkflowNativeOperatorRuntime.run(
                 kind,
                 id,
                 payload,
                 config,
                 %{"id" => "native", "kind" => kind, "operator_id" => id},
                 context,
                 client: Client
               )

      assert_receive {:task, task, _}

      assert KyuubikiWeb.Storage.DurableJson.encode!(task["input_artifact"]) ==
               KyuubikiWeb.Storage.DurableJson.encode!(payload)

      assert KyuubikiWeb.Storage.DurableJson.encode!(task["config"]) ==
               KyuubikiWeb.Storage.DurableJson.encode!(config)

      assert task["orchestration_context"] ==
               WorkflowNativeInvocation.bind_context(context, payload, config)

      assert :ok = OperatorTaskExecutionSummary.validate_digest(task)
    end
  end

  test "stale identity and pending delivery acknowledgements cannot publish operator output" do
    for {kind, id} <- @operators do
      assert {:error, {:operator_task_execution_receipt_invalid, "task_id"}} =
               run(WrongIdentity, %{}, kind, id)

      assert_receive {:task, _, _}

      assert {:error, {:native_workflow_operator_not_executed, "blocked"}} =
               run(Pending, %{}, kind, id)

      assert_receive {:task, _, _}
    end
  end

  test "invalid config cannot silently turn into default execution" do
    assert {:error, :invalid_native_workflow_operator_request} =
             run(Client, false, "extract", "extract.transport_result_diagnostics")

    refute_receive {:task, _, _}
  end

  test "native diagnostics and scoring forward activity without changing the receipt" do
    for {kind, id} <- @operators do
      on_progress = fn activity -> send(self(), {:activity, id, activity}) end
      assert {:ok, %{"ready" => true}} = run(Client, %{}, kind, id, on_progress: on_progress)
      assert_receive {:task, _, _}
      assert_receive {:activity, ^id, %{"stage" => "result_diagnostics", "completed_steps" => 64}}
    end
  end

  test "invalid activity consumers cannot dispatch a native task" do
    assert {:error, :invalid_native_workflow_operator_request} =
             run(Client, %{}, "extract", "extract.transport_result_diagnostics", on_progress: nil)

    refute_receive {:task, _, _}
  end

  test "native dispatch guards remain out of TaskIR and can block every supported operator" do
    for {kind, id} <- @operators do
      guard = fn ->
        send(self(), {:authorized, id})
        :ok
      end

      assert {:ok, %{"ready" => true}} = run(Client, %{}, kind, id, before_dispatch: guard)
      assert_receive {:authorized, ^id}
      assert_receive {:task, task, opts}
      assert opts[:before_dispatch] == guard
      assert {:ok, _} = Jason.encode(task)
      assert :ok = OperatorTaskExecutionSummary.validate_digest(task)

      assert {:error, :cancelled_before_dispatch} =
               run(Client, %{}, kind, id,
                 before_dispatch: fn -> {:error, :cancelled_before_dispatch} end
               )

      refute_receive {:task, _, _}, 0
    end
  end

  test "malformed native dispatch guards cannot be silently ignored" do
    for guard <- [nil, false, fn _ -> :ok end] do
      assert {:error, :invalid_native_workflow_operator_request} =
               run(Client, %{}, "extract", "extract.transport_result_diagnostics",
                 before_dispatch: guard
               )

      refute_receive {:task, _, _}, 0
    end
  end

  test "unsupported kinds and prefixed unknown IDs never reach the Agent client" do
    for {kind, id} <- @operators do
      wrong = if kind == "extract", do: "transform", else: "extract"

      assert {:error, {:unsupported_native_workflow_operator, ^wrong, ^id}} =
               run(Client, %{}, wrong, id)

      unknown = id <> "_unknown"

      assert {:error, {:unsupported_native_workflow_operator, ^kind, ^unknown}} =
               run(Client, %{}, kind, unknown)
    end

    refute_receive {:task, _, _}
  end
end
