defmodule KyuubikiWeb.Orchestra.OperatorTaskBatchContractTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun
  alias KyuubikiWeb.Orchestra.OperatorTaskIR
  alias KyuubikiWeb.WorkflowOperatorRuntime

  test "quality execution batch runtime output matches the shared batch contract shape" do
    required_fields =
      schema_path("operator-task-batch.schema.json")
      |> File.read!()
      |> Jason.decode!()
      |> Map.fetch!("required")

    payload = %{
      "cases" => [
        %{
          "id" => "quality_case_a",
          "parameters" => %{"thermal_load" => 12.0},
          "model" => heat_model(12.0)
        }
      ]
    }

    assert {:ok, batch} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_quality_execution_batch",
               payload,
               %{
                 "operator_id" => "solve.heat_plane_quad_2d",
                 "task_id_prefix" => "quality-contract",
                 "dataset_contract" => %{"id" => "kyuubiki.dataset.quality_contract/v1"},
                 "required_capabilities" => ["solver:thermal"]
               }
             )

    assert Map.take(batch, required_fields) |> map_size() == length(required_fields)
    assert batch["quality_execution_batch_contract"] == "kyuubiki.quality_execution_batch/v1"
    assert batch["task_count"] == length(batch["tasks"])
    assert batch["agent_rpc_method"] == "run_operator_task_ir"

    [entry] = batch["tasks"]
    [index_entry] = batch["case_index"]
    task_ir = entry["task_ir"]

    assert index_entry == Map.take(entry, ["case_id", "task_id", "task_digest"])
    assert entry["task_digest"] == get_in(task_ir, ["integrity", "task_digest"])
    assert entry["task_digest"] == OperatorTaskIR.compute_task_digest(task_ir)
    assert get_in(entry, ["agent_rpc", "method"]) == "run_operator_task_ir"
    assert get_in(entry, ["agent_rpc", "params", "task_ir"]) == task_ir
    assert entry["agent_rpc"]["routing_opts"][:required_capabilities] == ["solver:thermal"]
    assert get_in(task_ir, ["runtime_hints", "required_capabilities"]) == ["solver:thermal"]
  end

  test "operator task batch example keeps TaskIR digest and RPC mirror aligned" do
    example =
      schema_path("examples.operator-task-batch.json")
      |> File.read!()
      |> Jason.decode!()

    assert example["$schema"] == "operator-task-batch.schema.json"
    assert example["quality_execution_batch_contract"] == "kyuubiki.quality_execution_batch/v1"
    assert example["task_count"] == length(example["tasks"])

    [entry] = example["tasks"]
    task_ir = entry["task_ir"]

    assert entry["task_digest"] == get_in(task_ir, ["integrity", "task_digest"])
    assert entry["task_digest"] == OperatorTaskIR.compute_task_digest(task_ir)
    assert get_in(entry, ["agent_rpc", "params", "task_ir"]) == task_ir
    assert hd(example["case_index"]) == Map.take(entry, ["case_id", "task_id", "task_digest"])
  end

  test "batch preparation output matches the shared preparation contract shape" do
    required_fields =
      schema_path("operator-task-batch-preparation.schema.json")
      |> File.read!()
      |> Jason.decode!()
      |> Map.fetch!("required")

    assert {:ok, batch} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_quality_execution_batch",
               %{
                 "cases" => [
                   %{
                     "id" => "quality_case_a",
                     "parameters" => %{"thermal_load" => 12.0},
                     "model" => heat_model(12.0)
                   }
                 ]
               },
               %{
                 "operator_id" => "solve.heat_plane_quad_2d",
                 "task_id_prefix" => "quality-contract",
                 "dataset_contract" => %{"id" => "kyuubiki.dataset.quality_contract/v1"},
                 "required_capabilities" => ["solver:thermal"]
               }
             )

    assert {:ok, preparation} = OperatorTaskExecutor.prepare_batch(batch)

    assert Map.take(preparation, required_fields) |> map_size() == length(required_fields)

    assert preparation["operator_task_batch_preparation_contract"] ==
             "kyuubiki.operator_task_batch_preparation/v1"

    assert preparation["quality_execution_batch_contract"] ==
             "kyuubiki.quality_execution_batch/v1"

    assert preparation["run_phase"] == "prepare"
    assert preparation["run_id"] =~ "operator-task-batch:prepare:"
    assert preparation["batch_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert preparation["digest_algorithm"] == "sha256"
    assert is_binary(preparation["started_at"])
    assert is_binary(preparation["finished_at"])
    assert preparation["task_count"] == batch["task_count"]
    assert preparation["verified_count"] == 1
    assert preparation["error_count"] == 0

    [summary] = preparation["summaries"]
    [entry] = batch["tasks"]

    assert summary["case_id"] == entry["case_id"]
    assert summary["task_id"] == entry["task_id"]
    assert summary["task_digest"] == entry["task_digest"]
    assert summary["operator_id"] == batch["operator_id"]
    assert summary["status"] == "verified"
  end

  test "operator task batch preparation example keeps counters aligned" do
    example =
      schema_path("examples.operator-task-batch-preparation.json")
      |> File.read!()
      |> Jason.decode!()

    assert example["$schema"] == "operator-task-batch-preparation.schema.json"

    assert example["operator_task_batch_preparation_contract"] ==
             "kyuubiki.operator_task_batch_preparation/v1"

    assert example["quality_execution_batch_contract"] == "kyuubiki.quality_execution_batch/v1"
    assert example["run_phase"] == "prepare"
    assert example["run_id"] =~ "operator-task-batch:prepare:"
    assert example["batch_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert example["digest_algorithm"] == "sha256"
    assert example["task_count"] == length(example["summaries"])
    assert example["verified_count"] + example["error_count"] == example["task_count"]
    assert hd(example["summaries"])["status"] == "verified"
  end

  test "operator task batch checkpoint matches the shared checkpoint contract shape" do
    required_fields =
      schema_path("operator-task-batch-checkpoint.schema.json")
      |> File.read!()
      |> Jason.decode!()
      |> Map.fetch!("required")

    assert {:ok, batch} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_quality_execution_batch",
               %{
                 "cases" => [
                   %{
                     "id" => "quality_case_a",
                     "parameters" => %{"thermal_load" => 12.0},
                     "model" => heat_model(12.0)
                   }
                 ]
               },
               %{
                 "operator_id" => "solve.heat_plane_quad_2d",
                 "task_id_prefix" => "quality-contract",
                 "dataset_contract" => %{"id" => "kyuubiki.dataset.quality_contract/v1"},
                 "required_capabilities" => ["solver:thermal"]
               }
             )

    assert {:ok, preparation} = OperatorTaskExecutor.prepare_batch(batch)

    checkpoint =
      OperatorTaskBatchRun.checkpoint(batch,
        preparation: preparation,
        created_at: "2026-01-01T00:00:02Z"
      )

    assert Map.take(checkpoint, required_fields) |> map_size() == length(required_fields)

    assert checkpoint["operator_task_batch_checkpoint_contract"] ==
             "kyuubiki.operator_task_batch_checkpoint/v1"

    assert checkpoint["batch_digest"] == preparation["batch_digest"]
    assert checkpoint["checkpoint_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert checkpoint["task_count"] == batch["task_count"]
    assert checkpoint["case_index"] == batch["case_index"]
    assert checkpoint["preparation"]["run_id"] == preparation["run_id"]
    assert checkpoint["execution"] == nil
    assert checkpoint["resume_policy"] == %{"status" => "prepared", "next_action" => "execute"}
  end

  test "operator task batch checkpoint example keeps counters aligned" do
    example =
      schema_path("examples.operator-task-batch-checkpoint.json")
      |> File.read!()
      |> Jason.decode!()

    assert example["$schema"] == "operator-task-batch-checkpoint.schema.json"

    assert example["operator_task_batch_checkpoint_contract"] ==
             "kyuubiki.operator_task_batch_checkpoint/v1"

    assert example["batch_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert example["checkpoint_digest"] =~ ~r/^[a-f0-9]{64}$/
    assert example["task_count"] == length(example["case_index"])

    assert example["resume_policy"]["next_action"] in [
             "prepare",
             "execute",
             "archive",
             "retry_failed_cases",
             "fix_invalid_cases"
           ]
  end

  test "operator task batch resume plan matches the shared resume-plan contract shape" do
    required_fields =
      schema_path("operator-task-batch-resume-plan.schema.json")
      |> File.read!()
      |> Jason.decode!()
      |> Map.fetch!("required")

    assert {:ok, batch} =
             WorkflowOperatorRuntime.run_transform_operator(
               "transform.compose_quality_execution_batch",
               %{
                 "cases" => [
                   %{
                     "id" => "quality_case_a",
                     "parameters" => %{"thermal_load" => 12.0},
                     "model" => heat_model(12.0)
                   }
                 ]
               },
               %{
                 "operator_id" => "solve.heat_plane_quad_2d",
                 "task_id_prefix" => "quality-contract",
                 "dataset_contract" => %{"id" => "kyuubiki.dataset.quality_contract/v1"},
                 "required_capabilities" => ["solver:thermal"]
               }
             )

    assert {:ok, preparation} = OperatorTaskExecutor.prepare_batch(batch)
    checkpoint = OperatorTaskBatchRun.checkpoint(batch, preparation: preparation)

    assert {:ok, plan} = OperatorTaskBatchRun.resume_plan(batch, checkpoint)

    assert Map.take(plan, required_fields) |> map_size() == length(required_fields)

    assert plan["operator_task_batch_resume_plan_contract"] ==
             "kyuubiki.operator_task_batch_resume_plan/v1"

    assert plan["next_action"] == "execute"
    assert plan["target_case_ids"] == ["quality_case_a"]
    assert plan["blocked_case_ids"] == []
  end

  test "operator task batch resume plan example keeps action lists aligned" do
    example =
      schema_path("examples.operator-task-batch-resume-plan.json")
      |> File.read!()
      |> Jason.decode!()

    assert example["$schema"] == "operator-task-batch-resume-plan.schema.json"

    assert example["operator_task_batch_resume_plan_contract"] ==
             "kyuubiki.operator_task_batch_resume_plan/v1"

    assert example["next_action"] == example["resume_policy"]["next_action"]
    assert is_list(example["target_case_ids"])
    assert is_list(example["blocked_case_ids"])
  end

  test "material envelope catalog request example matches the shared request contract" do
    required_fields =
      schema_path("material-envelope-catalog-request.schema.json")
      |> File.read!()
      |> Jason.decode!()
      |> Map.fetch!("required")

    example =
      schema_path("examples.material-envelope-catalog-request.json")
      |> File.read!()
      |> Jason.decode!()

    assert example["$schema"] == "material-envelope-catalog-request.schema.json"
    assert Map.take(example, required_fields) |> map_size() == length(required_fields)
    assert example["workflow_id"] == "workflow.material-study-envelope-ranking-json"

    rows = get_in(example, ["input_artifacts", "material_rows", "rows"])
    assert is_list(rows)
    assert length(rows) == 3
    assert Enum.all?(rows, &is_binary(&1["case_id"]))
    assert Enum.all?(rows, &(map_size(&1["summaries"]) > 0))
  end

  defp heat_model(load) do
    %{
      "nodes" => [
        %{"id" => "n1", "x" => 0.0, "y" => 0.0, "temperature" => 293.15, "heat_load" => load},
        %{"id" => "n2", "x" => 1.0, "y" => 0.0, "temperature" => 293.15, "heat_load" => 0.0},
        %{"id" => "n3", "x" => 1.0, "y" => 1.0, "temperature" => 293.15, "heat_load" => 0.0},
        %{"id" => "n4", "x" => 0.0, "y" => 1.0, "temperature" => 293.15, "heat_load" => 0.0}
      ],
      "elements" => [
        %{"id" => "e1", "node_ids" => ["n1", "n2", "n3", "n4"], "conductivity" => 10.0}
      ]
    }
  end

  defp schema_path(file_name), do: Path.expand("../../schemas/#{file_name}", File.cwd!())
end
