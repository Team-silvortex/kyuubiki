defmodule KyuubikiWeb.WorkflowQualityExecutionRecoveryTest do
  use ExUnit.Case, async: false
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture
  alias KyuubikiWeb.WorkflowGraphRunner, as: Runner
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor, as: Executor
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun, as: Run

  test "direct batch rejection stops exports and corrected fresh runs execute with evidence" do
    for evidence <- [
          %{"converged" => false},
          %{"source_ranking_complete" => false},
          %{"sweep_enabled" => false},
          %{"coupled_readiness" => %{"coupled_readiness_state" => "block"}},
          %{"coupled_readiness" => %{"coupled_readiness_state" => "warn"}},
          %{"repair_focus" => %{"domain" => "unknown"}},
          %{"sweep_budget" => %{"case_budget_exceeded" => true}},
          %{"expansion_budget_ready" => false}
        ],
        policy <- ~w(fail skip) do
      graph = Fixture.batch_graph(policy)
      owner = self()
      ref = make_ref()

      options = [
        execute_solve: fn _, _, _ -> flunk("admission must not run a solver") end,
        execute_extract: fn _, _, _ -> flunk("no extraction expected") end,
        execute_transform: &Runtime.run_transform_operator/3,
        execute_export: fn operator, payload, config ->
          send(owner, {ref, :export})
          Runtime.run_export_operator(operator, payload, config)
        end
      ]

      cases = Fixture.expanded_cases()
      bad = put_in(cases, ["cases", Access.at(1), "metadata"], evidence)
      result = Runner.run(graph, %{"input" => bad}, options)

      if policy == "fail" do
        assert {:error, {:workflow_node_error, "batch", reason}} = result
        assert is_binary(reason)
      else
        assert {:ok, rejected} = result
        assert rejected["failed_nodes"] == ["batch"]
        assert rejected["completed_nodes"] == ["input"]

        for key <- ~w(batch.result export.json),
            do: refute(Map.has_key?(rejected["artifacts"], key))
      end

      refute_received {^ref, :export}

      repair = %{
        "coupled_readiness" => %{"coupled_readiness_state" => "block"},
        "optimization_hint" => %{"action" => "fix_coupled_readiness"},
        "repair_strategy" => "repair_coupled_readiness_sweep"
      }

      corrected = put_in(cases, ["cases", Access.at(1), "metadata"], repair)
      assert {:ok, fresh} = Runner.run(graph, %{"input" => corrected}, options)
      assert fresh["failed_nodes"] == []
      assert_received {^ref, :export}
      batch = Jason.decode!(fresh["artifacts"]["export.json"]["content"])
      assert List.last(batch["tasks"])["metadata"] == repair
      assert {:ok, prepared} = Executor.prepare_batch(batch)
      assert prepared["verified_count"] == 2
      assert {:ok, executed} = Executor.execute_batch(batch)
      Fixture.assert_execution(executed)
      checkpoint = Run.checkpoint(batch, execution: executed)
      assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
      assert plan["next_action"] == "archive"
    end
  end

  test "a quality failure anywhere in the batch prevents every task dispatch" do
    assert {:ok, batch} =
             Runtime.run_transform_operator(
               "transform.compose_quality_execution_batch",
               Fixture.expanded_cases(),
               %{
                 "operator_id" => "transform.evaluate_material_thermal_shock",
                 "operator_config" => %{"constraint_factor" => 0.7}
               }
             )

    # Trace only this worker; the production execution path is not replaced by a mock.
    owner = self()
    worker = spawn_link(fn -> execute_requests(owner) end)
    :erlang.trace_pattern({Executor, :execute_receipt, 2}, true, [:local])
    :erlang.trace(worker, true, [:call, {:tracer, owner}])

    try do
      for index <- [0, 1], strict <- [false, true] do
        other = Enum.at(batch["tasks"], 1 - index)

        invalid_batches = [
          {put_in(batch, ["tasks", Access.at(index), "metadata"], %{"sweep_enabled" => false}),
           :operator_task_batch_quality_rejected},
          {put_in(batch, ["tasks", Access.at(index), "task_ir", "operator"], 42),
           :invalid_operator_task_batch_structure},
          {put_in(batch, ["tasks", Access.at(index), "case_id"], other["case_id"]),
           :operator_task_batch_identity_conflict},
          {put_in(batch, ["tasks", Access.at(index), "task_ir", "task_id"], other["task_id"]),
           :operator_task_batch_identity_conflict}
        ]

        for {invalid, code} <- invalid_batches do
          send(worker, {:execute, invalid, [strict: strict]})
          assert_receive {^worker, {:error, reason}}
          assert elem(reason, 0) == code
          drain_trace_barrier(worker)
          refute_received {:trace, ^worker, :call, {Executor, :execute_receipt, _}}
        end
      end

      send(worker, {:execute, batch, []})
      assert_receive {^worker, {:ok, execution}}, 1_000
      Fixture.assert_execution(execution)
      drain_trace_barrier(worker)

      for _ <- 1..2,
          do: assert_received({:trace, ^worker, :call, {Executor, :execute_receipt, _}})

      refute_received {:trace, ^worker, :call, {Executor, :execute_receipt, _}}
    after
      :erlang.trace(worker, false, [:call])
      :erlang.trace_pattern({Executor, :execute_receipt, 2}, false, [:local])
      send(worker, :stop)
    end
  end

  test "ordinary per-task numerical failure remains isolated and retryable by case" do
    cases =
      put_in(Fixture.expanded_cases(), ["cases", Access.at(1), "model", "youngs_modulus"], -1)

    assert {:ok, batch} =
             Runtime.run_transform_operator(
               "transform.compose_quality_execution_batch",
               cases,
               %{"operator_id" => "transform.evaluate_material_thermal_shock"}
             )

    assert {:ok, execution} = Executor.execute_batch(batch)
    assert execution["ok_count"] == 1
    assert execution["error_count"] == 1
    assert execution["failed_case_ids"] == ["case-1"]
    checkpoint = Run.checkpoint(batch, execution: execution)
    assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
    assert plan["target_case_ids"] == ["case-1"]
    assert plan["next_action"] == "retry_failed_cases"
  end

  defp execute_requests(owner) do
    receive do
      {:execute, batch, opts} ->
        send(owner, {self(), Executor.execute_batch(batch, opts)})
        execute_requests(owner)

      :stop ->
        :ok
    end
  end

  defp drain_trace_barrier(worker) do
    ref = :erlang.trace_delivered(worker)
    assert_receive {:trace_delivered, ^worker, ^ref}
  end
end
