defmodule KyuubikiWeb.WorkflowNativeOperatorRuntimeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.{
    OperatorTaskExecutionSummary,
    OperatorTaskIR,
    OperatorTaskReadiness
  }

  alias KyuubikiWeb.{WorkflowNativeOperatorRuntime, WorkflowOperatorCatalog}

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
    def run_operator_task_ir(task, opts) do
      send(self(), {:task, task, opts})
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
    def run_operator_task_ir(task, opts) do
      {:ok, receipt} = Client.run_operator_task_ir(task, opts)
      {:ok, Map.put(receipt, "task_id", "different-task")}
    end
  end

  defmodule Pending do
    def run_operator_task_ir(task, opts) do
      {:ok, receipt} = Client.run_operator_task_ir(task, opts)

      {:ok,
       receipt
       |> Map.put("operator_task_ir_status", "verified_pending_execution")
       |> Map.put("execution_readiness", %{"status" => "blocked", "ready_to_dispatch" => false})}
    end
  end

  defp run(client, config, kind, id) do
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
      client: client
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
      assert task["runtime_hints"]["placement_tags"] == ["research"]
      assert task["runtime_hints"]["required_capabilities"] == ["task_ir"]
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
