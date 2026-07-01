defmodule KyuubikiWeb.Orchestra.OperatorTaskIRTest do
  use ExUnit.Case, async: true

  alias KyuubikiWeb.Orchestra.OperatorTaskIR

  test "lowers a material transform operator into a stable engine task envelope" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.evaluate_material_thermal_shock",
               %{
                 "temperature_delta" => 160.0,
                 "thermal_expansion" => 1.2e-5,
                 "youngs_modulus" => 70.0e9,
                 "yield_strength" => 320.0e6
               },
               %{"constraint_factor" => 0.7},
               task_id: "task-material-shock-1",
               orchestration_context: %{"orch_id" => "orch-main"},
               dataset_contract: %{"id" => "kyuubiki.dataset.material_thermal_shock/v1"}
             )

    assert task["schema_version"] == "kyuubiki.operator-task-ir/v1"
    assert task["task_id"] == "task-material-shock-1"
    assert task["operator"]["id"] == "transform.evaluate_material_thermal_shock"
    assert task["operator"]["kind"] == "transform"
    assert task["operator"]["operator_category_id"] == "optimization_selection"
    assert task["descriptor_authoring"]["mode"] == "elixir_control_plane"
    assert task["descriptor_authoring"]["runtime"] == "elixir"
    assert task["descriptor_authoring"]["execution_language"] == "language_neutral"
    assert task["config"] == %{"constraint_factor" => 0.7}
    assert task["orchestration_context"] == %{"orch_id" => "orch-main"}
    assert task["runtime_hints"]["agent_fetchable"] == true
    assert "workflow_transform_runtime" in task["runtime_hints"]["required_capabilities"]
    assert is_binary(task["integrity"]["descriptor_digest"])
    assert is_binary(task["integrity"]["task_digest"])
    assert "execution_program" in task["integrity"]["task_digest_fields"]
    assert "integrity" not in task["integrity"]["task_digest_fields"]

    assert task["execution_program"]["schema_version"] ==
             "kyuubiki.operator-execution-program/v1"

    assert task["execution_program"]["runtime_protocol"] == "kyuubiki.operator-execution/v1"
    assert task["execution_program"]["abi"]["kind"] == "operator_task"
    assert task["execution_program"]["entrypoint"]["kind"] == "operator_id"

    assert task["execution_program"]["entrypoint"]["name"] ==
             "transform.evaluate_material_thermal_shock"
  end

  test "lowers a workflow node while preserving node identity and ports" do
    node = %{
      "id" => "estimate_fatigue",
      "kind" => "transform",
      "operator_id" => "transform.estimate_material_fatigue_life",
      "config" => %{"fatigue_strength" => 120.0},
      "inputs" => [%{"id" => "candidates", "artifact_type" => "report/summary_collection"}],
      "outputs" => [%{"id" => "fatigue", "artifact_type" => "report/summary"}]
    }

    assert {:ok, task} =
             OperatorTaskIR.from_node(node, %{"candidates" => %{"aluminum" => %{}}})

    assert task["task_id"] ==
             "operator-task:estimate_fatigue:transform.estimate_material_fatigue_life"

    assert task["node"]["id"] == "estimate_fatigue"
    assert task["node"]["inputs"] == node["inputs"]
    assert task["node"]["outputs"] == node["outputs"]
    assert task["config"] == %{"fatigue_strength" => 120.0}
    assert task["execution_program"]["node_binding"]["node_id"] == "estimate_fatigue"
    assert task["execution_program"]["node_binding"]["input_ports"] == node["inputs"]
    assert task["execution_program"]["node_binding"]["output_ports"] == node["outputs"]
  end

  test "lowers solver operators into solver RPC execution programs without Elixir runtime coupling" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "solve.heat_plane_quad_2d",
               %{"nodes" => [], "elements" => []},
               %{},
               task_id: "task-heat-quad"
             )

    program = task["execution_program"]

    assert program["schema_version"] == "kyuubiki.operator-execution-program/v1"
    assert program["program_id"] == "solve.heat_plane_quad_2d"
    assert program["program_kind"] == "solver"
    assert program["runtime_protocol"] == "kyuubiki.solver-rpc/v1"
    assert program["abi"]["kind"] == "solver_rpc"

    assert program["entrypoint"] == %{
             "kind" => "solver_method",
             "name" => "solve_heat_plane_quad_2d"
           }

    refute inspect(program) =~ "Elixir"
  end

  test "builds task IR from a Rust-authored descriptor without catalog fetch coupling" do
    operator = %{
      "id" => "transform.rust_material_screen",
      "version" => "1.0.0",
      "domain" => "multi_domain",
      "family" => "rust_material_screen",
      "kind" => "transform",
      "origin" => "external_descriptor",
      "operator_category_id" => "material_research",
      "capability_tags" => ["transform", "material", "headless_safe"],
      "execution" => %{
        "package_ref" => "orchestra://operator-package/transform.rust_material_screen",
        "package_version" => "1.0.0",
        "required_capabilities" => ["workflow_transform_runtime"],
        "placement_tags" => ["material"],
        "agent_fetchable" => true
      }
    }

    assert {:ok, task} =
             OperatorTaskIR.build_from_descriptor(
               operator,
               %{"material_card" => %{"material_id" => "aluminum"}},
               %{"mode" => "screen"},
               descriptor_authoring: %{
                 "mode" => "rust_native",
                 "runtime" => "rust",
                 "source" => "rust_operator_sdk",
                 "hot_reloadable" => false
               }
             )

    assert task["operator"]["id"] == "transform.rust_material_screen"
    assert task["descriptor_authoring"]["mode"] == "rust_native"
    assert task["descriptor_authoring"]["runtime"] == "rust"
    assert task["descriptor_authoring"]["source"] == "rust_operator_sdk"
    assert task["descriptor_authoring"]["hot_reloadable"] == false
    assert task["descriptor_authoring"]["execution_language"] == "language_neutral"
    assert task["execution_program"]["program_id"] == "transform.rust_material_screen"
    assert task["execution_program"]["runtime_protocol"] == "kyuubiki.operator-execution/v1"
    assert task["execution_program"]["entrypoint"]["name"] == "transform.rust_material_screen"
    refute inspect(task["execution_program"]) =~ "Elixir"
  end

  test "builds stable task digests that change when task inputs change" do
    assert {:ok, task_a} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{"aluminum" => %{"score" => 1.0}}},
               %{"top_k" => 1},
               task_id: "digest-task"
             )

    assert {:ok, task_b} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{"aluminum" => %{"score" => 1.0}}},
               %{"top_k" => 1},
               task_id: "digest-task"
             )

    assert {:ok, task_c} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{"aluminum" => %{"score" => 1.0}}},
               %{"top_k" => 2},
               task_id: "digest-task"
             )

    assert task_a["integrity"]["task_digest"] == task_b["integrity"]["task_digest"]
    refute task_a["integrity"]["task_digest"] == task_c["integrity"]["task_digest"]
  end

  test "keeps a golden canonical task digest for cross-language implementations" do
    operator = %{
      "id" => "transform.fixture",
      "family" => "fixture",
      "kind" => "transform",
      "execution" => %{
        "package_ref" => "orchestra://operator-package/transform.fixture"
      }
    }

    assert {:ok, task} =
             OperatorTaskIR.build_from_descriptor(
               operator,
               %{"x" => 1},
               %{"alpha" => true},
               task_id: "fixture-task",
               descriptor_authoring: %{
                 "mode" => "rust_native",
                 "runtime" => "rust",
                 "source" => "fixture",
                 "hot_reloadable" => false
               }
             )

    assert task["integrity"]["descriptor_digest"] ==
             "b397ef3b203a0500a29aabe507868b4104ddf22faee5015df69cc0486ac35cd2"

    assert task["integrity"]["task_digest"] ==
             "86c14d1f22af9d14ab35669a2fcb869afab097a9883e6deabf92a362d8f4469f"
  end

  test "keeps a golden canonical task digest for float-heavy cross-language payloads" do
    operator = %{
      "id" => "transform.float_fixture",
      "family" => "fixture",
      "kind" => "transform",
      "execution" => %{
        "package_ref" => "orchestra://operator-package/transform.float_fixture"
      }
    }

    assert {:ok, task} =
             OperatorTaskIR.build_from_descriptor(
               operator,
               %{
                 "temperature_delta" => 160.0,
                 "thermal_expansion" => 1.2e-5,
                 "youngs_modulus" => 70.0e9,
                 "poisson_ratio" => 0.33
               },
               %{"constraint_factor" => 0.7},
               task_id: "float-fixture-task",
               descriptor_authoring: %{
                 "mode" => "rust_native",
                 "runtime" => "rust",
                 "source" => "float_fixture",
                 "hot_reloadable" => false
               }
             )

    assert task["integrity"]["descriptor_digest"] ==
             "083dfdfa3a8e7115d6966df8d64b457205db07811a4658d6bd319b60778aa612"

    assert task["integrity"]["task_digest"] ==
             "d87818ffb27cc8f01e6a360f973ebf1d40025362b28cda0909078b99cd6139b7"
  end

  test "rejects incomplete Rust-authored descriptors before task IR construction" do
    base_operator = %{
      "id" => "transform.rust_material_screen",
      "family" => "rust_material_screen",
      "kind" => "transform",
      "execution" => %{
        "package_ref" => "orchestra://operator-package/transform.rust_material_screen"
      }
    }

    assert {:error, :missing_operator_task_family} =
             base_operator
             |> Map.delete("family")
             |> OperatorTaskIR.build_from_descriptor(%{}, %{})

    assert {:error, :missing_operator_task_execution} =
             base_operator
             |> Map.delete("execution")
             |> OperatorTaskIR.build_from_descriptor(%{}, %{})

    assert {:error, {:invalid_operator_task_package_ref, "orchestra://operator-package/other"}} =
             base_operator
             |> put_in(["execution", "package_ref"], "orchestra://operator-package/other")
             |> OperatorTaskIR.build_from_descriptor(%{}, %{})
  end

  test "exposes the future agent RPC method and params shape" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{}
             )

    assert OperatorTaskIR.agent_rpc_method() == "run_operator_task_ir"
    assert OperatorTaskIR.agent_rpc_params(task) == %{"task_ir" => task}
  end

  test "exposes agent routing opts from runtime hints" do
    assert {:ok, task} =
             OperatorTaskIR.build(
               "transform.rank_material_candidates",
               %{"candidates" => %{}},
               %{},
               task_id: "operator-task:rank-materials",
               placement_tags: ["materials", "ranking"],
               orchestration_context: %{"orch_id" => "orch-main"}
             )

    assert OperatorTaskIR.agent_routing_opts(task) == [
             job_id: "operator-task:rank-materials",
             orchestration: %{"orch_id" => "orch-main"},
             placement_tags: ["materials", "ranking"],
             required_capabilities: ["workflow_transform_runtime"]
           ]
  end
end
