defmodule KyuubikiWeb.WorkflowMaterialStateRuntime do
  @moduledoc false

  def build_material_exploration_snapshot(payload, config)
      when is_map(payload) and is_map(config) do
    snapshot_id = Map.get(config, "snapshot_id", default_snapshot_id(payload))
    round_index = numeric_field(payload, "material_next_round_index")
    enabled = Map.get(payload, "material_next_round_enabled", false)

    {:ok,
     %{
       "material_exploration_snapshot_id" => snapshot_id,
       "material_exploration_snapshot_schema" => "kyuubiki.material_exploration_snapshot/v1",
       "material_exploration_status" => if(enabled, do: "active", else: "closed"),
       "material_exploration_round_index" => round_index,
       "material_exploration_next_action" =>
         Map.get(payload, "material_next_round_action", "unknown"),
       "material_exploration_seed_candidate_id" =>
         Map.get(payload, "material_next_round_seed_candidate_id", ""),
       "material_exploration_target_score" =>
         numeric_field(payload, "material_next_round_target_score"),
       "material_exploration_requested_candidate_count" =>
         numeric_field(payload, "material_next_round_requested_candidate_count") |> round(),
       "material_exploration_stop_reason" =>
         Map.get(payload, "material_next_round_stop_reason", ""),
       "material_exploration_checkpoint" => checkpoint(payload),
       "material_exploration_metadata" => Map.get(config, "metadata", %{})
     }}
  end

  def build_material_exploration_snapshot(_payload, _config),
    do: {:error, :invalid_material_exploration_snapshot_payload}

  defp default_snapshot_id(payload) do
    round = payload |> numeric_field("material_next_round_index") |> round()
    seed = Map.get(payload, "material_next_round_seed_candidate_id", "none")
    "material-exploration-r#{round}-#{seed}"
  end

  defp checkpoint(payload) do
    %{
      "next_round_enabled" => Map.get(payload, "material_next_round_enabled", false),
      "source_decision" => Map.get(payload, "material_next_round_source_decision", "unknown"),
      "constraints" => Map.get(payload, "material_next_round_constraints", %{}),
      "target_score" => numeric_field(payload, "material_next_round_target_score")
    }
  end

  defp numeric_field(payload, field) do
    case Map.get(payload, field) do
      value when is_number(value) -> value * 1.0
      _ -> 0.0
    end
  end
end
