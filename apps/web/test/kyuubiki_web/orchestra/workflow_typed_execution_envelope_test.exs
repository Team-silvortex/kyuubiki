defmodule KyuubikiWeb.Orchestra.WorkflowTypedExecutionEnvelopeTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.{WorkflowRecoveryEnvelope, WorkflowSolveCheckpoint}
  alias KyuubikiWeb.Storage.DurableJson
  alias KyuubikiWeb.TaskDigestV2
  alias KyuubikiWeb.WorkflowTaskDigestPolicy

  defp graph do
    %{
      "id" => "typed-recovery",
      "defaults" => %{"native_task_digest_encoding" => TaskDigestV2.encoding()},
      "nodes" => [%{"id" => "solve", "kind" => "solve", "operator_id" => "solve.bar_1d"}]
    }
  end

  defp numbers, do: %{"values" => [1, 1.0, -0.0, 0.0, 1.0e-30, 1.0000000000000002]}

  defp pending,
    do:
      WorkflowRecoveryEnvelope.new(graph(), %{"input" => numbers()}, %{"job_id" => "parent"}, %{})

  defp bits(values), do: Enum.map(Enum.filter(values, &is_float/1), &<<&1::float-64>>)

  defp reseal(recovery) do
    sha =
      :crypto.hash(:sha256, DurableJson.encode!(recovery["envelope"]))
      |> Base.encode16(case: :lower)

    Map.put(recovery, "envelope_sha256", sha)
  end

  test "typed execution survives an outer storage round trip without numeric normalization" do
    assert {:ok, recovery} = pending()
    assert recovery["envelope"]["schema_version"] == "kyuubiki.workflow-execution-envelope/v2"
    assert Enum.sort(Map.keys(recovery["envelope"])) == ["execution_json", "schema_version"]
    reloaded = recovery |> DurableJson.encode!() |> Jason.decode!()
    assert :ok = WorkflowRecoveryEnvelope.verify(reloaded)
    assert {:ok, execution} = WorkflowRecoveryEnvelope.execution(reloaded)
    values = execution["input_artifacts"]["input"]["values"]
    assert values === numbers()["values"]
    assert bits(values) == bits(numbers()["values"])
    assert Enum.at(bits(values), 1) == <<128, 0, 0, 0, 0, 0, 0, 0>>
  end

  test "config and input representation affect immutable recovery identity" do
    digests =
      for number <- [1, 1.0, -0.0, 0.0, 1.0e-30, 2.0e-30, 1.0000000000000002] do
        graph = put_in(graph(), ["nodes", Access.at(0), "config"], %{"threshold" => number})
        {:ok, recovery} = WorkflowRecoveryEnvelope.new(graph, %{"input" => number}, %{}, %{})
        recovery["envelope_sha256"]
      end

    assert length(Enum.uniq(digests)) == length(digests)
  end

  test "typed encoding cannot be silently removed or downgraded on restart" do
    {:ok, recovery} = pending()

    for encoding <- [nil, WorkflowTaskDigestPolicy.legacy(), "unknown"] do
      assert {:error, :workflow_recovery_policy_mismatch} =
               WorkflowRecoveryEnvelope.verify(
                 Map.put(recovery, "native_task_digest_encoding", encoding)
               )
    end

    assert {:error, :workflow_recovery_policy_mismatch} =
             WorkflowRecoveryEnvelope.verify(Map.delete(recovery, "native_task_digest_encoding"))

    {:ok, execution} = WorkflowRecoveryEnvelope.execution(recovery)
    downgraded = put_in(execution, ["graph", "defaults"], %{})

    changed =
      put_in(recovery, ["envelope", "execution_json"], DurableJson.encode!(downgraded))
      |> reseal()

    assert {:error, :invalid_workflow_execution_envelope} =
             WorkflowRecoveryEnvelope.verify(changed)

    changed = Map.put(recovery, "envelope", execution) |> reseal()

    assert {:error, :invalid_workflow_execution_envelope} =
             WorkflowRecoveryEnvelope.verify(changed)
  end

  test "tampered JSON and ambiguous alternate storage refuse recovery" do
    {:ok, recovery} = pending()
    damaged = put_in(recovery, ["envelope", "execution_json"], "{}")
    assert {:error, :workflow_recovery_digest_mismatch} = WorkflowRecoveryEnvelope.verify(damaged)

    assert {:error, :invalid_workflow_execution_envelope} =
             WorkflowRecoveryEnvelope.verify(reseal(damaged))

    for envelope <- [
          Map.put(recovery["envelope"], "graph", graph()),
          %{"schema_version" => "kyuubiki.workflow-execution-envelope/v3"},
          %{
            "schema_version" => "kyuubiki.workflow-execution-envelope/v2",
            "execution_json" => "["
          }
        ] do
      assert {:error, :invalid_workflow_execution_envelope} =
               recovery
               |> Map.put("envelope", envelope)
               |> reseal()
               |> WorkflowRecoveryEnvelope.verify()
    end
  end

  test "legacy records with no explicit encoding retain their original behavior" do
    {:ok, recovery} =
      WorkflowRecoveryEnvelope.new(
        Map.delete(graph(), "defaults"),
        %{"input" => numbers()},
        %{},
        %{}
      )

    old = Map.delete(recovery, "native_task_digest_encoding")
    assert :ok = WorkflowRecoveryEnvelope.verify(old)

    assert old["envelope"]["input_artifacts"]["input"]["values"] === [
             1,
             1,
             0,
             0,
             1.0e-30,
             1.0000000000000002
           ]

    assert WorkflowRecoveryEnvelope.public_summary(old)["native_task_digest_encoding"] ==
             WorkflowTaskDigestPolicy.legacy()
  end

  test "unsupported selectors fail before preparing a durable workflow" do
    for encoding <- [nil, false, 2, "typed-v3", %{}, []] do
      graph = put_in(graph(), ["defaults", "native_task_digest_encoding"], encoding)
      assert {:error, _} = WorkflowRecoveryEnvelope.new(graph, %{}, %{}, %{})
    end
  end

  test "solve checkpoint can bind and restore a lossless typed execution envelope" do
    {:ok, pending} = pending()
    {:ok, running, claim} = WorkflowRecoveryEnvelope.claim(pending, "session-a", :initial)
    {:ok, execution} = WorkflowRecoveryEnvelope.execution(running)

    request =
      WorkflowSolveCheckpoint.request(
        hd(execution["graph"]["nodes"]),
        numbers(),
        execution["orchestration_context"]
      )

    assert {:ok, retained, :retained} =
             WorkflowSolveCheckpoint.put(running, request, numbers(), claim)

    retained = retained |> DurableJson.encode!() |> Jason.decode!()
    assert :ok = WorkflowSolveCheckpoint.verify(retained, "parent")

    {:ok, restarted, next_claim} =
      WorkflowRecoveryEnvelope.claim(retained, "session-b", :process_restart)

    assert {:ok, result} = WorkflowSolveCheckpoint.lookup(restarted, request, next_claim)
    assert result === numbers()
    assert :execute = WorkflowSolveCheckpoint.lookup(running, request, claim)
  end

  test "terminal pruning retains encoding visibility but not another execution copy" do
    {:ok, recovery} = pending()

    for state <- ~w(completed failed cancelled) do
      terminal = WorkflowRecoveryEnvelope.transition(recovery, state)
      refute Map.has_key?(terminal, "envelope")
      public = WorkflowRecoveryEnvelope.public_summary(terminal)
      assert public["native_task_digest_encoding"] == TaskDigestV2.encoding()
      assert public["envelope_retained"] == false
      refute Map.has_key?(public, "execution_json")
    end
  end
end
