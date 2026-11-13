defmodule KyuubikiWeb.Orchestra.PackagedSolverTaskTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorExecutionProgram
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutionSummary
  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  test "external solver retains its semantic kind but uses the package ABI" do
    task = task()
    assert task["execution_program"]["program_kind"] == "solver"
    assert task["execution_program"]["abi"]["kind"] == "operator_task"
    assert task["execution_program"]["entrypoint"]["name"] == "solve.external_q9"
    assert {:ok, summary} = OperatorTaskExecutionSummary.build(task)
    assert summary["admission_report"]["accepted"]
  end

  test "external solver ABI rejects missing package identity and mismatched entrypoint" do
    task = task()

    for package_ref <- [nil, "builtin://q9", "https://example.invalid/q9", "bundle://"] do
      bad = put_in(task, ["execution_program", "package_ref"], package_ref)
      bad = put_in(bad, ["operator", "execution", "package_ref"], package_ref)
      bad = put_in(bad, ["runtime_hints", "package_ref"], package_ref)

      assert {:error, :operator_task_execution_abi_mismatch} =
               OperatorTaskExecutionSummary.build(bad)
    end

    bad = put_in(task, ["execution_program", "package_version"], nil)
    bad = put_in(bad, ["runtime_hints", "package_version"], nil)

    assert {:error, :operator_task_execution_abi_mismatch} =
             OperatorTaskExecutionSummary.build(bad)

    bad = put_in(task, ["execution_program", "entrypoint", "name"], "solve.other")
    assert {:error, :operator_task_entrypoint_mismatch} = OperatorTaskExecutionSummary.build(bad)
  end

  test "a package cannot shadow the native solver entrypoint" do
    task = task()
    bad = put_in(task, ["operator", "id"], "solve.bar_1d")
    bad = put_in(bad, ["execution_program", "program_id"], "solve.bar_1d")
    bad = put_in(bad, ["execution_program", "entrypoint", "name"], "solve.bar_1d")

    assert {:error, :operator_task_execution_abi_mismatch} =
             OperatorTaskExecutionSummary.build(bad)

    program =
      OperatorExecutionProgram.build(%{"id" => "solve.bar_1d", "kind" => "solver"}, %{
        "package_ref" => "orchestra://operator-package/solve.bar_1d",
        "package_version" => "0.1.0"
      })

    assert program["abi"]["kind"] == "solver_rpc"
  end

  defp task do
    operator = %{
      "id" => "solve.external_q9",
      "family" => "thermal_q9_rect",
      "kind" => "solver",
      "execution" => %{
        "package_ref" => "orchestra://operator-package/solve.external_q9",
        "package_version" => "0.1.0"
      }
    }

    {:ok, task} = OperatorTaskIR.build_from_descriptor(operator, %{}, %{})
    task
  end
end
