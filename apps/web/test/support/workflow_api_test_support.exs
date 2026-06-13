defmodule KyuubikiWeb.TestSupport.WorkflowApi do
  @moduledoc false

  import Plug.Test

  alias KyuubikiWeb.Router

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

  def heat_to_thermo_quad_input_artifacts do
    %{
      "heat_model" => %{
        "nodes" => [
          %{"id" => "h0", "x" => 0.0, "y" => 0.0, "fix_temperature" => true, "temperature" => 100.0, "heat_load" => 0.0},
          %{"id" => "h1", "x" => 1.0, "y" => 0.0, "fix_temperature" => false, "temperature" => 0.0, "heat_load" => 0.0},
          %{"id" => "h2", "x" => 1.0, "y" => 1.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0},
          %{"id" => "h3", "x" => 0.0, "y" => 1.0, "fix_temperature" => true, "temperature" => 20.0, "heat_load" => 0.0}
        ],
        "elements" => [
          %{"id" => "hq0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "node_l" => 3, "thickness" => 0.02, "conductivity" => 45.0}
        ]
      }
    }
  end

  def electrostatic_plane_quad_input_artifacts do
    %{
      "electrostatic_model" => %{
        "nodes" => [
          %{"id" => "n0", "x" => 0.0, "y" => 0.0, "fix_potential" => true, "potential" => 10.0, "charge_density" => 0.0},
          %{"id" => "n1", "x" => 1.0, "y" => 0.0, "fix_potential" => true, "potential" => 0.0, "charge_density" => 0.0},
          %{"id" => "n2", "x" => 1.0, "y" => 1.0, "fix_potential" => true, "potential" => 0.0, "charge_density" => 0.0},
          %{"id" => "n3", "x" => 0.0, "y" => 1.0, "fix_potential" => true, "potential" => 10.0, "charge_density" => 0.0}
        ],
        "elements" => [
          %{"id" => "epq0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "node_l" => 3, "thickness" => 0.05, "permittivity" => 2.0}
        ]
      }
    }
  end

  def electrostatic_plane_triangle_input_artifacts do
    %{
      "electrostatic_plane_triangle_model" => %{
        "nodes" => [
          %{"id" => "e0", "x" => 0.0, "y" => 0.0, "fix_potential" => true, "potential" => 12.0},
          %{"id" => "e1", "x" => 1.0, "y" => 0.0, "fix_potential" => true, "potential" => 0.0},
          %{"id" => "e2", "x" => 0.0, "y" => 1.0, "fix_potential" => false, "charge_density" => 0.0}
        ],
        "elements" => [
          %{"id" => "et0", "node_i" => 0, "node_j" => 1, "node_k" => 2, "thickness" => 0.05, "permittivity" => 2.0}
        ]
      }
    }
  end
end
