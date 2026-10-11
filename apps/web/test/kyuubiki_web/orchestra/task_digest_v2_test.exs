defmodule KyuubikiWeb.Orchestra.TaskDigestV2Test do
  use ExUnit.Case, async: true
  import Bitwise
  alias KyuubikiWeb.Orchestra.{OperatorTaskIR, OperatorTaskExecutionSummary, OperatorTaskEnvelope}
  alias KyuubikiWeb.TaskDigestV2

  defp golden do
    Path.expand("../../../../../schemas/examples.operator-task-ir.json", __DIR__)
    |> File.read!()
    |> Jason.decode!()
  end

  test "shared typed digest vectors survive Jason roundtrip" do
    fixture =
      Path.expand("../../../../../schemas/examples.task-ir-typed-digest.json", __DIR__)
      |> File.read!()
      |> Jason.decode!()

    for example <- fixture["cases"] do
      assert {:ok, task} =
               OperatorTaskIR.seal_task_digest(
                 %{"input_artifact" => example["value"]},
                 TaskDigestV2.encoding()
               )

      assert task["integrity"]["task_digest"] == example["task_digest"]

      assert :ok =
               OperatorTaskExecutionSummary.validate_digest(Jason.decode!(Jason.encode!(task)))
    end
  end

  test "shared generated binary64 corpus retains every bit through Jason" do
    fixture =
      Path.expand("../../../../../schemas/examples.task-ir-typed-digest.json", __DIR__)
      |> File.read!()
      |> Jason.decode!()

    corpus = fixture["generated_binary64"]
    mask = 0xFFFFFFFFFFFFFFFF
    finite_mask = String.to_integer(corpus["finite_mask_hex"], 16)

    {values, _state} =
      Enum.map_reduce(1..corpus["count"], String.to_integer(corpus["seed_hex"], 16), fn _,
                                                                                        state ->
        state = bxor(state, band(state <<< 13, mask))
        state = bxor(state, state >>> 7)
        state = bxor(state, band(state <<< 17, mask))
        <<value::float-64>> = <<band(state, finite_mask)::unsigned-64>>
        {value, state}
      end)

    assert {:ok, task} =
             OperatorTaskIR.seal_task_digest(
               %{"input_artifact" => values},
               TaskDigestV2.encoding()
             )

    assert task["integrity"]["task_digest"] == corpus["task_digest"]
    assert :ok = OperatorTaskExecutionSummary.validate_digest(Jason.decode!(Jason.encode!(task)))
  end

  for field <- OperatorTaskIR.task_digest_fields() do
    test "typed digest rejects exact numeric changes in #{field}" do
      for {a, b} <- [
            {1.0e-30, 2.0e-30},
            {1.0e-300, 2.0e-300},
            {0.0, -0.0},
            {1, 1.0},
            {0.3333333333333333, 0.33333333333333337},
            {5.0e-324, 1.0e-323}
          ] do
        original = Map.put(golden(), unquote(field), %{"nested" => [%{"value" => a}]})
        assert {:ok, task} = OperatorTaskIR.seal_task_digest(original, TaskDigestV2.encoding())
        changed = Map.put(task, unquote(field), %{"nested" => [%{"value" => b}]})

        assert {:error, {:operator_task_digest_mismatch, _}} =
                 OperatorTaskExecutionSummary.validate_digest(changed)
      end
    end
  end

  test "typed encoding cannot be removed downgraded or forged and admission returns errors" do
    assert {:ok, task} = OperatorTaskIR.seal_task_digest(golden(), TaskDigestV2.encoding())
    assert {:ok, _} = OperatorTaskEnvelope.prepare(%{"task" => task})

    for replacement <- ["kyuubiki.task-digest/fixed15-v1", "future", nil, false, [], %{}] do
      changed = put_in(task, ["integrity", "task_digest_encoding"], replacement)
      assert {:error, _} = OperatorTaskEnvelope.prepare(%{"task" => changed})

      unless replacement == "kyuubiki.task-digest/fixed15-v1" do
        assert {:error, _} = OperatorTaskExecutionSummary.build(changed)
      end
    end

    changed = update_in(task, ["integrity"], &Map.delete(&1, "task_digest_encoding"))

    assert {:error, {:operator_task_digest_mismatch, _}} =
             OperatorTaskEnvelope.prepare(%{"task" => changed})

    for algorithm <- ["sha512", nil, %{}] do
      changed = put_in(task, ["integrity", "digest_algorithm"], algorithm)
      assert {:error, _} = OperatorTaskEnvelope.prepare(%{"task" => changed})
    end
  end

  test "Elixir and Rust descriptor authors can explicitly seal the same full task" do
    for mode <- ["elixir_control_plane", "rust_native"] do
      assert {:ok, task} =
               OperatorTaskIR.build(
                 "transform.rank_material_candidates",
                 %{"candidates" => %{}},
                 %{},
                 descriptor_authoring: %{"mode" => mode},
                 task_id: "typed-authoring"
               )

      assert {:ok, sealed} = OperatorTaskIR.seal_task_digest(task, TaskDigestV2.encoding())
      assert Map.delete(sealed, "integrity") == Map.delete(task, "integrity")
      assert :ok = OperatorTaskExecutionSummary.validate_digest(sealed)

      assert {:ok, _} =
               OperatorTaskEnvelope.prepare(%{"task" => Jason.decode!(Jason.encode!(sealed))})
    end
  end

  test "legacy golden remains unchanged and sealing is idempotent" do
    task = golden()
    assert :ok = OperatorTaskExecutionSummary.validate_digest(task)

    assert {:ok, legacy} =
             OperatorTaskIR.seal_task_digest(task, "kyuubiki.task-digest/fixed15-v1")

    assert legacy["integrity"]["task_digest"] == task["integrity"]["task_digest"]
    assert {:ok, sealed} = OperatorTaskIR.seal_task_digest(task, TaskDigestV2.encoding())
    assert {:ok, ^sealed} = OperatorTaskIR.seal_task_digest(sealed, TaskDigestV2.encoding())
  end

  test "typed codec rejects unsupported values and excessive depth without converting them" do
    deep = Enum.reduce(1..129, nil, fn _, value -> [value] end)

    for value <- [
          18_446_744_073_709_551_616,
          -9_223_372_036_854_775_809,
          %{atom: 1},
          %{"x" => :atom},
          <<255>>,
          deep
        ] do
      task = Map.put(golden(), "input_artifact", %{"value" => value})
      assert {:error, _} = OperatorTaskIR.seal_task_digest(task, TaskDigestV2.encoding())
    end

    assert {:error, _} =
             OperatorTaskIR.seal_task_digest(%{"integrity" => nil}, TaskDigestV2.encoding())
  end
end
