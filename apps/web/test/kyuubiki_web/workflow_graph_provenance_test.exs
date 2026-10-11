defmodule KyuubikiWeb.WorkflowGraphProvenanceTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.{WorkflowCondition, WorkflowGraphProvenance, WorkflowGraphRunner}

  test "recursive path binding distinguishes sources behind the same join output" do
    state = roots()

    left =
      WorkflowGraphProvenance.publish(state, "join.payload", "join", "payload", ["a.payload"])

    right =
      WorkflowGraphProvenance.publish(state, "join.payload", "join", "payload", ["b.payload"])

    assert WorkflowGraphProvenance.input_digest(left, ["join.payload"]) !=
             WorkflowGraphProvenance.input_digest(right, ["join.payload"])

    left =
      WorkflowGraphProvenance.publish(left, "pass.payload", "pass", "payload", ["join.payload"])

    right =
      WorkflowGraphProvenance.publish(right, "pass.payload", "pass", "payload", ["join.payload"])

    assert WorkflowGraphProvenance.input_digest(left, ["pass.payload"]) !=
             WorkflowGraphProvenance.input_digest(right, ["pass.payload"])
  end

  test "unused paths and publication order do not change the selected binding" do
    state = roots()
    original = WorkflowGraphProvenance.input_digest(state, ["a.payload"])
    changed = WorkflowGraphProvenance.publish(state, "b.payload", "other", "payload", [])
    assert WorkflowGraphProvenance.input_digest(changed, ["a.payload"]) == original

    reverse =
      %{artifact_bindings: %{}}
      |> WorkflowGraphProvenance.publish("b.payload", "b", "payload", [])
      |> WorkflowGraphProvenance.publish("a.payload", "a", "payload", [])

    assert WorkflowGraphProvenance.input_digest(reverse, ["a.payload", "b.payload"]) ==
             WorkflowGraphProvenance.input_digest(state, ["a.payload", "b.payload"])
  end

  test "every consumed source and producer port contributes to the binding" do
    state = roots()
    both = WorkflowGraphProvenance.input_digest(state, ["a.payload", "b.payload"])
    changed = WorkflowGraphProvenance.publish(state, "b.payload", "b", "other-port", [])
    assert both != WorkflowGraphProvenance.input_digest(changed, ["a.payload", "b.payload"])
    assert both != WorkflowGraphProvenance.input_digest(state, ["a.payload"])
    assert state.artifact_bindings["a.payload"] != state.artifact_bindings["b.payload"]
  end

  test "a long chain retains one fixed-size digest per artifact, not ancestor trees" do
    state = roots()

    {state, last} =
      Enum.reduce(1..128, {state, "a.payload"}, fn i, {state, previous} ->
        id = "node-#{i}"
        key = id <> ".payload"
        {WorkflowGraphProvenance.publish(state, key, id, "payload", [previous]), key}
      end)

    assert map_size(state.artifact_bindings) == 130

    assert Enum.all?(state.artifact_bindings, fn {_, digest} -> digest =~ ~r/\A[0-9a-f]{64}\z/ end)

    assert WorkflowGraphProvenance.input_digest(state, [last]) =~ ~r/\A[0-9a-f]{64}\z/
    assert Map.keys(state) == [:artifact_bindings]
  end

  test "computed bindings replace caller values without changing routing authority" do
    digest = WorkflowGraphProvenance.input_digest(roots(), ["a.payload"])

    context = %{
      "job_id" => "parent",
      "control_mode" => "mesh",
      "workflow_input_binding_sha256" => "forged"
    }

    bound = WorkflowGraphProvenance.bind_context(context, digest)

    assert Map.drop(bound, ["workflow_input_binding_sha256"]) ==
             Map.drop(context, ["workflow_input_binding_sha256"])

    assert bound["workflow_input_binding_sha256"] == digest
  end

  for kind <- ~w(solve transform extract) do
    test "#{kind} callback receives the computed binding, not a caller option" do
      kind = unquote(kind)
      node = operator("processor", kind, "custom.operator", ["payload"])

      graph =
        graph([input("a"), node, output()], [edge("a", "processor"), edge("processor", "output")])

      expected = WorkflowGraphProvenance.input_digest(roots(), ["a.payload"])

      callback = fn _id, payload, _config, original_node, binding ->
        assert original_node == node
        assert binding == expected
        send(self(), :called)
        {:ok, payload}
      end

      solve = fn id, payload, original_node, binding ->
        callback.(id, payload, nil, original_node, binding)
      end

      opts =
        Keyword.put(
          executors(),
          String.to_existing_atom("execute_#{kind}"),
          if(kind == "solve", do: solve, else: callback)
        )

      for reverse <- [false, true] do
        graph = if reverse, do: update_in(graph["nodes"], &Enum.reverse/1), else: graph

        assert {:ok, result} =
                 WorkflowGraphRunner.run(
                   graph,
                   %{"a" => %{"value" => 1}},
                   opts ++ [input_binding_sha256: "forged"]
                 )

        assert_receive :called
        refute_receive :called, 0
        refute Map.has_key?(result, "artifact_bindings")
        assert result["artifacts"]["output.payload"] == %{"value" => 1}
      end
    end
  end

  test "condition callbacks receive the same selected input binding" do
    node = operator("condition", "condition", nil, ["payload"])

    node =
      node |> Map.delete("operator_id") |> Map.put("outputs", [port("if_true"), port("if_false")])

    node =
      Map.put(node, "config", %{
        "predicate" => %{"path" => "value", "operator" => "gt", "value" => 0}
      })

    graph =
      graph([input("a"), node, output()], [
        edge("a", "condition"),
        edge("condition", "output", "payload", "if_true")
      ])

    expected = WorkflowGraphProvenance.input_digest(roots(), ["a.payload"])

    callback = fn payload, original_node, binding ->
      assert binding == expected
      assert original_node == node
      send(self(), :condition_called)
      WorkflowCondition.decide(payload, original_node)
    end

    assert {:ok, result} =
             WorkflowGraphRunner.run(
               graph,
               %{"a" => %{"value" => 1}},
               Keyword.put(executors(), :execute_condition, callback)
             )

    assert_receive :condition_called
    assert result["artifacts"]["output.payload"] == %{"value" => 1}
  end

  test "named inputs bind all sources while fallback binds only its selected path" do
    for id <- ["transform.merge_summary_pair", "transform.first_available"],
        preferred <- ["a", "b"] do
      first = edge(preferred, "join", "left")
      second = edge(if(preferred == "a", do: "b", else: "a"), "join", "right")

      graph =
        graph(
          [
            input("a"),
            input("b"),
            operator("join", "transform", id, ["left", "right"]),
            output()
          ],
          [first, second, edge("join", "output")]
        )

      consumed =
        if id == "transform.first_available",
          do: [preferred <> ".payload"],
          else: [first["from"]["node"] <> ".payload", second["from"]["node"] <> ".payload"]

      expected = WorkflowGraphProvenance.input_digest(roots(), consumed)

      callback = fn _, payload, _, _, binding ->
        assert binding == expected
        send(self(), :bound)
        {:ok, payload}
      end

      assert {:ok, _} =
               WorkflowGraphRunner.run(
                 graph,
                 %{"a" => %{"value" => 1}, "b" => %{"value" => 1}},
                 Keyword.put(executors(), :execute_transform, callback)
               )

      assert_receive :bound
    end
  end

  test "identical values through local joins still produce different downstream bindings" do
    bindings =
      for preferred <- ["a", "b"], reverse <- [false, true] do
        nodes = [
          input("a"),
          input("b"),
          operator("join", "transform", "transform.first_available", ["left", "right"]),
          operator("pass", "transform", "custom.pass", ["payload"]),
          operator("solve", "solve", "custom.solve", ["payload"]),
          output()
        ]

        edges = [
          edge(preferred, "join", "left"),
          edge(if(preferred == "a", do: "b", else: "a"), "join", "right"),
          edge("join", "pass"),
          edge("pass", "solve"),
          edge("solve", "output")
        ]

        graph = graph(if(reverse, do: Enum.reverse(nodes), else: nodes), edges)

        solve = fn _, payload, _, binding ->
          send(self(), {:binding, binding})
          {:ok, payload}
        end

        opts =
          executors()
          |> Keyword.put(:execute_solve, solve)
          |> Keyword.put(:execute_transform, fn _, payload, _, _, _ -> {:ok, payload} end)

        assert {:ok, result} =
                 WorkflowGraphRunner.run(
                   graph,
                   %{"a" => %{"value" => 1}, "b" => %{"value" => 1}},
                   opts
                 )

        assert result["artifacts"]["output.payload"] == %{"value" => 1}
        assert_receive {:binding, binding}
        binding
      end

    assert [left, left, right, right] = bindings
    assert left != right
  end

  defp roots do
    %{artifact_bindings: %{}}
    |> WorkflowGraphProvenance.publish("a.payload", "a", "payload", [])
    |> WorkflowGraphProvenance.publish("b.payload", "b", "payload", [])
  end

  defp executors do
    unused = fn _, _, _ -> flunk("unexpected executor") end

    [
      execute_solve: unused,
      execute_transform: unused,
      execute_extract: unused,
      execute_export: unused
    ]
  end

  defp graph(nodes, edges),
    do: %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "provenance",
      "nodes" => nodes,
      "edges" => edges
    }

  defp input(id),
    do: %{"id" => id, "kind" => "input", "inputs" => [], "outputs" => [port("payload")]}

  defp output,
    do: %{"id" => "output", "kind" => "output", "inputs" => [port("payload")], "outputs" => []}

  defp operator(id, kind, operator, inputs),
    do: %{
      "id" => id,
      "kind" => kind,
      "operator_id" => operator,
      "inputs" => Enum.map(inputs, &port/1),
      "outputs" => [port("payload")]
    }

  defp port(id), do: %{"id" => id, "artifact_type" => "payload/json"}

  defp edge(from, to, input \\ "payload", output \\ "payload"),
    do: %{
      "id" => "#{from}-#{to}",
      "from" => %{"node" => from, "port" => output},
      "to" => %{"node" => to, "port" => input},
      "artifact_type" => "payload/json"
    }
end
