defmodule KyuubikiSdk.SolverRpcClient do
  @moduledoc "TCP RPC client for kyuubiki.solver-rpc/v1."

  alias KyuubikiSdk.Error

  @solver_methods %{
    "bar_1d" => "solve_bar_1d",
    "thermal_bar_1d" => "solve_thermal_bar_1d",
    "heat_bar_1d" => "solve_heat_bar_1d",
    "electrostatic_bar_1d" => "solve_electrostatic_bar_1d",
    "magnetostatic_bar_1d" => "solve_magnetostatic_bar_1d",
    "magnetostatic_plane_triangle_2d" => "solve_magnetostatic_plane_triangle_2d",
    "magnetostatic_plane_quad_2d" => "solve_magnetostatic_plane_quad_2d",
    "acoustic_bar_1d" => "solve_acoustic_bar_1d",
    "beam_1d" => "solve_beam_1d",
    "thermal_beam_1d" => "solve_thermal_beam_1d",
    "torsion_1d" => "solve_torsion_1d",
    "spring_1d" => "solve_spring_1d",
    "nonlinear_spring_1d" => "solve_nonlinear_spring_1d",
    "contact_gap_1d" => "solve_contact_gap_1d",
    "spring_2d" => "solve_spring_2d",
    "spring_3d" => "solve_spring_3d",
    "truss_2d" => "solve_truss_2d",
    "thermal_truss_2d" => "solve_thermal_truss_2d",
    "frame_2d" => "solve_frame_2d",
    "modal_frame_2d" => "solve_modal_frame_2d",
    "thermal_frame_2d" => "solve_thermal_frame_2d",
    "plane_triangle_2d" => "solve_plane_triangle_2d",
    "heat_plane_triangle_2d" => "solve_heat_plane_triangle_2d",
    "thermal_plane_triangle_2d" => "solve_thermal_plane_triangle_2d",
    "electrostatic_plane_triangle_2d" => "solve_electrostatic_plane_triangle_2d",
    "plane_quad_2d" => "solve_plane_quad_2d",
    "heat_plane_quad_2d" => "solve_heat_plane_quad_2d",
    "thermal_plane_quad_2d" => "solve_thermal_plane_quad_2d",
    "electrostatic_plane_quad_2d" => "solve_electrostatic_plane_quad_2d",
    "stokes_flow_quad_2d" => "solve_stokes_flow_plane_quad_2d",
    "truss_3d" => "solve_truss_3d",
    "thermal_truss_3d" => "solve_thermal_truss_3d",
    "frame_3d" => "solve_frame_3d",
    "modal_frame_3d" => "solve_modal_frame_3d",
    "thermal_frame_3d" => "solve_thermal_frame_3d"
  }

  defstruct [:host, :port, :timeout]

  def new(host, port, opts \\ []) do
    %__MODULE__{
      host: to_charlist(host),
      port: port,
      timeout: Keyword.get(opts, :timeout, 15_000)
    }
  end

  def ping(client), do: call(client, "ping", %{})
  def describe_agent(client), do: call(client, "describe_agent", %{})
  def solve_bar_1d(client, payload), do: solve_study(client, "bar_1d", payload)
  def solve_truss_2d(client, payload), do: solve_study(client, "truss_2d", payload)
  def solve_truss_3d(client, payload), do: solve_study(client, "truss_3d", payload)
  def solve_modal_frame_2d(client, payload), do: solve_study(client, "modal_frame_2d", payload)
  def solve_modal_frame_3d(client, payload), do: solve_study(client, "modal_frame_3d", payload)

  def solve_nonlinear_spring_1d(client, payload),
    do: solve_study(client, "nonlinear_spring_1d", payload)

  def solve_contact_gap_1d(client, payload), do: solve_study(client, "contact_gap_1d", payload)

  def solve_acoustic_bar_1d(client, payload), do: solve_study(client, "acoustic_bar_1d", payload)

  def solve_stokes_flow_quad_2d(client, payload),
    do: solve_study(client, "stokes_flow_quad_2d", payload)

  def solve_plane_triangle_2d(client, payload),
    do: solve_study(client, "plane_triangle_2d", payload)

  def solve_magnetostatic_plane_triangle_2d(client, payload),
    do: solve_study(client, "magnetostatic_plane_triangle_2d", payload)

  def solve_magnetostatic_plane_quad_2d(client, payload),
    do: solve_study(client, "magnetostatic_plane_quad_2d", payload)

  def cancel_job(client, job_id), do: call(client, "cancel_job", %{"job_id" => job_id})

  def solve_study(client, solve_kind, payload) do
    case Map.fetch(@solver_methods, normalize_solve_kind(solve_kind)) do
      {:ok, method} -> call(client, method, payload)
      :error -> {:error, Error.rpc("unsupported solve kind: #{solve_kind}")}
    end
  end

  def call(client, method, params) do
    with {:ok, socket} <-
           :gen_tcp.connect(
             client.host,
             client.port,
             [:binary, active: false, packet: 0],
             client.timeout
           ),
         :ok <- :gen_tcp.send(socket, encode_frame(method, params)),
         {:ok, response} <- recv_until_response(socket, []) do
      :gen_tcp.close(socket)
      {:ok, response}
    else
      {:error, %Error{} = error} -> {:error, error}
      {:error, reason} -> {:error, Error.transport(inspect(reason))}
      error -> {:error, Error.transport(inspect(error))}
    end
  end

  defp encode_frame(method, params) do
    payload =
      Jason.encode!(%{
        rpc_version: 1,
        id: Integer.to_string(System.unique_integer([:positive])),
        method: method,
        params: params
      })

    <<byte_size(payload)::unsigned-big-32, payload::binary>>
  end

  defp recv_until_response(socket, progress_frames) do
    with {:ok, <<size::unsigned-big-32>>} <- :gen_tcp.recv(socket, 4),
         {:ok, payload} <- :gen_tcp.recv(socket, size) do
      frame = Jason.decode!(payload)

      cond do
        Map.has_key?(frame, "event") ->
          recv_until_response(socket, [frame | progress_frames])

        frame["ok"] == true ->
          {:ok, %{result: frame["result"], progress_frames: Enum.reverse(progress_frames)}}

        true ->
          error = frame["error"] || %{}
          {:error, Error.rpc(error["message"] || "rpc failed", code: error["code"])}
      end
    end
  end

  defp normalize_solve_kind(kind) when is_atom(kind),
    do: normalize_solve_kind(Atom.to_string(kind))

  defp normalize_solve_kind("axial_bar_1d"), do: "bar_1d"
  defp normalize_solve_kind("stokes_flow_plane_quad_2d"), do: "stokes_flow_quad_2d"
  defp normalize_solve_kind(kind) when is_binary(kind), do: String.downcase(kind)
end
