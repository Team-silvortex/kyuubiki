defmodule KyuubikiWeb.AgentNativeTaskApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.Orchestra.OperatorTaskEnvelope
  alias KyuubikiWeb.Orchestra.OperatorTaskReadiness
  alias KyuubikiWeb.Playground.AgentClient

  test "Agent-native TaskIR does not require an attached package runtime" do
    task = native_task()
    receipt = receipt(task)
    start_agent(receipt)

    assert {:ok, ^receipt} = AgentClient.run_operator_task_ir(task, mode: :execute)
    assert_receive {:fake_agent_request, request}
    assert request["method"] == "run_operator_task_ir"
    assert request["params"]["task_ir"] == task
    assert request["params"]["mode"] == "execute"
  end

  test "execute API preserves native Agent TaskIR identity and receipt instead of legacy solve RPC" do
    task = native_task()
    receipt = receipt(task)
    start_agent(receipt)
    response = request(task)

    assert response.status == 200
    payload = Jason.decode!(response.resp_body)
    assert payload["status"] == "executed"
    assert payload["result"] == receipt
    assert_receive {:fake_agent_request, request}
    assert request["method"] == "run_operator_task_ir"

    assert Regex.match?(~r/\A[0-9a-f]{32}\z/, request["params"]["dispatch_attempt_id"])

    assert Map.delete(request["params"], "dispatch_attempt_id") == %{
             "task_ir" => task,
             "mode" => "execute",
             "job_id" => task["task_id"]
           }
  end

  test "native receipt validation still rejects a stale task identity" do
    task = native_task()
    start_agent(Map.put(receipt(task), "task_id", "other-task"))
    response = request(task)
    assert response.status == 422

    assert Jason.decode!(response.resp_body)["error_code"] ==
             "operator_task_execution_receipt_invalid"
  end

  test "central package TaskIR still requires package readiness and never contacts a detached Agent" do
    {:ok, task} = OperatorTaskIR.build("export.summary_json", %{"value" => 14.0}, %{})
    start_agent(receipt(task))

    assert {:error, {:no_matching_agent, %{required_operator_package_runtime: true}}} =
             AgentClient.run_operator_task_ir(task, mode: :execute)

    refute_receive {:fake_agent_request, _request}, 50
  end

  test "native tasks with a package reference do not bypass package readiness" do
    task =
      native_task()
      |> put_in(["execution_program", "package_ref"], "bundle://native/bar")
      |> put_in(["operator", "execution", "package_ref"], "bundle://native/bar")
      |> then(fn task ->
        put_in(task, ["integrity", "task_digest"], OperatorTaskIR.compute_task_digest(task))
      end)

    start_agent(receipt(task))

    assert {:error, {:no_matching_agent, %{required_operator_package_runtime: true}}} =
             AgentClient.run_operator_task_ir(task, mode: :execute)

    refute_receive {:fake_agent_request, _request}, 50
  end

  test "invalid native fetch declarations are rejected before contacting the Agent" do
    task = native_task()
    start_agent(receipt(task))

    for fetchable <- [true, nil, "false", "true", 0, 1, [], %{}] do
      invalid =
        task
        |> put_in(["runtime_hints", "agent_fetchable"], fetchable)
        |> then(fn task ->
          put_in(task, ["integrity", "task_digest"], OperatorTaskIR.compute_task_digest(task))
        end)

      assert {:error, {:operator_task_admission_rejected, report}} =
               AgentClient.run_operator_task_ir(invalid, mode: :execute)

      if not is_boolean(fetchable) do
        assert Enum.any?(report["violations"], &(&1["code"] == "agent_fetchable_missing"))
      end
    end

    refute_receive {:fake_agent_request, _request}, 50
  end

  test "native failure receipt retains task identity and engine recovery without penalizing Agent health" do
    task = native_task()
    failure = failure_receipt(task)
    start_agent_response(failure_response(failure))
    response = request(task)

    assert response.status == 200
    payload = Jason.decode!(response.resp_body)
    assert payload["status"] == "failed"
    assert payload["failure_receipt"] == failure
    assert payload["result"]["operator_task_ir_status"] == "failed"
    assert payload["result"]["failure_receipt"] == failure
    assert payload["task_digest"] == get_in(task, ["integrity", "task_digest"])
    assert payload["execution_readiness"]["current_stage"] == "dispatch_engine_solver"
    assert payload["execution_readiness"]["required_action"] == "inspect_engine_solver_failure"
    assert_receive {:fake_agent_request, _request}
    assert [%{consecutive_failures: 0}] = AgentPool.endpoints()
  end

  test "stale or malformed native failure receipts fail closed without reflecting rejected values" do
    task = native_task()
    valid = failure_receipt(task)

    cases =
      Enum.map(
        ~w(task_id task_digest operator_id schema_version failure_owner reason_code),
        fn field ->
          {field, Map.put(valid, field, "rejected-private-value")}
        end
      ) ++
        [
          {"program_id", Map.put(valid, "program_id", "rejected-private-value")},
          {"failure_stage", Map.put(valid, "failure_stage", "")},
          {"message", Map.put(valid, "message", String.duplicate("x", 4097))},
          {"recovery", Map.put(valid, "recovery", nil)},
          {"recovery.retryable", put_in(valid, ["recovery", "retryable"], "false")},
          {"recovery.safe_to_continue_other_tasks",
           put_in(valid, ["recovery", "safe_to_continue_other_tasks"], 1)},
          {"recovery.required_action", put_in(valid, ["recovery", "required_action"], nil)},
          {"task_id", Map.delete(valid, "task_id")},
          {"failure_receipt", nil}
        ]

    for {field, failure} <- cases do
      start_agent_response(failure_response(failure))
      response = request(task)
      assert response.status == 422, field
      payload = Jason.decode!(response.resp_body)
      assert payload["error_code"] == "operator_task_execution_receipt_invalid", field
      assert payload["error"] =~ field
      refute payload["error"] =~ "rejected-private-value"
      assert_receive {:fake_agent_request, _request}
      assert [%{consecutive_failures: 0}] = AgentPool.endpoints()
    end
  end

  test "native failure recovery survives batch checkpoint and safe-continuation policy" do
    task = native_task()
    {:ok, local} = OperatorTaskIR.build("export.summary_json", %{"value" => 14.0}, %{})

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "bad", "task_ir" => task},
        %{"case_id" => "independent", "task_ir" => local}
      ]
    }

    for safe? <- [true, false] do
      failure = put_in(failure_receipt(task), ["recovery", "safe_to_continue_other_tasks"], safe?)
      start_agent_response(failure_response(failure))
      assert {:ok, execution} = OperatorTaskEnvelope.execute_batch(%{"batch" => batch})
      assert execution["error_count"] == 1
      assert execution["executed_count"] == if(safe?, do: 1, else: 0)
      assert execution["skipped_case_ids"] == if(safe?, do: [], else: ["independent"])
      assert execution["results"] |> hd() |> Map.get("failure_receipt") == failure
      assert execution["error_codes"] == ["operator_task_solver_execution_failed"]

      assert {:ok, checkpoint} =
               OperatorTaskEnvelope.checkpoint_batch(%{
                 "batch" => batch,
                 "execution" => execution
               })

      assert {:ok, plan} =
               OperatorTaskEnvelope.resume_plan_batch(%{
                 "batch" => batch,
                 "checkpoint" => checkpoint
               })

      assert "inspect_engine_solver_failure" in plan["recovery_actions"]
      assert_receive {:fake_agent_request, _request}
    end
  end

  test "native recovery distinguishes manual retry from repair and unsafe continuation without replay" do
    task = native_task()
    {:ok, local} = OperatorTaskIR.build("export.summary_json", %{"value" => 14.0}, %{})

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "bad", "task_ir" => task},
        %{"case_id" => "peer", "task_ir" => local}
      ]
    }

    for retryable <- [true, false], safe <- [true, false] do
      failure =
        failure_receipt(task)
        |> put_in(["recovery", "retryable"], retryable)
        |> put_in(["recovery", "safe_to_continue_other_tasks"], safe)

      start_agent_response(failure_response(failure))
      assert {200, execution} = batch_request("execute-batch", %{"batch" => batch})
      assert_receive {:fake_agent_request, _request}

      assert {200, checkpoint} =
               batch_request("checkpoint-batch", %{"batch" => batch, "execution" => execution})

      assert {200, plan} =
               batch_request("resume-plan-batch", %{"batch" => batch, "checkpoint" => checkpoint})

      assert plan["recovery_evidence_verified"] == true
      assert plan["automatic_replay_allowed"] == false
      assert plan["continuation_blocked"] == not safe
      assert plan["retryable_case_ids"] == if(retryable, do: ["bad"], else: [])
      assert plan["repair_required_case_ids"] == if(retryable, do: [], else: ["bad"])

      assert plan["next_action"] ==
               if(retryable and safe, do: "retry_failed_cases", else: "resolve_incomplete_cases")

      assert plan["target_case_ids"] == if(safe, do: ["bad"], else: ["bad", "peer"])

      for {field, value} <- [
            {"retryable", not retryable},
            {"safe_to_continue_other_tasks", not safe},
            {"required_action", "rejected-private-value"}
          ] do
        changed =
          checkpoint
          |> put_in(["execution", "failure_receipts", Access.at(0), "recovery", field], value)
          |> rehash_checkpoint()

        for route <- ~w(verify-checkpoint-batch resume-plan-batch) do
          assert {422, rejected} =
                   batch_request(route, %{"batch" => batch, "checkpoint" => changed})

          assert rejected["error_code"] == "operator_task_checkpoint_evidence_invalid"
          refute rejected["error"] =~ "rejected-private-value"
        end
      end

      assert {200, ^plan} =
               batch_request("resume-plan-batch", %{"batch" => batch, "checkpoint" => checkpoint})

      for {path, value} <- [
            {["result", "task_id"], "wrong"},
            {["result", "operator_task_ir_status"], "executed"},
            {["result", "failure_receipt"], nil},
            {["result", "failure_receipt", "recovery", "retryable"], not retryable},
            {["result"], nil}
          ] do
        invalid = put_in(execution, ["results", Access.at(0)] ++ path, value)

        assert {200, diagnostic} =
                 batch_request("checkpoint-batch", %{"batch" => batch, "execution" => invalid})

        assert {200, blocked} =
                 batch_request("resume-plan-batch", %{
                   "batch" => batch,
                   "checkpoint" => diagnostic
                 })

        assert blocked["recovery_evidence_verified"] == false
        assert blocked["next_action"] == "resolve_incomplete_cases"
        assert blocked["recovery_actions"] == []
        assert blocked["retryable_case_ids"] == []
        assert blocked["automatic_replay_allowed"] == false
      end

      refute_receive {:fake_agent_request, _request}, 20
    end
  end

  defp batch_request(route, body) do
    response =
      conn(:post, "/api/v1/operator-tasks/" <> route, Jason.encode!(body))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    {response.status, Jason.decode!(response.resp_body)}
  end

  defp rehash_checkpoint(checkpoint) do
    digest =
      checkpoint
      |> Map.delete("checkpoint_digest")
      |> legacy_json()
      |> KyuubikiWeb.CanonicalJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    Map.put(checkpoint, "checkpoint_digest", digest)
  end

  defp legacy_json(value) when is_map(value),
    do: Map.new(value, fn {key, nested} -> {key, legacy_json(nested)} end)

  defp legacy_json([]), do: %{}
  defp legacy_json(value) when is_list(value), do: Enum.map(value, &legacy_json/1)
  defp legacy_json(value), do: value

  test "generic Agent RPC errors keep the existing application-error contract" do
    task = native_task()

    start_agent_response(%{
      "ok" => false,
      "error" => %{"code" => "invalid_params", "message" => "bad model"}
    })

    assert {:error, {:rpc_error, "invalid_params", "bad model"}} =
             AgentClient.run_operator_task_ir(task, mode: :execute)

    assert_receive {:fake_agent_request, _request}
    assert [%{consecutive_failures: 0}] = AgentPool.endpoints()
  end

  test "successful RPC envelopes cannot smuggle stale failure or contradictory recovery advice" do
    task = native_task()
    failure = failure_receipt(task)

    result =
      receipt(task)
      |> Map.put("operator_task_ir_status", "failed")
      |> Map.put("failure_receipt", failure)
      |> Map.put("execution_readiness", %{
        "status" => "blocked",
        "requested_mode" => "execute",
        "ready_to_dispatch" => false,
        "current_stage" => "dispatch_engine_solver",
        "blocking_stage" => "dispatch_engine_solver",
        "blocking_reason" => "operator_task_solver_execution_failed",
        "blocking_owner" => "agent_runtime",
        "required_action" => "inspect_engine_solver_failure"
      })

    for invalid <- [
          put_in(result, ["failure_receipt", "task_id"], "rejected-private-value"),
          put_in(result, ["execution_readiness", "required_action"], "rejected-private-value")
        ] do
      start_agent(invalid)
      response = request(task)
      assert response.status == 422
      refute response.resp_body =~ "rejected-private-value"
      assert_receive {:fake_agent_request, _request}
    end

    start_agent(result)
    response = request(task)
    assert response.status == 200
    assert Jason.decode!(response.resp_body)["failure_receipt"] == failure
    assert_receive {:fake_agent_request, _request}
  end

  defp failure_receipt(task) do
    %{
      "schema_version" => "kyuubiki.agent-operator-task-failure/v1",
      "failure_owner" => "agent_runtime",
      "failure_stage" => "dispatch_engine_solver",
      "task_id" => task["task_id"],
      "task_digest" => get_in(task, ["integrity", "task_digest"]),
      "operator_id" => get_in(task, ["operator", "id"]),
      "reason_code" => "operator_task_solver_execution_failed",
      "message" => "invalid bar area",
      "recovery" => %{
        "retryable" => false,
        "required_action" => "inspect_engine_solver_failure",
        "safe_to_continue_other_tasks" => true
      }
    }
  end

  defp failure_response(failure) do
    %{
      "ok" => false,
      "error" => %{
        "code" => "operator_task_solver_execution_failed",
        "message" => "invalid bar area",
        "details" => %{"operator_task_failure_receipt" => failure}
      }
    }
  end

  defp request(task) do
    conn(:post, "/api/v1/operator-tasks/execute", Jason.encode!(%{"task" => task}))
    |> put_req_header("content-type", "application/json")
    |> Router.call(@opts)
  end

  defp native_task do
    {:ok, task} =
      OperatorTaskIR.build(
        "solve.bar_1d",
        %{
          "length" => 1.0,
          "area" => 0.01,
          "youngs_modulus" => 210.0e9,
          "elements" => 4,
          "tip_force" => 1000.0
        },
        %{},
        task_id: "native-api-bar"
      )

    task
    |> put_in(["execution_program", "package_ref"], nil)
    |> put_in(["operator", "execution", "package_ref"], nil)
    |> Map.put("runtime_hints", %{
      "authority_mode" => "agent_local",
      "execution_mode" => "agent_native",
      "agent_fetchable" => false,
      "cache_scope" => "none",
      "operator_kind" => "solver",
      "required_capabilities" => ["solver_rpc"]
    })
    |> then(fn task ->
      put_in(task, ["integrity", "task_digest"], OperatorTaskIR.compute_task_digest(task))
    end)
  end

  defp receipt(task) do
    %{
      "task_id" => task["task_id"],
      "task_digest" => get_in(task, ["integrity", "task_digest"]),
      "operator_id" => get_in(task, ["operator", "id"]),
      "program_id" => get_in(task, ["execution_program", "program_id"]),
      "operator_task_ir_status" => "executed",
      "execution_readiness" => OperatorTaskReadiness.local_executed(),
      "result" => %{"tip_displacement" => 4.761904761904762e-7}
    }
  end

  defp start_agent(receipt) do
    start_agent_response(%{"ok" => true, "result" => receipt})
  end

  defp start_agent_response(response) do
    {:ok, _pid} =
      FakePlaygroundAgent.start_link({:capture, self(), [response]})

    port = await_fake_agent_port()

    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [
        %{
          id: "native-task-agent",
          host: "127.0.0.1",
          port: port,
          methods: ["run_operator_task_ir"],
          capabilities: ["solver_rpc"],
          operator_package_runtime: %{"ready" => false, "status" => "not_attached"}
        }
      ]
    )

    AgentPool.reload()
  end
end
