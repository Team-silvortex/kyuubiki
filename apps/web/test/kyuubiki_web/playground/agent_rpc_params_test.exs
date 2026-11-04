defmodule KyuubikiWeb.Playground.AgentRpcParamsTest do
  use ExUnit.Case, async: true
  alias KyuubikiWeb.Playground.AgentRpcParams

  test "atom control envelopes become the same JSON names and mode used on the wire" do
    for mode <- [:execute, :preflight, :unknown, nil, true, false, "execute", "preflight"] do
      original = %{mode: mode, task_ir: %{"task_id" => "task"}, job_id: "job"}
      assert {:ok, normalized} = AgentRpcParams.normalize("run_operator_task_ir", original)
      assert Enum.all?(Map.keys(normalized), &is_binary/1)
      assert normalized == original |> Jason.encode!() |> Jason.decode!()
    end
  end

  test "only the outer control envelope is normalized; payload values are unchanged" do
    payload = %{"input_artifact" => %{"model" => [1.0e308, -0.0, 1.0e-300]}, "task_id" => "task"}

    assert {:ok, normalized} =
             AgentRpcParams.normalize("run_operator_task_ir", %{mode: :execute, task_ir: payload})

    assert normalized["task_ir"] === payload
    assert normalized["mode"] == "execute"
  end

  test "collisions reject even equal values and unused extension fields" do
    for key <- [:mode, :task_ir, :job_id, :extension] do
      for other <- ["same", "different"] do
        params = %{key => "same", Atom.to_string(key) => other}

        assert {:error, {:request_encoding_failed, "ambiguous TaskIR parameter keys"}} =
                 AgentRpcParams.normalize("run_operator_task_ir", params)
      end
    end
  end

  test "invalid keys and custom-encoded parameter objects fail before routing" do
    for key <- [1, {:private, "payload"}, ["mode"]] do
      assert {:error, {:request_encoding_failed, "invalid TaskIR parameter key"}} =
               AgentRpcParams.normalize("run_operator_task_ir", %{key => "private"})
    end

    for value <- [%URI{path: "private"}, nil, [], "private"] do
      assert {:error, {:request_encoding_failed, "invalid TaskIR parameter object"}} =
               AgentRpcParams.normalize("run_operator_task_ir", value)
    end
  end

  test "legacy solver and recovery envelopes retain their existing representation" do
    params = %{mode: :execute, model: %{value: 1.0}}

    for method <- ["solve_bar_1d", "cancel_job", "describe_agent"] do
      assert {:ok, ^params} = AgentRpcParams.normalize(method, params)
    end
  end
end
