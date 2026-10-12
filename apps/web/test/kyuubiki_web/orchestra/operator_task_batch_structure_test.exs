defmodule KyuubikiWeb.Orchestra.OperatorTaskBatchStructureTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Orchestra.OperatorTaskExecutor, as: Executor
  alias KyuubikiWeb.Orchestra.OperatorTaskEnvelope, as: Envelope
  alias KyuubikiWeb.Orchestra.OperatorTaskBatchRun, as: Run
  alias KyuubikiWeb.Orchestra.OperatorTaskIR, as: IR

  for path <- [
        ["tasks"],
        ["tasks", 1],
        ["tasks", 1, "task_ir"],
        ["tasks", 1, "task_ir", "operator"],
        ["tasks", 1, "task_ir", "operator", "execution"],
        ["tasks", 1, "task_ir", "integrity"],
        ["tasks", 1, "task_ir", "execution_program"],
        ["tasks", 1, "task_ir", "execution_program", "abi"],
        ["tasks", 1, "task_ir", "execution_program", "entrypoint"],
        ["tasks", 1, "task_ir", "runtime_hints"],
        ["tasks", 1, "task_ir", "node"],
        ["tasks", 1, "task_ir", "orchestration_context"],
        ["tasks", 1, "task_ir", "runtime_hints", "cache_scope"],
        ["tasks", 1, "task_ir", "execution_program", "package_ref"]
      ] do
    test "malformed batch path #{Enum.join(path, ".")} is rejected without an exception" do
      batch = batch()
      checkpoint = Run.checkpoint(batch)

      path =
        unquote(path)
        |> Enum.map(fn
          n when is_integer(n) -> Access.at(n)
          key -> key
        end)

      for value <- [42, true, [], %{"bad" => 1}, nil] do
        invalid = put_in(batch, path, value)
        # Some paths accept object values, but the malformed scalar/list variants must not crash.
        nullable_leaf = List.last(unquote(path)) in ~w(cache_scope package_ref)

        if !(is_map(value) && !nullable_leaf && unquote(path) != ["tasks"]) &&
             !(nullable_leaf && value == nil) &&
             !(unquote(path) == ["tasks"] && value == []) do
          for invalid <- Enum.uniq([invalid, reseal_if_possible(invalid)]),
              call <- [
                fn -> Executor.prepare_batch(invalid) end,
                fn -> Executor.execute_batch(invalid) end,
                fn -> Envelope.checkpoint_batch(%{"batch" => invalid}) end,
                fn -> Run.verify_checkpoint(invalid, checkpoint) end,
                fn -> Run.resume_plan(invalid, checkpoint) end
              ] do
            assert match?({:error, _}, call.())
          end
        end
      end
    end
  end

  test "duplicate case and task identities reject before execution and checkpoint creation" do
    good = batch()
    [first, last] = good["tasks"]
    duplicate_case = %{good | "tasks" => [first, Map.put(last, "case_id", first["case_id"])]}
    duplicate_task = %{good | "tasks" => [first, %{last | "task_ir" => first["task_ir"]}]}

    for invalid <- [duplicate_case, duplicate_task] do
      assert {:error, {:operator_task_batch_identity_conflict, _}} =
               Executor.prepare_batch(invalid)

      assert {:error, {:operator_task_batch_identity_conflict, _}} =
               Executor.execute_batch(invalid)

      assert {:error, {:operator_task_batch_identity_conflict, _}} =
               Envelope.checkpoint_batch(%{"batch" => invalid})
    end
  end

  test "missing or blank recovery identity is not synthesized or discarded" do
    for path <- [
          ["tasks", Access.at(1), "case_id"],
          ["tasks", Access.at(1), "task_ir", "task_id"]
        ],
        value <- [nil, "", "  ", 3, [], %{}] do
      invalid = put_in(batch(), path, value)
      assert match?({:error, _}, Executor.prepare_batch(invalid))
      assert match?({:error, _}, Executor.execute_batch(invalid))
      assert match?({:error, _}, Envelope.checkpoint_batch(%{"batch" => invalid}))
    end
  end

  test "checkpoint API validates malformed summaries rather than crashing or dropping them" do
    for key <- ~w(preparation execution), value <- [42, "bad", true, []] do
      assert match?({:error, _}, Envelope.checkpoint_batch(%{"batch" => batch(), key => value}))
    end
  end

  test "checkpoint resume cannot use malformed policy even with a recomputed digest" do
    good = batch()
    checkpoint = Run.checkpoint(good)

    for policy <- [
          nil,
          false,
          [],
          "execute",
          %{},
          %{"status" => "prepared", "next_action" => "unknown"}
        ] do
      bad = checkpoint |> Map.put("resume_policy", policy) |> refresh_checkpoint()

      assert {:error, {:invalid_operator_task_batch_checkpoint_structure, "resume_policy", _}} =
               Run.verify_checkpoint(good, bad)

      assert {:error, {:invalid_operator_task_batch_checkpoint_structure, "resume_policy", _}} =
               Run.resume_plan(good, bad)
    end
  end

  test "checkpoint summaries cannot target cases outside the batch or duplicate a target" do
    for targets <- [["not-in-batch"], ["case-b", "case-b"], [nil], "case-b"] do
      payload = %{
        "batch" => batch(),
        "execution" => %{"error_count" => 1, "failed_case_ids" => targets}
      }

      assert match?({:error, _}, Envelope.checkpoint_batch(payload))
    end
  end

  test "rehashed well-shaped policy cannot contradict its checkpoint summaries" do
    good = batch()
    checkpoint = Run.checkpoint(good)

    changed =
      checkpoint
      |> Map.put("resume_policy", %{"status" => "prepared", "next_action" => "execute"})
      |> refresh_checkpoint()

    assert {:error, {:operator_task_batch_checkpoint_mismatch, "resume_policy", _, _}} =
             Run.verify_checkpoint(good, changed)

    assert {:error, _} = Run.resume_plan(good, changed)
  end

  test "nested malformed recovery receipts and foreign targets cannot enter a checkpoint" do
    for summary <- [
          %{"failure_receipts" => "bad"},
          %{"failure_receipts" => [nil]},
          %{"failure_receipts" => [%{"case_id" => "outside"}]},
          %{"failure_receipts" => [%{"case_id" => "case-a", "recovery" => "retry"}]},
          %{"failure_receipts" => [%{"recovery" => %{"required_action" => %{}}}]},
          %{"readiness_recovery_actions" => "retry"},
          %{"readiness_recovery_actions" => [nil]},
          %{"skipped_case_ids" => ["outside"]},
          %{"incomplete_case_ids" => ["outside"]},
          %{"blocked_readiness_case_ids" => ["outside"]}
        ],
        key <- ~w(preparation execution) do
      assert {:error, {:invalid_operator_task_batch_checkpoint_structure, _, _}} =
               Envelope.checkpoint_batch(%{"batch" => batch(), key => summary})

      manifest = batch() |> Run.checkpoint() |> Map.put(key, summary) |> refresh_checkpoint()

      assert {:error, {:invalid_operator_task_batch_checkpoint_structure, _, _}} =
               Run.verify_checkpoint(batch(), manifest)

      assert {:error, _} = Run.resume_plan(batch(), manifest)
    end
  end

  test "empty batches and optional header shorthand keep valid checkpoint round trips" do
    for good <- [
          batch(),
          %{
            "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
            "tasks" => []
          }
        ] do
      assert {:ok, prepared} = Executor.prepare_batch(good)

      assert {:ok, checkpoint} =
               Envelope.checkpoint_batch(%{"batch" => good, "preparation" => prepared})

      assert {:ok, plan} = Run.resume_plan(good, checkpoint)
      assert plan["next_action"] == "execute"
      assert length(plan["target_case_ids"]) == length(good["tasks"])
    end
  end

  test "ordinary digest errors stay isolated and corrected batches keep unique retry targets" do
    good = batch()
    bad = put_in(good, ["tasks", Access.at(1), "task_ir", "input_artifact", "value"], 99)
    assert {:ok, result} = Executor.execute_batch(bad)
    assert result["ok_count"] == 1
    assert result["failed_case_ids"] == ["case-b"]
    assert {:ok, checkpoint} = Envelope.checkpoint_batch(%{"batch" => bad, "execution" => result})
    assert {:ok, plan} = Run.resume_plan(bad, checkpoint)
    assert plan["target_case_ids"] == ["case-b"]
    assert {:ok, fresh} = Executor.execute_batch(good)
    assert fresh["ok_count"] == 2
    assert {:ok, _} = Run.verify_checkpoint(bad, checkpoint)
    assert {:error, _} = Run.verify_checkpoint(good, checkpoint)
  end

  defp batch do
    tasks =
      for id <- ~w(a b) do
        {:ok, task} =
          IR.build("export.summary_json", %{"value" => 14.0}, %{}, task_id: "task-#{id}")

        %{"case_id" => "case-#{id}", "task_ir" => task}
      end

    %{
      "quality_execution_batch_contract" => "kyuubiki.quality_execution_batch/v1",
      "tasks" => tasks
    }
  end

  defp refresh_checkpoint(value) do
    digest =
      value
      |> Map.delete("checkpoint_digest")
      |> legacy_json()
      |> KyuubikiWeb.CanonicalJson.encode!()
      |> then(&:crypto.hash(:sha256, &1))
      |> Base.encode16(case: :lower)

    Map.put(value, "checkpoint_digest", digest)
  end

  defp legacy_json(value) when is_map(value),
    do: Map.new(value, fn {key, nested} -> {key, legacy_json(nested)} end)

  defp legacy_json([]), do: %{}
  defp legacy_json(value) when is_list(value), do: Enum.map(value, &legacy_json/1)
  defp legacy_json(value), do: value

  defp reseal_if_possible(
         %{"tasks" => [_, %{"task_ir" => %{"integrity" => integrity} = task}]} = batch
       )
       when is_map(integrity) do
    case IR.compute_task_digest_checked(task) do
      {:ok, digest} ->
        put_in(batch, ["tasks", Access.at(1), "task_ir", "integrity", "task_digest"], digest)

      _ ->
        batch
    end
  end

  defp reseal_if_possible(batch), do: batch
end
