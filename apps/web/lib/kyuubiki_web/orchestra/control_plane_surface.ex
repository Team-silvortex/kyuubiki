defmodule KyuubikiWeb.Orchestra.ControlPlaneSurface do
  @moduledoc """
  Compact, testable surface for orchestra control-plane responsibilities.

  The control plane coordinates solver work, validation, benchmark lanes, and
  headless handoff contracts without becoming the solver runtime itself.
  """

  @schema_version "kyuubiki.orchestra-control-plane-surface/v1"

  @solver_routes [
    %{
      id: "workflow_graph_execution",
      entrypoint: "KyuubikiWeb.Orchestra.Engine.execute_workflow_graph/5",
      execution_contract: "kyuubiki.workflow-graph/v1",
      delegated_runtime: "workflow_operator_runtime"
    },
    %{
      id: "operator_task_ir_execution",
      entrypoint: "KyuubikiWeb.Orchestra.OperatorTaskExecutor.execute/1",
      execution_contract: "kyuubiki.operator-task-ir/v1",
      delegated_runtime: "operator_task_runtime"
    },
    %{
      id: "operator_task_batch_execution",
      entrypoint: "KyuubikiWeb.Orchestra.OperatorTaskExecutor.execute_batch/2",
      execution_contract: "kyuubiki.operator_task_batch_execution/v1",
      delegated_runtime: "operator_task_runtime"
    }
  ]

  @validation_gates [
    %{
      id: "workflow_graph_shape",
      contract: "kyuubiki.workflow-graph/v1",
      failure_mode: "reject_request"
    },
    %{
      id: "operator_task_digest",
      contract: "kyuubiki.operator-task-ir/v1",
      failure_mode: "reject_task"
    },
    %{
      id: "operator_task_batch_entry_rpc_mirror",
      contract: "kyuubiki.quality_execution_batch/v1",
      failure_mode: "reject_batch_entry"
    }
  ]

  @benchmark_lanes [
    %{
      id: "workflow_catalog",
      command: "mix test test/kyuubiki_web/benchmark/workflow_catalog_report_test.exs",
      evidence_scope: "catalog workflow execution and retained report shape"
    },
    %{
      id: "workflow_large_graph",
      command: "mix test test/kyuubiki_web/benchmark/workflow_large_graph_report_test.exs",
      evidence_scope: "large workflow graph orchestration response shaping"
    },
    %{
      id: "operator_task_batch",
      command: "mix test test/kyuubiki_web/orchestra/operator_task_batch_contract_test.exs",
      evidence_scope: "batched task admission, execution summary, and failure counts"
    }
  ]

  @headless_routes [
    %{
      id: "control_plane_api",
      transport: "http",
      parity_contract: "headless_sdk_control_plane"
    },
    %{
      id: "operator_task_ir_rpc",
      transport: "solver_rpc",
      parity_contract: "run_operator_task_ir"
    }
  ]

  @spec surface() :: map()
  def surface do
    %{
      schema_version: @schema_version,
      owner: "orchestra-control-plane",
      solver_execution_routes: @solver_routes,
      validation_gates: @validation_gates,
      benchmark_lanes: @benchmark_lanes,
      headless_routes: @headless_routes
    }
  end

  @spec validates_surface?() :: boolean()
  def validates_surface? do
    executable_contracts =
      @solver_routes
      |> Enum.map(& &1.execution_contract)
      |> MapSet.new()

    validation_contracts = MapSet.put(executable_contracts, "kyuubiki.quality_execution_batch/v1")

    Enum.all?(@validation_gates, fn gate ->
      MapSet.member?(validation_contracts, gate.contract)
    end)
  end

  @spec benchmark_commands() :: [String.t()]
  def benchmark_commands, do: Enum.map(@benchmark_lanes, & &1.command)

  @spec headless_parity_contracts() :: [String.t()]
  def headless_parity_contracts, do: Enum.map(@headless_routes, & &1.parity_contract)
end
