defmodule KyuubikiWeb.TestSupport.WorkflowApi do
  @moduledoc false

  require ExUnit.Assertions

  import Plug.Conn
  import Plug.Test

  alias KyuubikiWeb.Playground.AgentPool
  alias KyuubikiWeb.Router
  alias KyuubikiWeb.TestSupport.WorkflowApiFixtures

  def await_fake_agent_port(timeout \\ 1_000) do
    receive do
      {:fake_agent_ready, port} -> port
    after
      timeout -> ExUnit.Assertions.flunk("timed out waiting for fake agent port")
    end
  end

  def start_fake_agent_sessions(frame_sessions) when is_list(frame_sessions) do
    parent = self()

    Task.start_link(fn ->
      {:ok, listen_socket} =
        :gen_tcp.listen(0, [
          :binary,
          packet: 4,
          active: false,
          reuseaddr: true,
          ip: {127, 0, 0, 1}
        ])

      {:ok, port} = :inet.port(listen_socket)
      send(parent, {:fake_agent_ready, port})

      Enum.each(frame_sessions, fn frames ->
        {:ok, socket} = :gen_tcp.accept(listen_socket)
        {:ok, request_payload} = :gen_tcp.recv(socket, 0, 1_000)
        request = Jason.decode!(request_payload)

        Enum.each(frames, fn frame ->
          response_payload =
            frame
            |> Map.put_new("rpc_version", request["rpc_version"])
            |> Map.put_new("id", request["id"])
            |> Jason.encode!()

          :ok = :gen_tcp.send(socket, response_payload)
        end)

        :gen_tcp.close(socket)
      end)

      :gen_tcp.close(listen_socket)
    end)
  end

  def wait_for_job(job_id, router_opts, attempts \\ 20)

  def wait_for_job(job_id, router_opts, attempts)
      when is_binary(job_id) and attempts > 0 do
    conn =
      :get
      |> conn("/api/v1/jobs/#{job_id}")
      |> Router.call(router_opts)

    payload = Jason.decode!(conn.resp_body)

    if payload["job"]["status"] in ["completed", "failed"] do
      payload
    else
      Process.sleep(10)
      wait_for_job(job_id, router_opts, attempts - 1)
    end
  end

  def wait_for_job(_job_id, _router_opts, 0),
    do: ExUnit.Assertions.flunk("timed out waiting for async job completion")

  def configure_fake_agent_pool(port) when is_integer(port) do
    Application.put_env(:kyuubiki_web, AgentPool,
      endpoints: [%{id: "agent-a", host: "127.0.0.1", port: port}]
    )

    AgentPool.reload()
  end

  def submit_catalog_workflow_job(router_opts, workflow_id, input_artifacts)
      when is_list(router_opts) and is_binary(workflow_id) and is_map(input_artifacts) do
    conn =
      :post
      |> conn(
        "/api/v1/workflows/catalog/#{workflow_id}/jobs",
        Jason.encode!(%{"input_artifacts" => input_artifacts})
      )
      |> put_req_header("content-type", "application/json")
      |> Router.call(router_opts)

    ExUnit.Assertions.assert(conn.status == 202)
    payload = Jason.decode!(conn.resp_body)
    wait_for_job(payload["job"]["job_id"], router_opts)
  end

  def start_guarded_quad_sessions(:blocked) do
    start_fake_agent_sessions(WorkflowApiFixtures.guarded_quad_frames(:blocked))
  end

  def start_guarded_quad_sessions(:continued) do
    start_fake_agent_sessions(WorkflowApiFixtures.guarded_quad_frames(:continued))
  end

  def start_electrostatic_quad_summary_session do
    start_fake_agent_sessions(WorkflowApiFixtures.electrostatic_quad_summary_frames())
  end

  def start_guarded_triangle_sessions(:blocked) do
    start_fake_agent_sessions(WorkflowApiFixtures.guarded_triangle_frames(:blocked))
  end

  def start_guarded_triangle_sessions(:continued) do
    start_fake_agent_sessions(WorkflowApiFixtures.guarded_triangle_frames(:continued))
  end

  def assert_guard_blocked(result_payload, workflow_id, skipped_nodes, hotspot_max) do
    ExUnit.Assertions.assert(result_payload["job"]["status"] == "completed")
    ExUnit.Assertions.assert(result_payload["result"]["workflow_id"] == workflow_id)
    ExUnit.Assertions.assert(length(result_payload["result"]["completed_nodes"]) == 7)
    ExUnit.Assertions.assert(length(result_payload["result"]["progress_events"]) == 7)

    ExUnit.Assertions.assert(
      Enum.member?(result_payload["result"]["completed_nodes"], "field_hotspots")
    )

    ExUnit.Assertions.refute(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_heat")
    )

    ExUnit.Assertions.refute(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_thermo")
    )

    ExUnit.Assertions.assert(
      MapSet.subset?(
        MapSet.new(skipped_nodes),
        MapSet.new(result_payload["result"]["skipped_nodes"])
      )
    )

    ExUnit.Assertions.assert(
      result_payload["result"]["branch_decisions"] == [
        %{"node_id" => "gate", "chosen_output" => "if_true", "predicate_result" => true}
      ]
    )

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    ExUnit.Assertions.assert(exported["format"] == "json")

    summary = Jason.decode!(exported["content"])
    ExUnit.Assertions.assert(summary["field_hotspot_count"] == 1)
    ExUnit.Assertions.assert(summary["field_hotspot_max"] == hotspot_max)
    ExUnit.Assertions.assert(summary["field_threshold"] == hotspot_max)
    ExUnit.Assertions.assert(summary["source_field"] == "electric_field_magnitude")
  end

  def assert_guard_continued(
        result_payload,
        workflow_id,
        expected_temperature_deltas,
        expected_summary
      ) do
    ExUnit.Assertions.assert(result_payload["job"]["status"] == "completed")
    ExUnit.Assertions.assert(result_payload["result"]["workflow_id"] == workflow_id)
    ExUnit.Assertions.assert(length(result_payload["result"]["completed_nodes"]) == 11)
    ExUnit.Assertions.assert(length(result_payload["result"]["progress_events"]) == 11)

    ExUnit.Assertions.refute(
      Enum.member?(result_payload["result"]["completed_nodes"], "field_hotspots")
    )

    ExUnit.Assertions.assert(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_heat")
    )

    ExUnit.Assertions.assert(
      Enum.member?(result_payload["result"]["completed_nodes"], "solve_thermo")
    )

    ExUnit.Assertions.assert(
      MapSet.equal?(
        MapSet.new(result_payload["result"]["skipped_nodes"]),
        MapSet.new(["field_hotspots"])
      )
    )

    ExUnit.Assertions.assert(
      result_payload["result"]["branch_decisions"] == [
        %{"node_id" => "gate", "chosen_output" => "if_false", "predicate_result" => false}
      ]
    )

    bridged_heat_model = result_payload["result"]["artifacts"]["bridge_field_to_heat.heat_model"]

    ExUnit.Assertions.assert(
      Enum.all?(bridged_heat_model["nodes"], fn node -> node["heat_load"] == 300.0 end)
    )

    bridged_thermo_model =
      result_payload["result"]["artifacts"]["bridge_temperature.thermo_model"]

    ExUnit.Assertions.assert(
      Enum.map(bridged_thermo_model["nodes"], & &1["temperature_delta"]) ==
        expected_temperature_deltas
    )

    exported = result_payload["result"]["artifacts"]["json_output.json"]
    ExUnit.Assertions.assert(exported["format"] == "json")

    summary = Jason.decode!(exported["content"])
    ExUnit.Assertions.assert(summary["max_displacement"] == expected_summary["max_displacement"])
    ExUnit.Assertions.assert(summary["max_stress"] == expected_summary["max_stress"])

    ExUnit.Assertions.assert(
      summary["max_temperature_delta"] == expected_summary["max_temperature_delta"]
    )
  end

  def heat_to_thermo_quad_input_artifacts do
    WorkflowApiFixtures.heat_to_thermo_quad_input_artifacts()
  end

  def electrostatic_plane_quad_input_artifacts do
    WorkflowApiFixtures.electrostatic_plane_quad_input_artifacts()
  end

  def electrostatic_plane_triangle_input_artifacts do
    WorkflowApiFixtures.electrostatic_plane_triangle_input_artifacts()
  end
end
