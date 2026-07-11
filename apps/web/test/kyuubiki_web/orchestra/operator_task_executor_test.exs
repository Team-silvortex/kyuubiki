defmodule KyuubikiWeb.Orchestra.OperatorTaskExecutorTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun
  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  test "executes a transform operator task IR locally" do
    assert {:ok, task} =
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

    assert {:ok, result} = OperatorTaskExecutor.execute(task)

    assert result["material_thermal_shock_candidate_count"] == 1
    assert result["material_thermal_shock_status"] == "pass"
    assert result["material_thermal_shock_safety_factor"] > 1.0
  end

  test "executes an export operator task IR locally" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "export.summary_json",
               %{"material_status" => "pass", "material_safety_factor" => 1.5},
               %{}
             )

    assert {:ok, result} = OperatorTaskExecutor.execute(task)

    assert result["format"] == "json"
    assert result["content_type"] == "application/json"
    assert Jason.decode!(result["content"])["material_status"] == "pass"
  end

  test "executes a quality task batch locally" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)
    assert {:ok, task_b} = material_shock_task("case-b", 160.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a},
        %{"case_id" => "case-b", "task_ir" => task_b}
      ]
    }

    assert {:ok, result} = OperatorTaskExecutor.execute_batch(batch)

    assert result["operator_task_batch_execution_contract"] ==
             "kyuubiki.operator_task_batch_execution/v1"

    assert result["task_count"] == 2
    assert result["executed_count"] == 2
    assert result["ok_count"] == 2
    assert result["error_count"] == 0
    assert result["run_phase"] == "execute"
    assert result["run_id"] =~ "operator-task-batch:execute:"
    assert result["batch_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert result["digest_algorithm"] == "sha256"
    assert is_binary(result["started_at"])
    assert is_binary(result["finished_at"])
    assert result["readiness_counts"] == %{"executed" => 2}
    assert Enum.map(result["results"], & &1["case_id"]) == ["case-a", "case-b"]
    assert hd(result["results"])["result"]["material_thermal_shock_status"] == "pass"
    assert hd(result["results"])["execution_readiness"]["status"] == "executed"
    assert hd(result["results"])["execution_readiness"]["current_stage"] == "serialize_result"
  end

  test "prepares a quality task batch with replayable run metadata" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "operator_id" => "transform.evaluate_material_thermal_shock",
      "task_count" => 1,
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a}
      ]
    }

    assert {:ok, preparation} = OperatorTaskExecutor.prepare_batch(batch)

    assert preparation["operator_task_batch_preparation_contract"] ==
             "kyuubiki.operator_task_batch_preparation/v1"

    assert preparation["run_phase"] == "prepare"
    assert preparation["run_id"] =~ "operator-task-batch:prepare:"
    assert preparation["batch_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert preparation["verified_count"] == 1
    assert preparation["error_count"] == 0
    assert hd(preparation["summaries"])["status"] == "verified"
  end

  test "uses the same batch digest for preparation and execution phases" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a}
      ]
    }

    assert {:ok, preparation} = OperatorTaskExecutor.prepare_batch(batch)

    assert {:ok, execution} =
             OperatorTaskExecutor.execute_batch(batch,
               started_at: "2026-01-01T00:00:00Z",
               finished_at: "2026-01-01T00:00:01Z"
             )

    assert preparation["batch_digest"] == execution["batch_digest"]
    assert preparation["run_id"] != execution["run_id"]
    assert execution["started_at"] == "2026-01-01T00:00:00Z"
    assert execution["finished_at"] == "2026-01-01T00:00:01Z"
  end

  test "builds a resumable checkpoint manifest from batch run summaries" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a}
      ]
    }

    assert {:ok, preparation} = OperatorTaskExecutor.prepare_batch(batch)

    checkpoint =
      OperatorTaskBatchRun.checkpoint(batch,
        preparation: preparation,
        created_at: "2026-01-01T00:00:02Z"
      )

    assert checkpoint["operator_task_batch_checkpoint_contract"] ==
             "kyuubiki.operator_task_batch_checkpoint/v1"

    assert checkpoint["batch_digest"] == preparation["batch_digest"]
    assert checkpoint["checkpoint_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert checkpoint["task_count"] == 1
    assert hd(checkpoint["case_index"])["case_id"] == "case-a"
    assert checkpoint["preparation"]["verified_count"] == 1
    assert checkpoint["execution"] == nil
    assert checkpoint["resume_policy"] == %{"status" => "prepared", "next_action" => "execute"}
  end

  test "checkpoint resume policy distinguishes complete and partial executions" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a}
      ]
    }

    assert {:ok, execution} = OperatorTaskExecutor.execute_batch(batch)

    complete = OperatorTaskBatchRun.checkpoint(batch, execution: execution)

    assert complete["resume_policy"] == %{"status" => "complete", "next_action" => "archive"}

    partial_execution = %{execution | "error_count" => 1, "ok_count" => 0}
    partial = OperatorTaskBatchRun.checkpoint(batch, execution: partial_execution)

    assert partial["resume_policy"] == %{
             "status" => "partial",
             "next_action" => "retry_failed_cases"
           }
  end

  test "checkpoint resume policy targets readiness-blocked cases separately from errors" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a}
      ]
    }

    execution = %{
      "run_id" => "operator-task-batch:execute:test",
      "run_phase" => "execute",
      "batch_digest" => OperatorTaskBatchRun.batch_digest(batch),
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
          "case_id" => "case-a",
          "execution_readiness" => %{"status" => "blocked"}
        }
      ]
    }

    checkpoint = OperatorTaskBatchRun.checkpoint(batch, execution: execution)

    assert checkpoint["resume_policy"] == %{
             "status" => "blocked",
             "next_action" => "resolve_blocked_cases"
           }

    assert checkpoint["execution"]["readiness_counts"] == %{"blocked" => 1}
    assert checkpoint["execution"]["blocked_readiness_case_ids"] == ["case-a"]

    assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)
    assert plan["next_action"] == "resolve_blocked_cases"
    assert plan["target_case_ids"] == ["case-a"]
    assert plan["blocked_case_ids"] == ["case-a"]
  end

  test "builds resume plans from verified checkpoints" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a}
      ]
    }

    assert {:ok, preparation} = OperatorTaskExecutor.prepare_batch(batch)
    checkpoint = OperatorTaskBatchRun.checkpoint(batch, preparation: preparation)

    assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)

    assert plan["operator_task_batch_resume_plan_contract"] ==
             "kyuubiki.operator_task_batch_resume_plan/v1"

    assert plan["next_action"] == "execute"
    assert plan["target_case_ids"] == ["case-a"]
    assert plan["blocked_case_ids"] == []
    assert plan["batch_digest"] == checkpoint["batch_digest"]
    assert plan["checkpoint_digest"] == checkpoint["checkpoint_digest"]
  end

  test "resume plans target only failed cases for partial executions" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)
    assert {:ok, task_b} = material_shock_task("case-b", 160.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a},
        %{"case_id" => "case-b", "task_ir" => task_b}
      ]
    }

    execution = %{
      "run_id" => "operator-task-batch:execute:test",
      "run_phase" => "execute",
      "batch_digest" => OperatorTaskBatchRun.batch_digest(batch),
      "started_at" => "2026-01-01T00:00:00Z",
      "finished_at" => "2026-01-01T00:00:01Z",
      "task_count" => 2,
      "executed_count" => 2,
      "ok_count" => 1,
      "error_count" => 1,
      "failed_case_ids" => ["case-b"]
    }

    checkpoint = OperatorTaskBatchRun.checkpoint(batch, execution: execution)

    assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)

    assert plan["next_action"] == "retry_failed_cases"
    assert plan["target_case_ids"] == ["case-b"]
    assert plan["blocked_case_ids"] == []
  end

  test "records task batch failures without hiding the failed case" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    bad_task =
      task_a
      |> put_in(["input_artifact", "temperature_delta"], 999.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a},
        %{"case_id" => "case-b", "task_ir" => bad_task}
      ]
    }

    assert {:ok, result} = OperatorTaskExecutor.execute_batch(batch)

    assert result["ok_count"] == 1
    assert result["error_count"] == 1
    assert result["error_codes"] == ["operator_task_digest_mismatch"]
    assert result["error_code_counts"] == %{"operator_task_digest_mismatch" => 1}
    assert result["readiness_counts"] == %{"blocked" => 1, "executed" => 1}
    assert result["failed_case_ids"] == ["case-b"]
    assert List.last(result["results"])["case_id"] == "case-b"
    assert List.last(result["results"])["status"] == "error"
    assert List.last(result["results"])["error"] =~ "operator_task_digest_mismatch"
    assert List.last(result["results"])["error_code"] == "operator_task_digest_mismatch"
    assert List.last(result["results"])["execution_readiness"]["status"] == "blocked"

    assert List.last(result["results"])["execution_readiness"]["blocking_stage"] ==
             "local_execute"
  end

  test "rejects task batch headers that disagree with the task list" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "operator_id" => "transform.evaluate_material_thermal_shock",
      "task_count" => 2,
      "tasks" => [
        %{"case_id" => "case-a", "task_ir" => task_a}
      ]
    }

    assert {:error, {:operator_task_batch_count_mismatch, mismatch}} =
             OperatorTaskExecutor.execute_batch(batch)

    assert mismatch.declared == 2
    assert mismatch.actual == 1
  end

  test "records task batch entry contract failures before execution" do
    assert {:ok, task_a} = material_shock_task("case-a", 120.0)
    mirrored = put_in(task_a, ["config", "constraint_factor"], 0.3)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{
          "case_id" => "case-a",
          "task_id" => task_a["task_id"],
          "task_digest" => get_in(task_a, ["integrity", "task_digest"]),
          "operator_id" => get_in(task_a, ["operator", "id"]),
          "task_ir" => task_a,
          "agent_rpc" => %{
            "method" => "run_operator_task_ir",
            "params" => %{"task_ir" => mirrored}
          }
        }
      ]
    }

    assert {:ok, result} = OperatorTaskExecutor.execute_batch(batch)
    assert result["ok_count"] == 0
    assert result["error_count"] == 1
    assert result["error_codes"] == ["operator_task_batch_entry_rpc_mirror_mismatch"]

    assert result["error_code_counts"] == %{
             "operator_task_batch_entry_rpc_mirror_mismatch" => 1
           }

    assert hd(result["results"])["error"] =~ "operator_task_batch_entry_rpc_mirror_mismatch"

    assert hd(result["results"])["error_code"] ==
             "operator_task_batch_entry_rpc_mirror_mismatch"
  end

  test "rejects malformed task IR envelopes" do
    assert {:error, :invalid_operator_task_ir} = OperatorTaskExecutor.execute(%{})

    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{}
             )

    assert {:error, :missing_operator_task_digest} =
             task
             |> update_in(["integrity"], &Map.delete(&1, "task_digest"))
             |> OperatorTaskExecutor.execute()

    assert {:error, :missing_operator_task_input} =
             task
             |> Map.delete("input_artifact")
             |> refresh_task_digest()
             |> OperatorTaskExecutor.execute()
  end

  test "rejects task IR whose digest no longer matches the envelope" do
    assert {:ok, task} =
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

    tampered = put_in(task, ["config", "constraint_factor"], 0.2)

    assert {:error, {:operator_task_digest_mismatch, mismatch}} =
             OperatorTaskExecutor.execute(tampered)

    assert mismatch.expected == task["integrity"]["task_digest"]
    assert mismatch.actual == OperatorTaskIR.compute_task_digest(tampered)
  end

  test "rejects task IR without an execution program" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{}
             )

    assert {:error, :missing_operator_execution_program} =
             task
             |> Map.delete("execution_program")
             |> refresh_task_digest()
             |> OperatorTaskExecutor.execute()
  end

  test "rejects execution programs that do not match the operator snapshot" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{}
             )

    mismatched_id =
      put_in(task, ["execution_program", "program_id"], "transform.other_operator")
      |> refresh_task_digest()

    assert {:error, :operator_task_program_mismatch} =
             OperatorTaskExecutor.execute(mismatched_id)

    mismatched_protocol =
      put_in(task, ["execution_program", "runtime_protocol"], "kyuubiki.solver-rpc/v1")
      |> refresh_task_digest()

    assert {:error, :operator_task_execution_abi_mismatch} =
             OperatorTaskExecutor.execute(mismatched_protocol)

    mismatched_entrypoint =
      put_in(task, ["execution_program", "entrypoint", "name"], "transform.other_operator")
      |> refresh_task_digest()

    assert {:error, :operator_task_entrypoint_mismatch} =
             OperatorTaskExecutor.execute(mismatched_entrypoint)
  end

  test "rejects digest-valid task IR with inconsistent mirrored identity fields" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{}
             )

    mismatched_package_hint =
      task
      |> put_in(["runtime_hints", "package_ref"], "orchestra://operator-package/wrong")
      |> refresh_task_digest()

    assert {:error,
            {:operator_task_mirror_mismatch,
             %{
               source: "execution_program.package_ref",
               mirror: "runtime_hints.package_ref"
             }}} = OperatorTaskExecutor.execute(mismatched_package_hint)

    mismatched_kind_hint =
      task
      |> put_in(["runtime_hints", "operator_kind"], "solver")
      |> refresh_task_digest()

    assert {:error,
            {:operator_task_mirror_mismatch,
             %{
               source: "operator.kind",
               mirror: "runtime_hints.operator_kind"
             }}} = OperatorTaskExecutor.execute(mismatched_kind_hint)
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
