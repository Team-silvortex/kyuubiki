defmodule KyuubikiSdk.OperatorTasksTest do
  use ExUnit.Case, async: false

  alias KyuubikiSdk.ControlPlaneClient

  setup do
    parent = self()

    {:ok, listener} =
      :gen_tcp.listen(0, [:binary, packet: 0, active: false, reuseaddr: true])

    {:ok, port} = :inet.port(listener)

    acceptor =
      spawn_link(fn ->
        {:ok, socket} = :gen_tcp.accept(listener)
        {:ok, request} = read_http_request(socket)
        send(parent, {:request, request})

        body = Jason.encode!(response_for(request))

        response =
          "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: #{byte_size(body)}\r\nconnection: close\r\n\r\n#{body}"

        :ok = :gen_tcp.send(socket, response)
        :gen_tcp.close(socket)
      end)

    on_exit(fn ->
      Process.exit(acceptor, :kill)
      :gen_tcp.close(listener)
    end)

    {:ok, base_url: "http://127.0.0.1:#{port}"}
  end

  test "control plane client executes operator task batch", %{base_url: base_url} do
    client = ControlPlaneClient.new(base_url)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{
          "case_id" => "case-a",
          "task_ir" => %{
            "schema_version" => "kyuubiki.operator-task-ir/v1",
            "task_id" => "task-a"
          }
        }
      ]
    }

    assert {:ok, result} = ControlPlaneClient.execute_operator_task_batch(client, batch)

    assert result["status"] == "executed"
    assert result["ok_count"] == 1
    assert result["error_codes"] == []
    assert result["error_code_counts"] == %{}

    assert get_in(result, ["results", Access.at(0), "result", "material_thermal_shock_status"]) ==
             "pass"

    assert_receive {:request, request}
    [headers, body] = String.split(request, "\r\n\r\n", parts: 2)
    assert headers =~ "POST /api/v1/operator-tasks/execute-batch"
    assert Jason.decode!(body) == %{"batch" => batch}
  end

  test "control plane client prepares operator task batch", %{base_url: base_url} do
    client = ControlPlaneClient.new(base_url)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{
          "case_id" => "case-a",
          "task_ir" => %{
            "schema_version" => "kyuubiki.operator-task-ir/v1",
            "task_id" => "task-a"
          }
        }
      ]
    }

    assert {:ok, result} = ControlPlaneClient.prepare_operator_task_batch(client, batch)

    assert result["status"] == "verified"
    assert result["verified_count"] == 1
    assert result["error_codes"] == []
    assert result["error_code_counts"] == %{}
    assert get_in(result, ["summaries", Access.at(0), "case_id"]) == "case-a"

    assert_receive {:request, request}
    [headers, body] = String.split(request, "\r\n\r\n", parts: 2)
    assert headers =~ "POST /api/v1/operator-tasks/prepare-batch"
    assert Jason.decode!(body) == %{"batch" => batch}
  end

  test "extracts nested operator task failure receipts" do
    payload = %{
      "status" => "failed",
      "results" => [
        %{
          "case_id" => "case-a",
          "failure_receipt" => %{
            "schema_version" => "kyuubiki.headless-operator-task-failure/v1",
            "failure_stage" => "summarize_execution_program",
            "recovery" => %{"required_action" => "fix_task_ir_contract_mirror_fields"}
          }
        },
        %{
          "case_id" => "case-b",
          "error" => %{
            "details" => %{
              "operator_task_failure_receipt" => %{
                "schema_version" => "kyuubiki.agent-operator-task-failure/v1",
                "failure_stage" => "verify_digest",
                "recovery" => %{"required_action" => "rebuild_task_ir_and_recompute_digest"}
              }
            }
          }
        },
        %{
          "case_id" => "case-c",
          "failure_receipt" => %{
            "schema_version" => "kyuubiki.control-plane-operator-task-failure/v1",
            "failure_stage" => "validate_batch_entry",
            "recovery" => %{"required_action" => "fix_quality_execution_batch_entry"}
          }
        }
      ],
      "resume_plan" => %{
        "next_action" => "retry_failed_cases",
        "target_case_ids" => ["case-a", "case-c"],
        "blocked_case_ids" => ["case-b"],
        "recovery_actions" => [
          "fix_quality_execution_batch_entry",
          "inspect_operator_task_batch_checkpoint"
        ]
      }
    }

    receipts = KyuubikiSdk.operator_task_failure_receipts(payload)

    assert Enum.map(receipts, & &1["failure_stage"]) == [
             "summarize_execution_program",
             "verify_digest",
             "validate_batch_entry"
           ]

    assert KyuubikiSdk.operator_task_failure_actions(payload) == [
             "fix_task_ir_contract_mirror_fields",
             "rebuild_task_ir_and_recompute_digest",
             "fix_quality_execution_batch_entry",
             "inspect_operator_task_batch_checkpoint"
           ]

    assert KyuubikiSdk.operator_task_recovery_summary(payload) == %{
             "next_action" => "retry_failed_cases",
             "target_case_ids" => ["case-a", "case-c"],
             "blocked_case_ids" => ["case-b"],
             "recovery_actions" => [
               "fix_task_ir_contract_mirror_fields",
               "rebuild_task_ir_and_recompute_digest",
               "fix_quality_execution_batch_entry",
               "inspect_operator_task_batch_checkpoint"
             ],
             "failure_receipt_count" => 3,
             "failure_receipts" => receipts
           }
  end

  test "control plane client checkpoints operator task batch", %{base_url: base_url} do
    client = ControlPlaneClient.new(base_url)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{
          "case_id" => "case-a",
          "task_ir" => %{
            "schema_version" => "kyuubiki.operator-task-ir/v1",
            "task_id" => "task-a"
          }
        }
      ]
    }

    preparation = %{"run_id" => "prepare-run", "batch_digest" => String.duplicate("a", 64)}

    assert {:ok, result} =
             ControlPlaneClient.checkpoint_operator_task_batch(client, batch,
               preparation: preparation
             )

    assert result["operator_task_batch_checkpoint_contract"] ==
             "kyuubiki.operator_task_batch_checkpoint/v1"

    assert get_in(result, ["resume_policy", "next_action"]) == "execute"

    assert_receive {:request, request}
    [headers, body] = String.split(request, "\r\n\r\n", parts: 2)
    assert headers =~ "POST /api/v1/operator-tasks/checkpoint-batch"
    assert Jason.decode!(body) == %{"batch" => batch, "preparation" => preparation}
  end

  test "control plane client verifies operator task batch checkpoint", %{base_url: base_url} do
    client = ControlPlaneClient.new(base_url)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{
          "case_id" => "case-a",
          "task_ir" => %{
            "schema_version" => "kyuubiki.operator-task-ir/v1",
            "task_id" => "task-a"
          }
        }
      ]
    }

    checkpoint = %{
      "operator_task_batch_checkpoint_contract" => "kyuubiki.operator_task_batch_checkpoint/v1",
      "batch_digest" => String.duplicate("a", 64),
      "checkpoint_digest" => String.duplicate("b", 64)
    }

    assert {:ok, result} =
             ControlPlaneClient.verify_operator_task_batch_checkpoint(client, batch, checkpoint)

    assert result["operator_task_batch_checkpoint_verification_contract"] ==
             "kyuubiki.operator_task_batch_checkpoint_verification/v1"

    assert result["status"] == "verified"

    assert_receive {:request, request}
    [headers, body] = String.split(request, "\r\n\r\n", parts: 2)
    assert headers =~ "POST /api/v1/operator-tasks/verify-checkpoint-batch"
    assert Jason.decode!(body) == %{"batch" => batch, "checkpoint" => checkpoint}
  end

  test "control plane client plans operator task batch resume", %{base_url: base_url} do
    client = ControlPlaneClient.new(base_url)

    batch = %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => [
        %{
          "case_id" => "case-a",
          "task_ir" => %{
            "schema_version" => "kyuubiki.operator-task-ir/v1",
            "task_id" => "task-a"
          }
        }
      ]
    }

    checkpoint = %{
      "operator_task_batch_checkpoint_contract" => "kyuubiki.operator_task_batch_checkpoint/v1",
      "batch_digest" => String.duplicate("a", 64),
      "checkpoint_digest" => String.duplicate("b", 64)
    }

    assert {:ok, result} =
             ControlPlaneClient.plan_operator_task_batch_resume(client, batch, checkpoint)

    assert result["operator_task_batch_resume_plan_contract"] ==
             "kyuubiki.operator_task_batch_resume_plan/v1"

    assert result["target_case_ids"] == ["case-a"]

    assert_receive {:request, request}
    [headers, body] = String.split(request, "\r\n\r\n", parts: 2)
    assert headers =~ "POST /api/v1/operator-tasks/resume-plan-batch"
    assert Jason.decode!(body) == %{"batch" => batch, "checkpoint" => checkpoint}
  end

  defp response_for(request) do
    cond do
      request =~ "POST /api/v1/operator-tasks/prepare-batch" ->
        %{
          "status" => "verified",
          "operator_task_batch_preparation_contract" =>
            "kyuubiki.operator_task_batch_preparation/v1",
          "task_count" => 1,
          "verified_count" => 1,
          "error_count" => 0,
          "error_codes" => [],
          "error_code_counts" => %{},
          "summaries" => [%{"case_id" => "case-a", "status" => "verified"}]
        }

      request =~ "POST /api/v1/operator-tasks/checkpoint-batch" ->
        %{
          "operator_task_batch_checkpoint_contract" =>
            "kyuubiki.operator_task_batch_checkpoint/v1",
          "batch_digest" => String.duplicate("a", 64),
          "checkpoint_digest" => String.duplicate("b", 64),
          "resume_policy" => %{"status" => "prepared", "next_action" => "execute"},
          "case_index" => [%{"case_id" => "case-a"}]
        }

      request =~ "POST /api/v1/operator-tasks/verify-checkpoint-batch" ->
        %{
          "operator_task_batch_checkpoint_verification_contract" =>
            "kyuubiki.operator_task_batch_checkpoint_verification/v1",
          "status" => "verified",
          "batch_digest" => String.duplicate("a", 64),
          "checkpoint_digest" => String.duplicate("b", 64),
          "resume_policy" => %{"status" => "prepared", "next_action" => "execute"}
        }

      request =~ "POST /api/v1/operator-tasks/resume-plan-batch" ->
        %{
          "operator_task_batch_resume_plan_contract" =>
            "kyuubiki.operator_task_batch_resume_plan/v1",
          "next_action" => "execute",
          "target_case_ids" => ["case-a"],
          "blocked_case_ids" => []
        }

      true ->
        %{
          "status" => "executed",
          "operator_task_batch_execution_contract" => "kyuubiki.operator_task_batch_execution/v1",
          "task_count" => 1,
          "ok_count" => 1,
          "error_count" => 0,
          "error_codes" => [],
          "error_code_counts" => %{},
          "results" => [
            %{
              "case_id" => "case-a",
              "task_id" => "task-a",
              "status" => "ok",
              "result" => %{"material_thermal_shock_status" => "pass"}
            }
          ]
        }
    end
  end

  defp read_http_request(socket, acc \\ "") do
    with {:ok, chunk} <- :gen_tcp.recv(socket, 0, 1_000) do
      next = acc <> chunk

      case request_complete?(next) do
        true -> {:ok, next}
        false -> read_http_request(socket, next)
      end
    end
  end

  defp request_complete?(request) do
    case String.split(request, "\r\n\r\n", parts: 2) do
      [headers, body] ->
        content_length =
          headers
          |> String.split("\r\n")
          |> Enum.find_value(0, fn line ->
            case String.split(line, ":", parts: 2) do
              [key, value] when key in ["Content-Length", "content-length"] ->
                value |> String.trim() |> String.to_integer()

              _ ->
                nil
            end
          end)

        byte_size(body) >= content_length

      _ ->
        false
    end
  end
end
