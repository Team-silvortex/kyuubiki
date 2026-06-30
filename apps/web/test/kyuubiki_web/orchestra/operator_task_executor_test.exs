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

    assert {:error, :missing_operator_task_input} =
             OperatorTaskExecutor.execute(%{
               "schema_version" => "kyuubiki.operator-task-ir/v1",
               "operator" => %{
                 "id" => "transform.rank_material_candidates",
                 "kind" => "transform"
               }
             })
  end
end
