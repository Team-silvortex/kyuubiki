defmodule KyuubikiWeb.WorkflowQualityExecutionAdmissionTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.WorkflowOperatorRuntime, as: Runtime
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor, as: Executor
  alias KyuubikiWeb.Orchestra.OperatorTaskEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun, as: Run
  alias KyuubikiWeb.TestSupport.QualitySweepRecovery, as: Fixture

  @invalid [
    {"unconverged", %{"converged" => false}},
    {"invalid_convergence", %{"converged" => "true"}},
    {"failed_stability", %{"stability_result" => %{"converged" => false}}},
    {"invalid_stability", %{"stability_result" => nil}},
    {"disabled", %{"sweep_enabled" => false}},
    {"invalid_enabled", %{"sweep_enabled" => "true"}},
    {"stopped", %{"sweep_action" => "stop"}},
    {"unknown_action", %{"sweep_action" => "ignore"}},
    {"incomplete", %{"source_ranking_complete" => false}},
    {"rejected", %{"source_rejected_candidates" => [%{"id" => "bad"}]}},
    {"invalid_rejections", %{"source_rejected_candidates" => "none"}},
    {"reason", %{"sweep_blocking_reason" => "repair first"}},
    {"blocked", %{"coupled_readiness" => %{"coupled_readiness_state" => "block"}}},
    {"warning", %{"coupled_readiness" => %{"coupled_readiness_state" => "warn"}}},
    {"invalid_report", %{"coupled_readiness" => "ready"}},
    {"invalid_hint", %{"optimization_hint" => "continue"}},
    {"orphan_repair", %{"optimization_hint" => %{"action" => "fix_coupled_readiness"}}},
    {"repair_rejected", %{"optimization_hint" => %{"action" => "repair_rejected_candidates"}}},
    {"wrong_strategy", %{"repair_strategy" => "ignore"}},
    {"wrong_focus", %{"repair_focus" => %{"domain" => "unknown"}}},
    {"budget_block", %{"sweep_budget" => %{"case_budget_exceeded" => true}}},
    {"invalid_budget", %{"sweep_budget" => "unlimited"}},
    {"invalid_budget_flag", %{"sweep_budget" => %{"case_budget_exceeded" => "false"}}},
    {"expansion_disabled", %{"expansion_enabled" => false}},
    {"expansion_not_ready", %{"expansion_budget_ready" => false}},
    {"invalid_expansion_ready", %{"expansion_budget_ready" => "true"}}
  ]

  for {name, evidence} <- @invalid do
    test "direct cases and compiled batches reject #{name} in every evidence scope" do
      evidence = unquote(Macro.escape(evidence))
      cases = cases()
      assert {:ok, batch} = compose(cases)

      for location <- ~w(root root_metadata case_metadata entry entry_metadata) do
        assert {:error, reason} = compose(inject(cases, "cases", location, evidence))
        assert is_binary(reason)

        invalid = inject(batch, "tasks", location, evidence) |> exchange()

        for run <- [&Executor.prepare_batch/1, &Executor.execute_batch/1] do
          assert {:error, {:operator_task_batch_quality_rejected, reason}} = run.(invalid)
          assert is_binary(reason)
        end

        assert {:error, {:operator_task_batch_quality_rejected, _}} =
                 Envelope.prepare_batch(%{"batch" => invalid})

        assert {:error, {:operator_task_batch_quality_rejected, _}} =
                 Envelope.execute_batch(%{"batch" => invalid})
      end
    end
  end

  test "repair and review intent survive composition, preflight, execution and checkpoint" do
    for evidence <- [
          repair(),
          review(),
          %{"coupled_readiness" => %{"coupled_readiness_state" => "ready"}}
        ],
        location <- ~w(root root_metadata case_metadata entry entry_metadata) do
      payload = inject(cases(), "cases", location, evidence)
      assert {:ok, batch} = compose(exchange(payload))
      assert extract(batch, "tasks", location) == evidence
      assert {:ok, prepared} = Executor.prepare_batch(batch)
      assert prepared["verified_count"] == 2
      assert prepared["error_count"] == 0
      assert {:ok, executed} = Executor.execute_batch(batch)
      Fixture.assert_execution(executed)
      assert prepared["batch_digest"] == executed["batch_digest"]
      checkpoint = Run.checkpoint(batch, execution: executed)
      assert {:ok, plan} = Run.resume_plan(batch, checkpoint)
      assert plan["next_action"] == "archive"
    end
  end

  test "changed evidence invalidates the old checkpoint without changing numerical TaskIR" do
    for location <- ~w(root root_metadata case_metadata entry entry_metadata) do
      assert {:ok, batch} = compose(inject(cases(), "cases", location, repair()))
      assert {:ok, prepared} = Executor.prepare_batch(batch)
      checkpoint = Run.checkpoint(batch, preparation: prepared)
      changed = inject(batch, "tasks", location, Map.put(repair(), "sweep_action", "replan"))
      # The explicit action changes evidence even though both intents are admissible.
      assert {:ok, _} = Executor.prepare_batch(changed)
      refute Run.batch_digest(batch) == Run.batch_digest(changed)

      assert Enum.map(batch["tasks"], & &1["task_ir"]) ==
               Enum.map(changed["tasks"], & &1["task_ir"])

      assert {:error, _} = Run.verify_checkpoint(changed, checkpoint)
      assert {:error, _} = Run.resume_plan(changed, checkpoint)
    end
  end

  test "generic JSON metadata remains caller owned and need not match between cases" do
    for metadata <- [
          nil,
          "note",
          ["a", 3],
          42,
          %{"note" => "trial", "case_count_estimate" => 900}
        ] do
      payload = put_in(cases(), ["cases", Access.at(1), "metadata"], metadata)
      assert {:ok, batch} = compose(payload)
      assert List.last(batch["tasks"])["metadata"] == metadata
      assert {:ok, result} = Executor.execute_batch(batch)
      Fixture.assert_execution(result)
    end
  end

  test "125 scope placements cannot hide blocked evidence behind null or absent metadata" do
    choices = [
      :absent,
      nil,
      repair(),
      %{"sweep_enabled" => false},
      %{"coupled_readiness" => %{"coupled_readiness_state" => "block"}}
    ]

    assert {:ok, batch} = compose(cases())

    for a <- choices, b <- choices, c <- choices do
      placements = Enum.zip(~w(root_metadata case_metadata entry_metadata), [a, b, c])

      payload =
        Enum.reduce(placements, cases(), fn
          {_, :absent}, value -> value
          {location, evidence}, value -> inject(value, "cases", location, evidence)
        end)

      compiled =
        Enum.reduce(placements, batch, fn
          {_, :absent}, value -> value
          {location, evidence}, value -> inject(value, "tasks", location, evidence)
        end)

      accepted = Enum.all?([a, b, c], &(&1 in [:absent, nil, repair()]))

      if accepted do
        assert {:ok, _} = compose(exchange(payload))
        assert {:ok, prepared} = Executor.prepare_batch(exchange(compiled))
        assert prepared["verified_count"] == 2
      else
        assert {:error, _} = compose(exchange(payload))

        assert {:error, {:operator_task_batch_quality_rejected, _}} =
                 Executor.prepare_batch(exchange(compiled))
      end
    end
  end

  test "malformed direct inputs reach admission instead of unsupported operator fallback" do
    for {payload, config} <- [{nil, config()}, {[], config()}, {cases(), []}] do
      assert {:error, reason} =
               Runtime.run_transform_operator(
                 "transform.compose_quality_execution_batch",
                 payload,
                 config
               )

      assert is_binary(reason)
      assert reason =~ "transform.compose_quality_execution_batch:"
    end
  end

  defp compose(payload),
    do:
      Runtime.run_transform_operator(
        "transform.compose_quality_execution_batch",
        payload,
        config()
      )

  defp config,
    do: %{
      "operator_id" => "transform.evaluate_material_thermal_shock",
      "operator_config" => %{"constraint_factor" => 0.7}
    }

  defp cases, do: Fixture.expanded_cases()

  defp repair,
    do: %{
      "coupled_readiness" => %{"coupled_readiness_state" => "block"},
      "optimization_hint" => %{"action" => "fix_coupled_readiness"}
    }

  defp review,
    do: %{
      "coupled_readiness" => %{"coupled_readiness_state" => "warn"},
      "optimization_hint" => %{"action" => "review_coupled_readiness"}
    }

  defp inject(value, _key, "root", evidence), do: Map.merge(value, evidence)
  defp inject(value, _key, "root_metadata", evidence), do: Map.put(value, "metadata", evidence)

  defp inject(value, _key, "case_metadata", evidence),
    do: Map.put(value, "case_metadata", evidence)

  defp inject(value, key, "entry", evidence),
    do: update_in(value, [key, Access.at(1)], &Map.merge(&1, evidence))

  defp inject(value, key, "entry_metadata", evidence),
    do: put_in(value, [key, Access.at(1), "metadata"], evidence)

  defp extract(value, _key, "root"), do: Map.take(value, ~w(coupled_readiness optimization_hint))
  defp extract(value, _key, "root_metadata"), do: value["metadata"]
  defp extract(value, _key, "case_metadata"), do: value["case_metadata"]

  defp extract(value, key, "entry"),
    do: Map.take(List.last(value[key]), ~w(coupled_readiness optimization_hint))

  defp extract(value, key, "entry_metadata"), do: List.last(value[key])["metadata"]
  defp exchange(value), do: value |> Jason.encode!() |> Jason.decode!()
end
