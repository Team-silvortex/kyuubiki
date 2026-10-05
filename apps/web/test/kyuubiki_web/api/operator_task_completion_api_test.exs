defmodule KyuubikiWeb.OperatorTaskCompletionApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Orchestra.OperatorTaskReadiness

  test "single execution exposes Agent blocking instead of dispatch success and can recover" do
    task = central_task("pending-task")
    blocked = agent_receipt(task, "verified_pending_engine_execution")
    start_agent(blocked)

    {200, payload} = request("execute", %{"task" => task})
    assert payload["status"] == "blocked"
    assert payload["task_id"] == task["task_id"]
    assert payload["result"] == blocked
    assert payload["execution_readiness"]["required_action"] == "attach_operator_package_runtime"
    assert_receive {:fake_agent_request, sent}
    assert sent["params"]["task_ir"] == task

    completed = agent_receipt(task, "executed")
    start_agent(completed)
    {200, recovered} = request("execute", %{"task" => task})
    assert recovered["status"] == "executed"
    assert recovered["result"] == completed
    assert recovered["execution_readiness"] == OperatorTaskReadiness.local_executed()
  end

  test "malformed or stale Agent success cannot be promoted by the service API" do
    task = central_task("receipt-task")

    for {field, value} <- [
          {"task_id", "secret-stale-value"},
          {"task_digest", nil},
          {"operator_id", "wrong"},
          {"program_id", "wrong"},
          {"operator_task_ir_status", "unknown"},
          {"execution_readiness", nil},
          {"execution_readiness", []},
          {"result", nil},
          {"result", []},
          {"ok", false},
          {"error", "secret-agent-error"},
          {"blocked_stage", "fetch_package"},
          {"validation_receipt", %{"digest_verified" => false}},
          {"validation_receipt", nil},
          {"provenance_receipt", %{"task_digest" => "secret-stale-digest"}},
          {"provenance_receipt", []},
          {"execution_readiness", %{"status" => "executed", "ready_to_dispatch" => false}}
        ] do
      start_agent(Map.put(agent_receipt(task, "executed"), field, value))
      {422, payload} = request("execute", %{"task" => task})
      assert payload["error_code"] == "operator_task_execution_receipt_invalid"
      refute payload["error"] =~ "secret-stale-value"
      refute payload["error"] =~ "secret-agent-error"
      refute payload["error"] =~ "secret-stale-digest"
    end
  end

  test "contradictory executed readiness fails closed on the actual Agent RPC boundary" do
    task = central_task("contradictory-task")

    for field <- ["blocking_stage", "blocking_reason", "required_action"] do
      receipt =
        put_in(agent_receipt(task, "executed"), ["execution_readiness", field], "secret-value")

      start_agent(receipt)
      {422, payload} = request("execute", %{"task" => task})
      assert payload["error_code"] == "operator_task_execution_receipt_invalid"
      refute payload["error"] =~ "secret-value"
    end
  end

  test "a pending package batch remains blocked across checkpoint JSON and resume planning" do
    task = central_task("batch-pending")
    receipt = agent_receipt(task, "verified_pending_engine_execution")
    start_agent(receipt)
    batch = batch([{"case-pending", task}])
    {200, execution} = request("execute-batch", %{"batch" => batch})
    assert execution["status"] == "blocked"
    assert execution["attempted_count"] == 1
    assert execution["executed_count"] == 0
    assert execution["ok_count"] == 0
    assert execution["blocked_count"] == 1
    assert execution["error_count"] == 0
    assert execution["skipped_count"] == 0
    assert execution["results"] |> hd() |> Map.get("status") == "blocked"
    assert hd(execution["results"])["result"] == receipt
    assert execution["readiness_counts"] == %{"blocked" => 1}

    {200, checkpoint} = request("checkpoint-batch", %{"batch" => batch, "execution" => execution})
    assert checkpoint["resume_policy"]["next_action"] == "resolve_blocked_cases"
    {200, plan} = request("resume-plan-batch", %{"batch" => batch, "checkpoint" => checkpoint})
    assert plan["target_case_ids"] == ["case-pending"]
    assert plan["blocked_case_ids"] == ["case-pending"]
    assert plan["recovery_actions"] == ["attach_operator_package_runtime"]
  end

  test "package resolution readiness is noncompletion and survives checkpointing" do
    task = central_task("package-ready")

    receipt =
      agent_receipt(task, "verified_pending_engine_execution")
      |> put_in(["execution_readiness", "status"], "ready_for_package_resolution")
      |> put_in(["execution_readiness", "blocking_stage"], nil)
      |> put_in(
        ["execution_readiness", "required_action"],
        "resolve_fetch_verify_and_activate_package"
      )

    start_agent(receipt)
    batch = batch([{"case-package", task}])
    {200, execution} = request("execute-batch", %{"batch" => batch})
    assert execution["status"] == "blocked"
    assert execution["executed_count"] == 0
    assert execution["readiness_counts"] == %{"ready_for_package_resolution" => 1}
    checkpoint = OperatorTaskBatchRun.checkpoint(batch, execution: execution)
    assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)
    assert plan["next_action"] == "resolve_blocked_cases"
    assert plan["target_case_ids"] == ["case-package"]
    assert plan["recovery_actions"] == ["resolve_fetch_verify_and_activate_package"]
  end

  test "strict batches preserve skipped tasks and do not execute past an Agent blocker" do
    task = central_task("strict-pending")
    start_agent(agent_receipt(task, "verified_pending_engine_execution"))

    batch =
      batch([
        {"case-good", local_task("good")},
        {"case-blocked", task},
        {"case-skipped", local_task("skipped")}
      ])

    assert {:ok, execution} = OperatorTaskExecutor.execute_batch(batch, strict: true)
    assert execution["attempted_count"] == 2
    assert execution["executed_count"] == 1
    assert execution["blocked_count"] == 1
    assert execution["error_count"] == 0
    assert execution["skipped_count"] == 1
    assert execution["skipped_case_ids"] == ["case-skipped"]
    assert Enum.map(execution["results"], & &1["case_id"]) == ["case-good", "case-blocked"]

    checkpoint =
      execution
      |> then(&OperatorTaskBatchRun.checkpoint(batch, execution: &1))
      |> Jason.encode!()
      |> Jason.decode!()

    assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)
    assert plan["next_action"] == "resolve_incomplete_cases"
    assert plan["target_case_ids"] == ["case-blocked", "case-skipped"]
    assert plan["blocked_case_ids"] == ["case-blocked"]
    assert plan["recovery_actions"] == ["attach_operator_package_runtime"]
  end

  test "mixed independent cases retain successful data and all unresolved recovery targets" do
    pending = central_task("mixed-pending")
    start_agent(agent_receipt(pending, "verified_pending_engine_execution"))
    bad = put_in(local_task("bad"), ["config", "tampered"], true)

    batch =
      batch([{"case-good", local_task("good")}, {"case-pending", pending}, {"case-bad", bad}])

    {200, execution} = request("execute-batch", %{"batch" => batch})
    assert execution["status"] == "partial"
    assert execution["attempted_count"] == 3
    assert execution["executed_count"] == 1
    assert execution["ok_count"] == 1
    assert execution["blocked_count"] == 1
    assert execution["error_count"] == 1
    assert execution["failed_case_ids"] == ["case-bad"]
    assert Jason.decode!(hd(execution["results"])["result"]["content"])["value"] == 14.0
    assert Enum.map(execution["results"], & &1["status"]) == ["ok", "blocked", "error"]
    checkpoint = OperatorTaskBatchRun.checkpoint(batch, execution: execution)
    assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)
    assert plan["next_action"] == "resolve_incomplete_cases"
    assert plan["target_case_ids"] == ["case-pending", "case-bad"]
    assert plan["blocked_case_ids"] == ["case-pending"]

    assert Enum.sort(plan["recovery_actions"]) ==
             ["attach_operator_package_runtime", "rebuild_task_ir_and_recompute_digest"]
  end

  test "error-only API batches never report executed and cannot archive" do
    task = put_in(local_task("bad"), ["config", "tampered"], true)
    batch = batch([{"case-bad", task}])
    {200, execution} = request("execute-batch", %{"batch" => batch})
    assert execution["status"] == "failed"
    assert execution["attempted_count"] == 1
    assert execution["executed_count"] == 0
    assert execution["error_count"] == 1
    checkpoint = OperatorTaskBatchRun.checkpoint(batch, execution: execution)
    assert checkpoint["resume_policy"]["next_action"] == "retry_failed_cases"
  end

  test "zero error counts without complete case coverage never authorize archival" do
    batch = batch([{"case-a", local_task("a")}, {"case-b", local_task("b")}])

    execution = %{
      "run_phase" => "execute",
      "task_count" => 2,
      "executed_count" => 1,
      "ok_count" => 1,
      "error_count" => 0,
      "results" => []
    }

    checkpoint = OperatorTaskBatchRun.checkpoint(batch, execution: execution)
    refute checkpoint["resume_policy"]["next_action"] == "archive"
  end

  test "raw execution API does not expose blocked acknowledgements as successful results" do
    task = central_task("raw-pending")
    start_agent(agent_receipt(task, "verified_pending_engine_execution"))

    assert {:error, {:operator_task_execution_blocked, readiness}} =
             OperatorTaskExecutor.execute(task)

    assert readiness["required_action"] == "attach_operator_package_runtime"
  end

  test "only absent readiness uses the legacy compatibility path" do
    task = central_task("legacy-completed")
    receipt = agent_receipt(task, "executed") |> Map.delete("execution_readiness")
    start_agent(receipt)
    {200, payload} = request("execute", %{"task" => task})
    assert payload["status"] == "executed"
    assert payload["execution_readiness"] == OperatorTaskReadiness.local_executed()
    assert payload["result"]["result"] == receipt["result"]
  end

  test "Agent failure receipts remain failures and retain their actual readiness" do
    task = central_task("agent-failed")
    receipt = agent_receipt(task, "failed")
    start_agent(receipt)
    {200, execution} = request("execute-batch", %{"batch" => batch([{"failed", task}])})
    assert execution["status"] == "failed"
    assert execution["executed_count"] == 0
    assert execution["blocked_count"] == 0
    assert execution["error_count"] == 1
    assert hd(execution["results"])["result"] == receipt
    assert hd(execution["results"])["execution_readiness"] == receipt["execution_readiness"]
    assert execution["error_codes"] == ["operator_task_execution_failed"]
  end

  test "checkpoint archival needs task-bound unique results and consistent completion metadata" do
    batch = batch([{"case-a", local_task("a")}, {"case-b", local_task("b")}])
    assert {:ok, complete} = OperatorTaskExecutor.execute_batch(batch)

    assert OperatorTaskBatchRun.checkpoint(batch, execution: complete)["resume_policy"][
             "next_action"
           ] == "archive"

    mutations = [
      Map.put(complete, "results", nil),
      Map.put(complete, "results", [hd(complete["results"]), hd(complete["results"])]),
      put_in(complete, ["results", Access.at(0), "task_digest"], String.duplicate("0", 64)),
      put_in(
        complete,
        ["results", Access.at(0), "execution_readiness", "required_action"],
        "fix"
      ),
      Map.put(complete, "batch_digest", String.duplicate("0", 64)),
      Map.put(complete, "attempted_count", 1),
      Map.put(complete, "blocked_count", 1),
      Map.put(complete, "skipped_count", 1),
      Map.put(complete, "error_code_counts", %{"failed" => 1}),
      Map.put(complete, "readiness_counts", %{"blocked" => 2}),
      Map.put(complete, "status", "blocked"),
      Map.put(complete, "run_phase", "prepare")
    ]

    for execution <- mutations do
      checkpoint = OperatorTaskBatchRun.checkpoint(batch, execution: execution)
      refute checkpoint["resume_policy"]["next_action"] == "archive"
      assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)
      refute plan["target_case_ids"] == []
    end
  end

  defp request(path, body) do
    response =
      :post
      |> conn("/api/v1/operator-tasks/" <> path, Jason.encode!(body))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {response.status, Jason.decode!(response.resp_body)}
  end

  defp local_task(id) do
    {:ok, task} =
      OperatorTaskIR.build("export.summary_json", %{"value" => 14.0}, %{}, task_id: id)

    task
  end

  defp central_task(id) do
    local_task(id)
    |> put_in(["execution_program", "package_version"], "0.1.0")
    |> put_in(["runtime_hints", "package_version"], "0.1.0")
    |> then(fn task ->
      put_in(task, ["integrity", "task_digest"], OperatorTaskIR.compute_task_digest(task))
    end)
  end

  defp batch(cases) do
    %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => Enum.map(cases, fn {id, task} -> %{"case_id" => id, "task_ir" => task} end)
    }
  end

  defp agent_receipt(task, status) do
    %{
      "task_id" => task["task_id"],
      "task_digest" => get_in(task, ["integrity", "task_digest"]),
      "operator_id" => get_in(task, ["operator", "id"]),
      "program_id" => get_in(task, ["execution_program", "program_id"]),
      "operator_task_ir_status" => status,
      "execution_readiness" =>
        if(status == "executed",
          do: OperatorTaskReadiness.local_executed(),
          else: %{
            "status" => "blocked",
            "ready_to_dispatch" => false,
            "requested_mode" => "execute",
            "current_stage" => "fetch_package",
            "blocking_stage" => "fetch_package",
            "required_action" => "attach_operator_package_runtime"
          }
        ),
      "result" => if(status == "executed", do: %{"summary" => %{"sum" => 14.0}}, else: nil)
    }
  end

  defp start_agent(receipt) do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link({:capture, self(), [%{"ok" => true, "result" => receipt}]})

    assert_receive {:fake_agent_ready, port}, 1_000

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "completion-agent",
          host: "127.0.0.1",
          port: port,
          methods: ["run_operator_task_ir"],
          capabilities: ["workflow_export_runtime"],
          tags: ["export"],
          operator_package_runtime: %{ready: true}
        }
      ]
    )

    AgentPool.reload()
  end
end
