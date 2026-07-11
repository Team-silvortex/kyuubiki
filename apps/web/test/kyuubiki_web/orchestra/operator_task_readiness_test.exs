defmodule KyuubikiWeb.Orchestra.OperatorTaskReadinessTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorTaskReadiness

  test "normalizes legacy detached runtime results into blocked readiness" do
    result =
      OperatorTaskReadiness.normalize_agent_result(%{
        "operator_task_ir_status" => "verified_pending_engine_execution",
        "requested_mode" => "execute",
        "blocked_stage" => "fetch_package",
        "execution_runtime_status" => "operator_package_runtime_not_yet_attached",
        "operator_package_runtime_ready" => false
      })

    assert result["execution_readiness"] == %{
             "status" => "blocked",
             "requested_mode" => "execute",
             "ready_to_dispatch" => false,
             "current_stage" => "fetch_package",
             "blocking_stage" => "fetch_package",
             "blocking_reason" => "operator_package_runtime_not_yet_attached",
             "blocking_owner" => "operator_package_runtime",
             "required_action" => "attach_operator_package_runtime"
           }
  end

  test "normalizes legacy attached runtime results into package resolution readiness" do
    result =
      OperatorTaskReadiness.normalize_agent_result(%{
        "operator_task_ir_status" => "verified_pending_engine_execution",
        "requested_mode" => "execute",
        "next_stage" => "fetch_package",
        "blocked_stage" => nil,
        "operator_package_runtime_ready" => true
      })

    assert result["execution_readiness"]["status"] == "ready_for_package_resolution"
    assert result["execution_readiness"]["blocking_stage"] == nil

    assert result["execution_readiness"]["required_action"] ==
             "resolve_fetch_verify_and_activate_package"
  end

  test "preserves runtime supplied readiness payloads" do
    readiness = %{
      "status" => "executed",
      "ready_to_dispatch" => true,
      "current_stage" => "serialize_result"
    }

    result =
      OperatorTaskReadiness.normalize_agent_result(%{
        "operator_task_ir_status" => "executed",
        "execution_readiness" => readiness
      })

    assert result["execution_readiness"] == readiness
  end
end
