defmodule KyuubikiWeb.WorkflowGraphInputs do
  @moduledoc false

  @enveloped_operators ~w(
    transform.benchmark_structural_pair
    transform.benchmark_acoustic_pair
    transform.benchmark_modal_pair
    transform.benchmark_dynamic_pair
    transform.benchmark_transport_pair
    transform.benchmark_coupled_heat_pair
    transform.benchmark_electrostatic_pair
    transform.benchmark_magnetostatic_pair
    transform.benchmark_cfd_pair
    transform.join_parameter_sweep_results
  )
  @named_operators @enveloped_operators ++
                     ~w(
                       transform.merge_summary_pair
                       transform.compare_summary_pair
                       transform.aggregate_summary_collection
                       transform.select_best_summary
                       transform.compose_quality_objective
                       transform.compose_diagnostics_bundle
                       transform.compose_diagnostics_report_payload
                       transform.resolve_focus_bridge_execution
                       transform.validate_electrostatic_heat_bridge
                       transform.validate_heat_thermo_bridge
                     )

  # Call only after endpoint validation. Count connected edges, not spare ports.
  def validate_bindings(nodes, edges) do
    counts = Enum.frequencies_by(edges, &get_in(&1, ["to", "node"]))

    Enum.reduce_while(nodes, :ok, fn node, :ok ->
      count = Map.get(counts, node["id"], 0)

      cond do
        node["kind"] == "input" and count > 0 ->
          {:halt, {:error, "workflow input node #{node["id"]} cannot have incoming edges"}}

        count > 1 and node["kind"] != "output" and not multiple_inputs?(node) ->
          {:halt,
           {:error,
            "workflow node #{node["id"]} uses a single input but has #{count} incoming edges"}}

        true ->
          {:cont, :ok}
      end
    end)
  end

  def partial_inputs?(%{"kind" => "transform", "operator_id" => "transform.first_available"}),
    do: true

  def partial_inputs?(_node), do: false

  def named_inputs?(%{"kind" => "transform", "operator_id" => id})
      when id in @named_operators,
      do: true

  def named_inputs?(_node), do: false

  def input_envelope?(%{"kind" => "transform", "operator_id" => id})
      when id in @enveloped_operators,
      do: true

  def input_envelope?(_node), do: false

  defp multiple_inputs?(node), do: partial_inputs?(node) or named_inputs?(node)
end
