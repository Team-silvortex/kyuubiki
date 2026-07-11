defmodule KyuubikiWeb.OperatorTaskApiTest do
  use KyuubikiWeb.TestSupport.ApiRouterCase

  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  test "prepares and verifies an operator task IR envelope" do
    task = fixture_task!()

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "verified"
    assert payload["operator_id"] == "transform.api_fixture"
    assert payload["program_id"] == "transform.api_fixture"
    assert payload["program_kind"] == "transform"
    assert payload["runtime_protocol"] == "kyuubiki.operator-execution/v1"
    assert payload["abi_kind"] == "operator_task"
    assert payload["entrypoint_kind"] == "operator_id"
    assert payload["entrypoint_name"] == "transform.api_fixture"
    assert payload["package_ref"] == "orchestra://operator-package/transform.api_fixture"
    assert payload["package_version"] == "library-managed"
    assert payload["authority_mode"] == "central_operator_library"
    assert payload["execution_mode"] == "orchestra_fetch"
    assert payload["cache_scope"] == "job"
    assert payload["agent_fetchable"] == true
    assert payload["task_digest"] == get_in(task, ["integrity", "task_digest"])
  end

  test "rejects tampered operator task IR envelopes" do
    task = put_in(fixture_task!(), ["config", "alpha"], false)

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 422
    payload = Jason.decode!(conn.resp_body)
    assert payload["error"] =~ "operator_task_digest_mismatch"
    assert payload["error_code"] == "operator_task_digest_mismatch"
  end

  test "rejects operator task IR envelopes with mismatched execution ABI" do
    task =
      fixture_task!()
      |> put_in(["execution_program", "abi", "kind"], "solver_rpc")
      |> refresh_task_digest()

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 422
    payload = Jason.decode!(conn.resp_body)
    assert payload["error"] =~ "operator_task_execution_abi_mismatch"
    assert payload["error_code"] == "operator_task_execution_abi_mismatch"
  end

  test "rejects operator task IR envelopes with digest-valid mirrored identity mismatch" do
    task =
      fixture_task!()
      |> put_in(["runtime_hints", "package_ref"], "orchestra://operator-package/wrong")
      |> refresh_task_digest()

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 422
    payload = Jason.decode!(conn.resp_body)
    assert payload["error"] =~ "operator_task_mirror_mismatch"
    assert payload["error_code"] == "operator_task_mirror_mismatch"
  end

  test "executes an operator task IR envelope through the service API" do
    {:ok, task} =
      OperatorTaskIR.build(
        "transform.evaluate_material_thermal_shock",
        %{
          "temperature_delta" => 160.0,
          "thermal_expansion" => 1.2e-5,
          "youngs_modulus" => 70.0e9,
          "poisson_ratio" => 0.33,
          "yield_strength" => 320.0e6
        },
        %{"constraint_factor" => 0.7}
      )

    conn =
      :post
      |> conn("/api/v1/operator-tasks/execute", Jason.encode!(%{"task" => task}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "executed"
    assert payload["operator_id"] == "transform.evaluate_material_thermal_shock"
    assert payload["result"]["material_thermal_shock_status"] == "pass"
  end

  test "prepares an operator task batch through the service API" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)
    {:ok, task_b} = material_shock_task("batch-case-b", 160.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "operator_id" => "transform.evaluate_material_thermal_shock",
      "task_count" => 2,
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => task_a},
        %{"case_id" => "batch-case-b", "task_ir" => task_b}
      ]
    }

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare-batch", Jason.encode!(%{"batch" => batch}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "verified"

    assert payload["operator_task_batch_preparation_contract"] ==
             "kyuubiki.operator_task_batch_preparation/v1"

    assert payload["task_count"] == 2
    assert payload["verified_count"] == 2
    assert payload["error_count"] == 0
    assert Enum.map(payload["summaries"], & &1["case_id"]) == ["batch-case-a", "batch-case-b"]
    assert hd(payload["summaries"])["status"] == "verified"
    assert hd(payload["summaries"])["operator_id"] == "transform.evaluate_material_thermal_shock"
  end

  test "operator task batch preparation exposes per-case error codes" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)

    bad_task =
      task_a
      |> put_in(["runtime_hints", "operator_kind"], "solver")
      |> refresh_task_digest()

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => bad_task}
      ]
    }

    conn =
      :post
      |> conn("/api/v1/operator-tasks/prepare-batch", Jason.encode!(%{"batch" => batch}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "verified"
    assert payload["verified_count"] == 0
    assert payload["error_count"] == 1
    assert payload["error_codes"] == ["operator_task_mirror_mismatch"]
    assert payload["error_code_counts"] == %{"operator_task_mirror_mismatch" => 1}
    assert hd(payload["summaries"])["error_code"] == "operator_task_mirror_mismatch"
  end

  test "executes an operator task batch through the service API" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)
    {:ok, task_b} = material_shock_task("batch-case-b", 160.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => task_a},
        %{"case_id" => "batch-case-b", "task_ir" => task_b}
      ]
    }

    conn =
      :post
      |> conn("/api/v1/operator-tasks/execute-batch", Jason.encode!(%{"batch" => batch}))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)
    assert payload["status"] == "executed"

    assert payload["operator_task_batch_execution_contract"] ==
             "kyuubiki.operator_task_batch_execution/v1"

    assert payload["task_count"] == 2
    assert payload["ok_count"] == 2
    assert payload["error_count"] == 0
    assert Enum.map(payload["results"], & &1["case_id"]) == ["batch-case-a", "batch-case-b"]
  end

  test "builds an operator task batch checkpoint through the service API" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => task_a}
      ]
    }

    preparation =
      batch
      |> KyuubikiWeb.Orchestra.OperatorTaskExecutor.prepare_batch()
      |> elem(1)

    body = %{
      "batch" => batch,
      "preparation" => preparation,
      "created_at" => "2026-01-01T00:00:02Z"
    }

    conn =
      :post
      |> conn("/api/v1/operator-tasks/checkpoint-batch", Jason.encode!(body))
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    assert payload["operator_task_batch_checkpoint_contract"] ==
             "kyuubiki.operator_task_batch_checkpoint/v1"

    assert payload["batch_digest"] == preparation["batch_digest"]
    assert payload["checkpoint_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert payload["resume_policy"] == %{"status" => "prepared", "next_action" => "execute"}
    assert hd(payload["case_index"])["case_id"] == "batch-case-a"
  end

  test "verifies an operator task batch checkpoint through the service API" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => task_a}
      ]
    }

    checkpoint =
      KyuubikiWeb.Orchestra.OperatorTaskBatchRun.checkpoint(batch,
        created_at: "2026-01-01T00:00:02Z"
      )

    conn =
      :post
      |> conn(
        "/api/v1/operator-tasks/verify-checkpoint-batch",
        Jason.encode!(%{"batch" => batch, "checkpoint" => checkpoint})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    assert payload["operator_task_batch_checkpoint_verification_contract"] ==
             "kyuubiki.operator_task_batch_checkpoint_verification/v1"

    assert payload["status"] == "verified"
    assert payload["batch_digest"] == checkpoint["batch_digest"]
    assert payload["checkpoint_digest"] == checkpoint["checkpoint_digest"]
  end

  test "rejects tampered operator task batch checkpoints through the service API" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => task_a}
      ]
    }

    checkpoint =
      batch
      |> KyuubikiWeb.Orchestra.OperatorTaskBatchRun.checkpoint(created_at: "2026-01-01T00:00:02Z")
      |> put_in(["task_count"], 2)

    conn =
      :post
      |> conn(
        "/api/v1/operator-tasks/verify-checkpoint-batch",
        Jason.encode!(%{"batch" => batch, "checkpoint" => checkpoint})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 422
    assert Jason.decode!(conn.resp_body)["error"] =~ "operator_task_batch_checkpoint_mismatch"
  end

  test "builds an operator task batch resume plan through the service API" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => task_a}
      ]
    }

    preparation =
      batch
      |> KyuubikiWeb.Orchestra.OperatorTaskExecutor.prepare_batch()
      |> elem(1)

    checkpoint =
      KyuubikiWeb.Orchestra.OperatorTaskBatchRun.checkpoint(batch, preparation: preparation)

    conn =
      :post
      |> conn(
        "/api/v1/operator-tasks/resume-plan-batch",
        Jason.encode!(%{"batch" => batch, "checkpoint" => checkpoint})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert conn.status == 200
    payload = Jason.decode!(conn.resp_body)

    assert payload["operator_task_batch_resume_plan_contract"] ==
             "kyuubiki.operator_task_batch_resume_plan/v1"

    assert payload["next_action"] == "execute"
    assert payload["target_case_ids"] == ["batch-case-a"]
    assert payload["blocked_case_ids"] == []
  end

  test "builds a readiness-blocked operator task batch resume plan through the service API" do
    {:ok, task_a} = material_shock_task("batch-case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "batch-case-a", "task_ir" => task_a}
      ]
    }

    execution = %{
      "run_id" => "operator-task-batch:execute:test",
      "run_phase" => "execute",
      "batch_digest" => KyuubikiWeb.Orchestra.OperatorTaskBatchRun.batch_digest(batch),
      "started_at" => "2026-01-01T00:00:00Z",
      "finished_at" => "2026-01-01T00:00:01Z",
      "task_count" => 1,
      "executed_count" => 1,
      "ok_count" => 1,
      "error_count" => 0,
      "failed_case_ids" => [],
      "readiness_counts" => %{"blocked" => 1},
      "results" => [
        %{
          "case_id" => "batch-case-a",
          "execution_readiness" => %{"status" => "blocked"}
        }
      ]
    }

    checkpoint_conn =
      :post
      |> conn(
        "/api/v1/operator-tasks/checkpoint-batch",
        Jason.encode!(%{"batch" => batch, "execution" => execution})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert checkpoint_conn.status == 200
    checkpoint = Jason.decode!(checkpoint_conn.resp_body)

    assert checkpoint["resume_policy"] == %{
             "status" => "blocked",
             "next_action" => "resolve_blocked_cases"
           }

    assert checkpoint["execution"]["blocked_readiness_case_ids"] == ["batch-case-a"]

    resume_conn =
      :post
      |> conn(
        "/api/v1/operator-tasks/resume-plan-batch",
        Jason.encode!(%{"batch" => batch, "checkpoint" => checkpoint})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(@opts)

    assert resume_conn.status == 200
    payload = Jason.decode!(resume_conn.resp_body)
    assert payload["next_action"] == "resolve_blocked_cases"
    assert payload["target_case_ids"] == ["batch-case-a"]
    assert payload["blocked_case_ids"] == ["batch-case-a"]
  end

  defp fixture_task! do
    descriptor = %{
      "id" => "transform.api_fixture",
      "family" => "fixture",
      "kind" => "transform",
      "execution" => %{"package_ref" => "orchestra://operator-package/transform.api_fixture"}
    }

    {:ok, task} =
      OperatorTaskIR.build_from_descriptor(
        descriptor,
        %{"x" => 1},
        %{"alpha" => true},
        descriptor_authoring: %{
          "mode" => "rust_native",
          "runtime" => "rust",
          "source" => "api_fixture",
          "hot_reloadable" => false
        },
        task_id: "operator-task-api-fixture"
      )

    task
  end

  defp refresh_task_digest(task) do
    put_in(task, ["integrity", "task_digest"], OperatorTaskIR.compute_task_digest(task))
  end

  defp material_shock_task(task_id, temperature_delta) do
    OperatorTaskIR.build(
      "transform.evaluate_material_thermal_shock",
      %{
        "temperature_delta" => temperature_delta,
        "thermal_expansion" => 1.2e-5,
        "youngs_modulus" => 70.0e9,
        "poisson_ratio" => 0.33,
        "yield_strength" => 320.0e6
      },
      %{"constraint_factor" => 0.7},
      task_id: task_id
    )
  end
end
