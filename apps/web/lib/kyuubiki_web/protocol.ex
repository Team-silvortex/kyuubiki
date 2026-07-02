defmodule KyuubikiWeb.Protocol do
  @moduledoc """
  Public protocol descriptors for the independently deployable Kyuubiki programs.
  """

  alias KyuubikiWeb.Playground.AgentClient
  alias KyuubikiWeb.Playground.AgentPool

  @solver_rpc_protocol "kyuubiki.solver-rpc/v1"
  @control_plane_protocol "kyuubiki.control-plane/http-v1"
  @rpc_version 1

  @spec descriptor() :: map()
  def descriptor do
    %{
      "program" => "kyuubiki-orchestrator",
      "role" => "control_plane",
      "protocol" => control_plane_protocol(),
      "compatible_solver_rpc" => solver_rpc_protocol(),
      "authority" => control_plane_authority(),
      "programs" => [
        %{
          "program" => "kyuubiki-frontend",
          "role" => "gui_client",
          "transport" => %{
            "kind" => "http",
            "encoding" => "json"
          }
        },
        %{
          "program" => "kyuubiki-orchestrator",
          "role" => "control_plane",
          "transport" => %{
            "kind" => "http",
            "encoding" => "json"
          }
        },
        %{
          "program" => "kyuubiki-rust-agent",
          "role" => "solver_agent",
          "transport" => %{
            "kind" => "tcp",
            "framing" => "length_prefixed_u32",
            "encoding" => "json"
          }
        }
      ]
    }
  end

  @spec control_plane_protocol() :: map()
  def control_plane_protocol do
    %{
      "name" => @control_plane_protocol,
      "version" => 1,
      "transport" => %{
        "kind" => "http",
        "encoding" => "json"
      },
      "resources" => %{
        "health" => "/api/health",
        "protocol" => "/api/v1/protocol",
        "agents" => "/api/v1/agents",
        "jobs" => "/api/v1/jobs",
        "results" => "/api/v1/results",
        "operators" => "/api/v1/operators",
        "operator_descriptor" => "/api/v1/operators/:operator_id",
        "store" => "/api/v1/store",
        "store_sources" => "/api/v1/store/sources",
        "store_entry" => "/api/v1/store/:kind/:entry_id",
        "workflow_graph_run" => "/api/v1/workflows/graph/run",
        "workflow_graph_jobs" => "/api/v1/workflows/graph/jobs",
        "workflow_catalog" => "/api/v1/workflows/catalog",
        "workflow_catalog_job_submit" => "/api/v1/workflows/catalog/:workflow_id/jobs",
        "projects" => "/api/v1/projects",
        "workload_catalog" => "/api/v1/workloads/catalog",
        "project_bundle_download" => "/api/v1/projects/:project_id/bundle"
      }
    }
  end

  @spec solver_rpc_protocol() :: map()
  def solver_rpc_protocol do
    %{
      "name" => @solver_rpc_protocol,
      "rpc_version" => @rpc_version,
      "transport" => %{
        "kind" => "tcp",
        "framing" => "length_prefixed_u32",
        "encoding" => "json"
      },
      "methods" => [
        "ping",
        "describe_agent",
        "solve_bar_1d",
        "solve_thermal_bar_1d",
        "solve_heat_bar_1d",
        "solve_transient_heat_bar_1d",
        "solve_electrostatic_bar_1d",
        "solve_electrostatic_plane_triangle_2d",
        "solve_electrostatic_plane_quad_2d",
        "solve_heat_plane_triangle_2d",
        "solve_heat_plane_quad_2d",
        "solve_thermal_truss_2d",
        "solve_thermal_truss_3d",
        "solve_spring_1d",
        "solve_transient_spring_1d",
        "solve_harmonic_spring_1d",
        "solve_spring_2d",
        "solve_spring_3d",
        "solve_beam_1d",
        "solve_thermal_beam_1d",
        "solve_thermal_frame_2d",
        "solve_thermal_frame_3d",
        "solve_torsion_1d",
        "solve_truss_2d",
        "solve_truss_3d",
        "solve_plane_triangle_2d",
        "solve_thermal_plane_triangle_2d",
        "solve_plane_quad_2d",
        "solve_thermal_plane_quad_2d",
        "solve_frame_2d",
        "solve_frame_3d",
        "cancel_job"
      ]
    }
  end

  @spec describe_agents() :: [map()]
  def describe_agents do
    AgentPool.endpoints()
    |> Enum.map(&describe_agent/1)
  end

  defp control_plane_authority do
    %{
      "control_mode" => "orch_managed",
      "authority_mode" => "single_orchestrator",
      "session_state" => "orch_bound_pending_session",
      "orchestrator_id" => "orchestra/default",
      "orchestrator_session_id" => nil,
      "accepts_multi_orchestrator_binding" => false,
      "agent_library_replication" => "central_fetch"
    }
  end

  defp describe_agent(endpoint) do
    base = %{
      "id" => endpoint.id,
      "host" => endpoint.host,
      "port" => endpoint.port,
      "tags" => endpoint |> Map.get(:tags, []) |> List.wrap(),
      "methods" => endpoint |> Map.get(:methods, []) |> List.wrap(),
      "capabilities" => endpoint |> Map.get(:capabilities, []) |> List.wrap(),
      "health_score" => Map.get(endpoint, :health_score),
      "capacity" => Map.get(endpoint, :capacity),
      "region" => Map.get(endpoint, :region),
      "zone" => Map.get(endpoint, :zone),
      "role" => Map.get(endpoint, :role)
    }

    case AgentClient.describe_agent(endpoint) do
      {:ok, descriptor} ->
        Map.put(base, "descriptor", merge_descriptor_authority(descriptor, endpoint))

      {:error, reason} ->
        Map.put(base, "descriptor_error", inspect(reason))
    end
  end

  defp merge_descriptor_authority(descriptor, endpoint) when is_map(descriptor) do
    authority =
      descriptor
      |> Map.get("authority")
      |> normalize_descriptor_authority(endpoint, descriptor)

    Map.put(descriptor, "authority", authority)
  end

  defp normalize_descriptor_authority(nil, endpoint, descriptor) do
    runtime_mode =
      descriptor
      |> get_in(["runtime", "runtime_mode"])
      |> to_string()

    endpoint_authority(endpoint, runtime_mode)
  end

  defp normalize_descriptor_authority(authority, endpoint, descriptor) when is_map(authority) do
    runtime_mode =
      descriptor
      |> get_in(["runtime", "runtime_mode"])
      |> to_string()

    synthesized = endpoint_authority(endpoint, runtime_mode)

    Map.merge(synthesized, authority)
    |> maybe_fill_nil("orchestrator_id", synthesized["orchestrator_id"])
    |> maybe_fill_nil("orchestrator_session_id", synthesized["orchestrator_session_id"])
  end

  defp endpoint_authority(endpoint, runtime_mode) do
    control_mode = Map.get(endpoint, :control_mode)
    orch_id = Map.get(endpoint, :orch_id)
    orch_session_id = Map.get(endpoint, :orch_session_id)
    session_state = Map.get(endpoint, :session_state)

    case control_mode || runtime_mode do
      "offline_mesh" ->
        %{
          "control_mode" => "offline_mesh",
          "authority_mode" => "offline_mesh",
          "session_state" => session_state || "offline_mesh",
          "orchestrator_id" => nil,
          "orchestrator_session_id" => nil,
          "accepts_multi_orchestrator_binding" => false,
          "agent_library_replication" => "central_fetch"
        }

      "orchestrated" ->
        %{
          "control_mode" => "orch_managed",
          "authority_mode" => "single_orchestrator",
          "session_state" => session_state || default_session_state(orch_session_id),
          "orchestrator_id" => orch_id,
          "orchestrator_session_id" => orch_session_id,
          "accepts_multi_orchestrator_binding" => false,
          "agent_library_replication" => "central_fetch"
        }

      "orch_managed" ->
        %{
          "control_mode" => "orch_managed",
          "authority_mode" => "single_orchestrator",
          "session_state" => session_state || default_session_state(orch_session_id),
          "orchestrator_id" => orch_id,
          "orchestrator_session_id" => orch_session_id,
          "accepts_multi_orchestrator_binding" => false,
          "agent_library_replication" => "central_fetch"
        }

      _ ->
        %{
          "control_mode" => "standalone",
          "authority_mode" => "self_directed",
          "orchestrator_id" => nil,
          "orchestrator_session_id" => nil,
          "accepts_multi_orchestrator_binding" => false,
          "agent_library_replication" => "central_fetch"
        }
    end
  end

  defp maybe_fill_nil(map, key, fallback) do
    if Map.get(map, key) == nil, do: Map.put(map, key, fallback), else: map
  end

  defp default_session_state(session_id) when is_binary(session_id) and session_id != "",
    do: "orch_session_bound"

  defp default_session_state(_session_id), do: "orch_bound_pending_session"
end
