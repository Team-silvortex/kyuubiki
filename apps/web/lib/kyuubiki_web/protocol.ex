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
        "projects" => "/api/v1/projects"
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
        "solve_truss_2d",
        "solve_truss_3d",
        "solve_plane_triangle_2d",
        "cancel_job"
      ]
    }
  end

  @spec describe_agents() :: [map()]
  def describe_agents do
    AgentPool.endpoints()
    |> Enum.map(&describe_agent/1)
  end

  defp describe_agent(endpoint) do
    base = %{
      "id" => endpoint.id,
      "host" => endpoint.host,
      "port" => endpoint.port,
      "tags" => endpoint |> Map.get(:tags, []) |> List.wrap(),
      "capacity" => Map.get(endpoint, :capacity),
      "region" => Map.get(endpoint, :region),
      "zone" => Map.get(endpoint, :zone),
      "role" => Map.get(endpoint, :role)
    }

    case AgentClient.describe_agent(endpoint) do
      {:ok, descriptor} ->
        Map.put(base, "descriptor", descriptor)

      {:error, reason} ->
        Map.put(base, "descriptor_error", inspect(reason))
    end
  end
end
