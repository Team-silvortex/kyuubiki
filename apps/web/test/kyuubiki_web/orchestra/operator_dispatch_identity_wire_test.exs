defmodule KyuubikiWeb.Orchestra.OperatorDispatchIdentityWireTest do
  use ExUnit.Case, async: false

  alias KyuubikiWeb.Orchestra.OperatorDispatchCancellation, as: Cancellation
  alias KyuubikiWeb.Orchestra.OperatorDispatchInspection, as: Inspection
  alias KyuubikiWeb.Orchestra.OperatorDispatchJournal, as: Journal
  alias KyuubikiWeb.TestSupport.ControlledSolverAgent

  setup do
    root =
      Path.join(System.tmp_dir!(), "dispatch-identity-wire-#{System.unique_integer([:positive])}")

    journal = start_supervised!({Journal, root: root, name: nil})
    on_exit(fn -> File.rm_rf!(root) end)
    start_supervised!({ControlledSolverAgent, {self(), %{"stage" => "querying"}}})
    assert_receive {:fake_agent_ready, port}
    endpoint = %{id: "owner", host: "127.0.0.1", port: port, agent_session_id: "owner-session"}
    digest = String.duplicate("a", 64)

    request = %{
      "id" => "original-rpc",
      "params" => %{
        "job_id" => "job",
        "task_ir" => %{
          "task_id" => "task",
          "integrity" => %{"task_digest" => digest},
          "operator" => %{"id" => "op"},
          "execution_program" => %{"program_id" => "op"}
        }
      }
    }

    {:ok, attempt} = Journal.begin_dispatch(request, endpoint, journal)
    :ok = Journal.finish_dispatch(attempt, "outcome_unknown", journal)
    query = %{"task_id" => "task", "task_digest" => digest, "attempt_id" => attempt}
    path = Path.join(root, attempt <> ".json")
    %{journal: journal, endpoint: endpoint, query: query, path: path}
  end

  for action <- [:inspect, :cancel],
      fault <- [
        :valid,
        :float,
        :exponent,
        :out_of_range,
        :duplicate,
        :malformed,
        :scalar,
        :disconnect
      ] do
    @tag action: action, fault: fault
    test "#{action} raw #{fault} reply preserves unknown history and cancellation scope", ctx do
      before = File.read!(ctx.path)
      call = Task.async(fn -> invoke(ctx.action, ctx) end)
      assert_receive {:solver_request, agent, request}, 2_000

      expected_method = if ctx.action == :inspect, do: "describe_agent", else: "cancel_execution"
      assert request["method"] == expected_method
      if ctx.action == :cancel, do: assert(request["params"] === target())
      if ctx.action == :inspect, do: assert(request["params"] == %{})

      if ctx.fault == :disconnect do
        send(agent, :disconnect)
      else
        send(agent, {:raw_reply, raw_reply(request["id"], ctx.action, ctx.fault)})
      end

      assert {:ok, result} = Task.await(call)
      verify(ctx.action, ctx.fault, result)
      refute inspect(result) =~ "SECRET"
      refute result["automatic_replay_authorized"]
      assert File.read!(ctx.path) == before
      assert {:ok, [%{"state" => "outcome_unknown"}]} = Journal.lookup_task("task", ctx.journal)
      refute_receive {:solver_request, _, _}, 20
    end
  end

  defp invoke(:inspect, ctx),
    do:
      Inspection.inspect_task(Map.take(ctx.query, ~w(task_id task_digest)),
        journal: ctx.journal,
        endpoints: [ctx.endpoint]
      )

  defp invoke(:cancel, ctx),
    do:
      Cancellation.cancel(Map.put(ctx.query, "execution_target", target()),
        journal: ctx.journal,
        endpoints: [ctx.endpoint]
      )

  defp verify(:inspect, fault, result) do
    assert [attempt] = result["attempts"]
    observation = attempt["observation"]

    expected =
      case fault do
        :valid -> "original_endpoint_reports_active_request"
        :disconnect -> "original_endpoint_unreachable"
        _ -> "agent_observation_invalid"
      end

    assert observation["status"] == expected
    assert observation["outcome"] == "unknown"
    refute result["terminal_result_available"]

    if fault == :valid,
      do: assert(observation["execution_target"] === target()),
      else: assert(is_nil(observation["execution_target"]))
  end

  defp verify(:cancel, fault, result) do
    assert result["delivery_attempted"]

    for field <-
          ~w(execution_terminal_confirmed journal_mutation_performed publication_performed job_wide_fallback_performed),
        do: refute(result[field])

    if fault == :valid do
      assert result["status"] == "requested"
      assert result["cancel_registered"]
      assert result["agent_acknowledgement"]["execution_target"] === target()
    else
      assert result["status"] == "cancellation_outcome_unknown"

      assert result["uncertainty_reason"] ==
               if(fault == :disconnect,
                 do: "original_endpoint_unreachable",
                 else: "agent_acknowledgement_invalid"
               )

      assert is_nil(result["cancel_registered"])
      assert is_nil(result["agent_acknowledgement"])
    end
  end

  defp raw_reply(id, action, fault) do
    payload = Jason.encode!(payload(action))

    payload =
      case fault do
        :float ->
          String.replace(payload, ~S("generation":3), ~S("generation":3.0))

        :exponent ->
          String.replace(payload, ~S("generation":3), ~S("generation":3e0))

        :out_of_range ->
          String.replace(payload, ~S("generation":3), ~S("generation":18446744073709551616))

        :duplicate ->
          String.replace(payload, ~S("generation":3), ~S("generation":3,"generation":4))

        :scalar ->
          "false"

        _ ->
          payload
      end

    if fault == :malformed,
      do: ~s({"rpc_version":1,"id":#{Jason.encode!(id)},"SECRET":"PRIVATE",),
      else: ~s({"rpc_version":1,"id":#{Jason.encode!(id)},"ok":true,"result":#{payload}})
  end

  defp target,
    do: %{
      "process_instance_id" => "owner-process",
      "request_id" => "original-rpc",
      "job_id" => "job",
      "generation" => 3
    }

  defp payload(:inspect),
    do: %{
      "lifecycle" => %{"process_instance_id" => target()["process_instance_id"]},
      "solver_control" => %{
        "schema_version" => "kyuubiki.agent-solver-control/v1",
        "available" => true,
        "active" => [Map.put(target(), "cancel_requested", false)]
      }
    }

  defp payload(:cancel),
    do: %{
      "schema_version" => "kyuubiki.agent-execution-cancellation/v1",
      "execution_target" => target(),
      "status" => "requested",
      "cancel_registered" => true,
      "execution_terminal_confirmed" => false,
      "pending_cancellation_created" => false,
      "operator_package_cleanup_performed" => false,
      "automatic_replay_authorized" => false
    }
end
