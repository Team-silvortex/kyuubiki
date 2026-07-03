defmodule KyuubikiWeb.TestSupport.WorkflowLargeGraphBenchmark do
  @moduledoc false

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Router
  alias KyuubikiWeb.TestSupport.WorkflowApi
  alias KyuubikiWeb.TestSupport.WorkflowApiFixtures
  alias KyuubikiWeb.WorkflowGraphResponse

  @default_pass_through_counts [96, 192, 256, 384, 512]

  def default_pass_through_counts, do: @default_pass_through_counts

  def run_suite(router_opts, pass_through_counts \\ @default_pass_through_counts)
      when is_list(router_opts) and is_list(pass_through_counts) do
    Enum.map(pass_through_counts, &run_case(router_opts, &1))
  end

  def run_case(router_opts, pass_through_count)
      when is_list(router_opts) and is_integer(pass_through_count) and pass_through_count > 0 do
    run_case(router_opts, pass_through_count, %{})
  end

  def run_case(router_opts, pass_through_count, response_options)
      when is_list(router_opts) and is_integer(pass_through_count) and pass_through_count > 0 and
             is_map(response_options) do
    {:ok, _pid} = WorkflowApi.start_fake_agent_sessions(WorkflowApiFixtures.large_graph_frames())

    port = WorkflowApi.await_fake_agent_port()
    WorkflowApi.configure_fake_agent_pool(port)

    started_at = System.monotonic_time(:millisecond)

    conn =
      :post
      |> conn(
        "/api/v1/workflows/graph/run",
        Jason.encode!(%{
          "graph" => build_graph(pass_through_count),
          "input_artifacts" => %{
            "heat_model" => WorkflowApi.heat_to_thermo_quad_input_artifacts()["heat_model"]
          },
          "response_options" => response_options
        })
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(router_opts)

    elapsed_ms = System.monotonic_time(:millisecond) - started_at

    if conn.status != 200 do
      raise "large graph benchmark failed with status #{conn.status}"
    end

    payload = Jason.decode!(conn.resp_body)
    verify_payload!(payload, pass_through_count, response_options)

    %{
      "pass_through_count" => pass_through_count,
      "elapsed_ms" => elapsed_ms,
      "completed_nodes" => length(payload["completed_nodes"]),
      "skipped_nodes" => length(payload["skipped_nodes"]),
      "tail_artifact_key" => tail_artifact_key(pass_through_count),
      "performance" => Map.get(payload, "performance", %{}),
      "response_options" =>
        WorkflowGraphResponse.resolve_options(build_graph(pass_through_count), response_options),
      "slowest_nodes" => get_in(payload, ["performance", "slowest_nodes"]) || [],
      "payload" => payload
    }
  end

  def benchmark_report(
        router_opts,
        pass_through_counts \\ @default_pass_through_counts,
        response_options \\ %{}
      )
      when is_list(router_opts) and is_list(pass_through_counts) and is_map(response_options) do
    cases = Enum.map(pass_through_counts, &run_case(router_opts, &1, response_options))

    %{
      "generated_at" => DateTime.utc_now() |> DateTime.truncate(:second) |> DateTime.to_iso8601(),
      "cases" => Enum.map(cases, &report_case/1),
      "summary" => summarize_cases(cases)
    }
  end

  def build_graph(pass_through_count)
      when is_integer(pass_through_count) and pass_through_count > 0 do
    %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.large-heat-to-thermo-chain-#{pass_through_count}",
      "name" => "Large heat to thermo chain",
      "version" => "1.0.0",
      "entry_nodes" => ["heat_model"],
      "output_nodes" => ["thermo_output"],
      "defaults" => %{"cache_policy" => "cached", "orchestrated" => true},
      "nodes" => build_nodes(pass_through_count),
      "edges" => build_edges(pass_through_count)
    }
  end

  def verify_payload!(payload, pass_through_count, response_options \\ %{})
      when is_map(payload) and is_integer(pass_through_count) and is_map(response_options) do
    expected_completed_count = pass_through_count + 5
    tail_key = tail_artifact_key(pass_through_count)
    options = WorkflowGraphResponse.resolve_options(build_graph(pass_through_count), response_options)

    unless length(payload["completed_nodes"]) == expected_completed_count do
      raise "unexpected completed node count for #{pass_through_count}"
    end

    unless payload["skipped_nodes"] == [] do
      raise "large graph benchmark unexpectedly skipped nodes"
    end

    unless hd(payload["completed_nodes"]) == "heat_model" do
      raise "large graph benchmark did not begin with heat_model"
    end

    unless List.last(payload["completed_nodes"]) == "thermo_output" do
      raise "large graph benchmark did not finish with thermo_output"
    end

    if Map.get(options, "include_artifacts") &&
         get_in(payload, ["artifacts", tail_key, "max_temperature"]) != 100.0 do
      raise "tail artifact max_temperature drifted for #{pass_through_count}"
    end

    if Map.get(options, "include_artifacts") &&
         get_in(payload, ["artifacts", "thermo_output.result", "max_stress"]) <= 0.0 do
      raise "thermo output max_stress missing for #{pass_through_count}"
    end

    if not Map.get(options, "include_artifacts") and Map.has_key?(payload, "artifacts") do
      raise "artifacts should be omitted for #{pass_through_count}"
    end

    if not Map.get(options, "include_node_runs") and Map.has_key?(payload, "node_runs") do
      raise "node_runs should be omitted for #{pass_through_count}"
    end

    if not Map.get(options, "include_artifact_lineage") &&
         Map.has_key?(payload, "artifact_lineage") do
      raise "artifact_lineage should be omitted for #{pass_through_count}"
    end

    if not Map.get(options, "include_dataset_lineage") &&
         Map.has_key?(payload, "dataset_lineage") do
      raise "dataset_lineage should be omitted for #{pass_through_count}"
    end

    if not Map.get(options, "include_branch_decisions") &&
         Map.has_key?(payload, "branch_decisions") do
      raise "branch_decisions should be omitted for #{pass_through_count}"
    end

    unless get_in(payload, ["performance", "completed_node_count"]) == expected_completed_count do
      raise "performance completed node count drifted for #{pass_through_count}"
    end

    payload
  end

  defp report_case(case_result) do
    %{
      "pass_through_count" => case_result["pass_through_count"],
      "elapsed_ms" => case_result["elapsed_ms"],
      "completed_nodes" => case_result["completed_nodes"],
      "skipped_nodes" => case_result["skipped_nodes"],
      "performance" => case_result["performance"],
      "response_options" => case_result["response_options"],
      "slowest_nodes" => case_result["slowest_nodes"]
    }
  end

  defp summarize_cases(cases) do
    elapsed_ms_values = Enum.map(cases, & &1["elapsed_ms"])

    %{
      "case_count" => length(cases),
      "smallest_case" => Enum.min_by(cases, & &1["pass_through_count"])["pass_through_count"],
      "largest_case" => Enum.max_by(cases, & &1["pass_through_count"])["pass_through_count"],
      "fastest_elapsed_ms" => Enum.min(elapsed_ms_values),
      "slowest_elapsed_ms" => Enum.max(elapsed_ms_values),
      "average_elapsed_ms" => Enum.sum(elapsed_ms_values) / max(length(elapsed_ms_values), 1)
    }
  end

  defp build_nodes(pass_through_count) do
    [
      %{
        "id" => "heat_model",
        "kind" => "input",
        "outputs" => [%{"id" => "model", "artifact_type" => "study_model/heat_plane_quad_2d"}]
      },
      %{
        "id" => "solve_heat",
        "kind" => "solve",
        "operator_id" => "solve.heat_plane_quad_2d",
        "inputs" => [%{"id" => "model", "artifact_type" => "study_model/heat_plane_quad_2d"}],
        "outputs" => [%{"id" => "result", "artifact_type" => "result/heat_plane_quad_2d"}]
      }
    ] ++
      Enum.map(0..(pass_through_count - 1), fn index ->
        %{
          "id" => pass_node_id(index),
          "kind" => "transform",
          "operator_id" => "transform.first_available",
          "inputs" => [%{"id" => "input", "artifact_type" => "result/heat_plane_quad_2d"}],
          "outputs" => [%{"id" => "result", "artifact_type" => "result/heat_plane_quad_2d"}]
        }
      end) ++
      [
        %{
          "id" => "bridge_temperature",
          "kind" => "transform",
          "operator_id" => "bridge.temperature_field_to_thermo_quad_2d",
          "config" => %{
            "nodes" => [
              %{
                "id" => "t0",
                "x" => 0.0,
                "y" => 0.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              },
              %{
                "id" => "t1",
                "x" => 1.0,
                "y" => 0.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              },
              %{
                "id" => "t2",
                "x" => 1.0,
                "y" => 1.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              },
              %{
                "id" => "t3",
                "x" => 0.0,
                "y" => 1.0,
                "fix_x" => true,
                "fix_y" => true,
                "temperature_delta" => 0.0
              }
            ],
            "elements" => [
              %{
                "id" => "tq0",
                "node_i" => 0,
                "node_j" => 1,
                "node_k" => 2,
                "node_l" => 3,
                "thickness" => 0.02,
                "youngs_modulus" => 210.0e9,
                "poisson_ratio" => 0.3,
                "thermal_expansion" => 11.0e-6
              }
            ]
          },
          "inputs" => [%{"id" => "heat_result", "artifact_type" => "result/heat_plane_quad_2d"}],
          "outputs" => [
            %{"id" => "thermo_model", "artifact_type" => "study_model/thermal_plane_quad_2d"}
          ]
        },
        %{
          "id" => "solve_thermo",
          "kind" => "solve",
          "operator_id" => "solve.thermal_plane_quad_2d",
          "inputs" => [%{"id" => "model", "artifact_type" => "study_model/thermal_plane_quad_2d"}],
          "outputs" => [%{"id" => "result", "artifact_type" => "result/thermal_plane_quad_2d"}]
        },
        %{
          "id" => "thermo_output",
          "kind" => "output",
          "inputs" => [%{"id" => "result", "artifact_type" => "result/thermal_plane_quad_2d"}],
          "outputs" => []
        }
      ]
  end

  defp build_edges(pass_through_count) do
    [
      %{
        "id" => "edge-heat-input",
        "from" => %{"node" => "heat_model", "port" => "model"},
        "to" => %{"node" => "solve_heat", "port" => "model"},
        "artifact_type" => "study_model/heat_plane_quad_2d"
      }
    ] ++
      Enum.map(0..(pass_through_count - 1), fn index ->
        from_node = if index == 0, do: "solve_heat", else: pass_node_id(index - 1)

        %{
          "id" => "edge-pass-#{index}",
          "from" => %{"node" => from_node, "port" => "result"},
          "to" => %{"node" => pass_node_id(index), "port" => "input"},
          "artifact_type" => "result/heat_plane_quad_2d"
        }
      end) ++
      [
        %{
          "id" => "edge-tail-to-bridge",
          "from" => %{"node" => pass_node_id(pass_through_count - 1), "port" => "result"},
          "to" => %{"node" => "bridge_temperature", "port" => "heat_result"},
          "artifact_type" => "result/heat_plane_quad_2d"
        },
        %{
          "id" => "edge-bridge-to-thermo",
          "from" => %{"node" => "bridge_temperature", "port" => "thermo_model"},
          "to" => %{"node" => "solve_thermo", "port" => "model"},
          "artifact_type" => "study_model/thermal_plane_quad_2d"
        },
        %{
          "id" => "edge-thermo-output",
          "from" => %{"node" => "solve_thermo", "port" => "result"},
          "to" => %{"node" => "thermo_output", "port" => "result"},
          "artifact_type" => "result/thermal_plane_quad_2d"
        }
      ]
  end

  defp tail_artifact_key(pass_through_count) do
    "pass_#{String.pad_leading(Integer.to_string(pass_through_count - 1), 3, "0")}.result"
  end

  defp pass_node_id(index) do
    "pass_#{String.pad_leading(Integer.to_string(index), 3, "0")}"
  end
end
