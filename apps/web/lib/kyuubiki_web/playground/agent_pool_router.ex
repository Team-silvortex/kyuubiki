defmodule KyuubikiWeb.Playground.AgentPoolRouter do
  @moduledoc """
  Routes agent endpoints by RPC method, capability, placement, and authority hints.
  """

  @spec route([map()], String.t() | nil, keyword()) :: [map()]
  def route(endpoints, method, opts \\ []) when is_list(endpoints) and is_list(opts) do
    endpoints
    |> route_by_method(method)
    |> filter_by_constraints(method, opts)
  end

  defp route_by_method(endpoints, nil), do: endpoints

  defp route_by_method(endpoints, method) when is_binary(method) do
    tags = preferred_tags(method)

    {capable, remaining} = Enum.split_with(endpoints, &supports_method?(&1, method))

    case capable do
      [] ->
        {preferred_by_tags, fallback} =
          Enum.split_with(remaining, fn endpoint ->
            endpoint_tags(endpoint) |> Enum.any?(&(&1 in tags))
          end)

        sort_by_health_capacity(preferred_by_tags) ++ fallback

      _ ->
        sort_by_health_capacity(capable) ++ remaining
    end
  end

  defp filter_by_constraints(endpoints, method, opts) do
    required_capabilities = opts |> Keyword.get(:required_capabilities, []) |> normalize_values()
    placement_tags = opts |> Keyword.get(:placement_tags, []) |> normalize_values()
    orchestration = opts |> Keyword.get(:orchestration, %{}) |> normalize_orchestration()
    lease = opts |> Keyword.get(:execution_lease, %{}) |> normalize_lease()

    if required_capabilities == [] and placement_tags == [] and orchestration == %{} and
         lease == %{} do
      endpoints
    else
      endpoints
      |> Enum.reduce({[], []}, fn endpoint, {typed, legacy} ->
        case classify(
               endpoint,
               method,
               required_capabilities,
               placement_tags,
               orchestration,
               lease
             ) do
          {:typed_match, score} -> {[{endpoint, score} | typed], legacy}
          :legacy_fallback -> {typed, [endpoint | legacy]}
          :explicit_mismatch -> {typed, legacy}
        end
      end)
      |> order_matches()
    end
  end

  defp classify(endpoint, method, required_capabilities, placement_tags, orchestration, lease) do
    capability_status = capability_match_status(endpoint, method, required_capabilities)
    tag_status = tag_match_status(endpoint, placement_tags)
    orchestration_status = orchestration_match_status(endpoint, orchestration)
    lease_status = execution_lease_status(endpoint, lease)

    cond do
      :mismatch in [capability_status, tag_status, orchestration_status, lease_status] ->
        :explicit_mismatch

      :unknown in [capability_status, tag_status, orchestration_status, lease_status] ->
        :legacy_fallback

      true ->
        {:typed_match,
         endpoint_match_score(endpoint, required_capabilities, placement_tags, orchestration)}
    end
  end

  defp order_matches({typed_matches, legacy_fallbacks}) do
    typed_matches
    |> Enum.sort_by(fn {_endpoint, score} -> -score end)
    |> Enum.map(&elem(&1, 0))
    |> Kernel.++(Enum.reverse(legacy_fallbacks))
  end

  defp capability_match_status(_endpoint, _method, []), do: :match

  defp capability_match_status(endpoint, method, required_capabilities) do
    capability_ids = endpoint |> Map.get(:capabilities, []) |> Enum.map(& &1.id)

    has_metadata? =
      capability_ids != [] or Map.get(endpoint, :methods, []) != [] or
        is_binary(Map.get(endpoint, :role)) or
        Enum.any?(Map.get(endpoint, :capabilities, []), &is_binary(&1.role))

    cond do
      not has_metadata? ->
        :unknown

      Enum.all?(
        required_capabilities,
        &supports_capability?(endpoint, method, &1, capability_ids)
      ) ->
        :match

      true ->
        :mismatch
    end
  end

  defp supports_capability?(endpoint, method, "solver_rpc", capability_ids) do
    "solver_rpc" in capability_ids or Map.get(endpoint, :role) == "solver" or
      supports_method?(endpoint, method || "") or
      Enum.any?(Map.get(endpoint, :capabilities, []), &(&1.role == "solver"))
  end

  defp supports_capability?(endpoint, _method, capability_id, capability_ids) do
    capability_id in capability_ids or Map.get(endpoint, :role) == capability_id
  end

  defp tag_match_status(_endpoint, []), do: :match

  defp tag_match_status(endpoint, placement_tags) do
    tags = endpoint_tags(endpoint)

    cond do
      tags == [] -> :unknown
      Enum.any?(placement_tags, &(&1 in tags)) -> :match
      true -> :mismatch
    end
  end

  defp orchestration_match_status(_endpoint, orchestration) when orchestration == %{}, do: :match

  defp orchestration_match_status(endpoint, orchestration) do
    has_metadata? =
      Enum.any?(
        [:control_mode, :orch_id, :cluster_id, :orch_session_id],
        &is_binary(Map.get(endpoint, &1))
      )

    if has_metadata? do
      authority_matches?(endpoint, orchestration)
    else
      :unknown
    end
  end

  defp authority_matches?(endpoint, %{control_mode: "offline_mesh"} = orchestration) do
    cluster_id = Map.get(orchestration, :cluster_id)

    if Map.get(endpoint, :control_mode) == "offline_mesh" and
         (is_nil(cluster_id) or cluster_id == Map.get(endpoint, :cluster_id)) do
      :match
    else
      :mismatch
    end
  end

  defp authority_matches?(endpoint, %{control_mode: "orch_managed"} = orchestration) do
    orch_matches? =
      is_nil(orchestration[:orch_id]) or orchestration[:orch_id] == endpoint[:orch_id]

    session_matches? =
      is_nil(orchestration[:orch_session_id]) or
        orchestration[:orch_session_id] == endpoint[:orch_session_id]

    if endpoint[:control_mode] == "orch_managed" and orch_matches? and session_matches?,
      do: :match,
      else: :mismatch
  end

  defp authority_matches?(_endpoint, _orchestration), do: :match

  defp execution_lease_status(endpoint, lease) when lease == %{} do
    if Map.get(endpoint, :active_lease), do: :mismatch, else: :match
  end

  defp execution_lease_status(endpoint, lease) do
    case Map.get(endpoint, :active_lease) do
      nil -> :match
      %{"lease_id" => lease_id} -> if lease_id == lease[:lease_id], do: :match, else: :mismatch
      %{lease_id: lease_id} -> if lease_id == lease[:lease_id], do: :match, else: :mismatch
      _ -> :unknown
    end
  end

  defp endpoint_match_score(endpoint, required_capabilities, placement_tags, orchestration) do
    capability_ids = endpoint |> Map.get(:capabilities, []) |> Enum.map(& &1.id)
    tags = endpoint_tags(endpoint)

    capability_score =
      Enum.count(required_capabilities, &(&1 in capability_ids or endpoint[:role] == &1))

    tag_score = Enum.count(placement_tags, &(&1 in tags))

    orchestration_score(endpoint, orchestration) + capability_score * 10 + tag_score
  end

  defp orchestration_score(endpoint, %{control_mode: "orch_managed"}) do
    if endpoint[:control_mode] == "orch_managed", do: 100, else: 0
  end

  defp orchestration_score(endpoint, %{control_mode: "offline_mesh"} = orchestration) do
    base_score = if endpoint[:control_mode] == "offline_mesh", do: 100, else: 0

    if is_binary(orchestration[:cluster_id]) and orchestration[:cluster_id] != "" and
         endpoint[:cluster_id] == orchestration[:cluster_id],
       do: base_score + 20,
       else: base_score
  end

  defp orchestration_score(_endpoint, _orchestration), do: 0

  defp endpoint_tags(endpoint) do
    direct = endpoint |> Map.get(:tags, []) |> List.wrap() |> Enum.filter(&is_binary/1)

    capability_tags =
      endpoint
      |> Map.get(:capabilities, [])
      |> List.wrap()
      |> Enum.flat_map(
        &(&1
          |> Map.get(:tags, [])
          |> List.wrap()
          |> Enum.filter(fn tag -> is_binary(tag) end))
      )

    Enum.uniq(direct ++ capability_tags)
  end

  defp supports_method?(endpoint, method) when is_binary(method) do
    direct_methods = endpoint |> Map.get(:methods, []) |> List.wrap() |> Enum.filter(&is_binary/1)

    capability_methods =
      endpoint
      |> Map.get(:capabilities, [])
      |> List.wrap()
      |> Enum.flat_map(
        &(&1
          |> Map.get(:methods, [])
          |> List.wrap()
          |> Enum.filter(fn method -> is_binary(method) end))
      )

    method in Enum.uniq(direct_methods ++ capability_methods)
  end

  defp sort_by_health_capacity(endpoints) do
    endpoints
    |> Enum.with_index()
    |> Enum.sort_by(fn {endpoint, index} ->
      {-(Map.get(endpoint, :health_score) || 100), -Map.get(endpoint, :capacity, 1), index}
    end)
    |> Enum.map(&elem(&1, 0))
  end

  defp normalize_values(values) when is_list(values) do
    values |> Enum.filter(&is_binary/1) |> Enum.reject(&(&1 == "")) |> Enum.uniq()
  end

  defp normalize_values(_values), do: []

  defp normalize_orchestration(%{} = context) do
    %{}
    |> put_value(:control_mode, context[:control_mode] || context["control_mode"])
    |> put_value(:orch_id, context[:orch_id] || context["orch_id"])
    |> put_value(:orch_session_id, context[:orch_session_id] || context["orch_session_id"])
    |> put_value(:cluster_id, context[:cluster_id] || context["cluster_id"])
  end

  defp normalize_orchestration(_context), do: %{}

  defp normalize_lease(%{} = lease),
    do: %{} |> put_value(:lease_id, lease[:lease_id] || lease["lease_id"])

  defp normalize_lease(_lease), do: %{}

  defp put_value(context, _key, nil), do: context
  defp put_value(context, key, value) when is_binary(value), do: Map.put(context, key, value)
  defp put_value(context, _key, _value), do: context

  defp preferred_tags("solve_bar_1d"), do: ["bar"]
  defp preferred_tags("solve_thermal_bar_1d"), do: ["bar", "thermal", "line"]
  defp preferred_tags("solve_heat_bar_1d"), do: ["heat", "bar", "line"]

  defp preferred_tags("solve_electrostatic_bar_1d"),
    do: ["electromagnetic", "electrostatic", "bar", "line"]

  defp preferred_tags("solve_electrostatic_plane_triangle_2d"),
    do: ["electromagnetic", "electrostatic", "plane", "triangle", "mesh"]

  defp preferred_tags("solve_electrostatic_plane_quad_2d"),
    do: ["electromagnetic", "electrostatic", "plane", "quad", "mesh"]

  defp preferred_tags("solve_heat_plane_triangle_2d"), do: ["heat", "plane", "mesh"]
  defp preferred_tags("solve_heat_plane_quad_2d"), do: ["heat", "plane", "mesh", "quad"]
  defp preferred_tags("solve_thermal_truss_2d"), do: ["truss", "thermal", "plane"]
  defp preferred_tags("solve_thermal_truss_3d"), do: ["truss", "thermal", "space"]
  defp preferred_tags("solve_spring_1d"), do: ["spring", "line", "support"]
  defp preferred_tags("solve_spring_2d"), do: ["spring", "plane", "support"]
  defp preferred_tags("solve_spring_3d"), do: ["spring", "space", "support"]
  defp preferred_tags("solve_beam_1d"), do: ["beam", "bending", "line"]
  defp preferred_tags("solve_thermal_beam_1d"), do: ["beam", "thermal", "bending", "line"]
  defp preferred_tags("solve_thermal_frame_2d"), do: ["frame", "thermal", "beam", "bending"]

  defp preferred_tags("solve_thermal_frame_3d"),
    do: ["frame", "thermal", "space", "beam", "bending"]

  defp preferred_tags("solve_torsion_1d"), do: ["torsion", "shaft", "line"]
  defp preferred_tags("solve_truss_2d"), do: ["truss"]
  defp preferred_tags("solve_truss_3d"), do: ["truss", "space"]
  defp preferred_tags("solve_plane_triangle_2d"), do: ["plane", "mesh"]
  defp preferred_tags("solve_thermal_plane_triangle_2d"), do: ["plane", "thermal", "mesh"]
  defp preferred_tags("solve_plane_quad_2d"), do: ["plane", "mesh", "quad"]
  defp preferred_tags("solve_thermal_plane_quad_2d"), do: ["plane", "thermal", "mesh", "quad"]
  defp preferred_tags("solve_frame_2d"), do: ["frame", "beam", "bending"]
  defp preferred_tags("solve_frame_3d"), do: ["frame", "space", "beam", "bending"]
  defp preferred_tags(_method), do: ["general", "cpu"]
end
