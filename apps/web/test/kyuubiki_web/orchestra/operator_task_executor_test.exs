defmodule KyuubikiWeb.Orchestra.OperatorTaskExecutorTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor
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

  defp refresh_task_digest(task) do
    put_in(task, ["integrity", "task_digest"], OperatorTaskIR.compute_task_digest(task))
  end
end
