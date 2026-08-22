defmodule KyuubikiWeb.Orchestra.HeadlessHandoffRegistry do
  @moduledoc """
  Supervised Orchestra ownership for GUI-to-headless handoff envelopes.

  The registry is intentionally bounded and in-memory. It replaces the former
  frontend-process registry without turning transient UI state into solver state.
  """

  use GenServer

  @schema "kyuubiki.headless-orchestra-handoff/v1"
  @max_entries 256

  def start_link(_options), do: GenServer.start_link(__MODULE__, %{}, name: __MODULE__)
  def register(payload), do: GenServer.call(__MODULE__, {:register, payload})
  def status(handoff_id), do: GenServer.call(__MODULE__, {:status, handoff_id})
  def snapshot(handoff_id), do: GenServer.call(__MODULE__, {:snapshot, handoff_id})
  def list, do: GenServer.call(__MODULE__, :list)
  def reset, do: GenServer.call(__MODULE__, :reset)

  @impl true
  def init(_state), do: {:ok, %{records: %{}, order: []}}

  @impl true
  def handle_call({:register, payload}, _from, state) do
    case validate(payload) do
      :ok ->
        handoff_id = handoff_id()

        record = %{
          receipt: build_receipt(handoff_id, payload),
          envelope: payload,
          monotonic_ms: System.monotonic_time(:millisecond)
        }

        records = Map.put(state.records, handoff_id, record)
        {records, order} = trim(records, [handoff_id | state.order])
        {:reply, {:ok, with_stage(record)}, %{records: records, order: order}}

      {:error, reason} ->
        {:reply, {:error, reason}, state}
    end
  end

  def handle_call({:status, handoff_id}, _from, state) do
    {:reply, lookup(state, handoff_id, &with_stage/1), state}
  end

  def handle_call({:snapshot, handoff_id}, _from, state) do
    reply =
      lookup(state, handoff_id, fn record ->
        Map.put(with_stage(record), "envelope", record.envelope)
      end)

    {:reply, reply, state}
  end

  def handle_call(:list, _from, state) do
    handoffs =
      state.order
      |> Enum.flat_map(fn handoff_id ->
        case Map.fetch(state.records, handoff_id) do
          {:ok, record} -> [with_stage(record)]
          :error -> []
        end
      end)

    {:reply, handoffs, state}
  end

  def handle_call(:reset, _from, _state), do: {:reply, :ok, %{records: %{}, order: []}}

  defp validate(payload) when is_map(payload) do
    required = [
      {"schema_version", &(&1 == @schema)},
      {"generated_at", &is_binary/1},
      {"workflow_id", &nonempty_string?/1},
      {"execution_batch", &is_map/1},
      {"dispatch_plan", &is_map/1},
      {"governance", &is_map/1},
      {"runtime_manifest", &is_map/1}
    ]

    case Enum.find(required, fn {key, validator} ->
           not validator.(Map.get(payload, key))
         end) do
      nil -> validate_nested(payload)
      {key, _validator} -> {:error, "invalid or missing #{key}"}
    end
  end

  defp validate(_payload), do: {:error, "handoff payload must be an object"}

  defp validate_nested(payload) do
    batch = payload["execution_batch"]
    plan = payload["dispatch_plan"]
    runtime = payload["runtime_manifest"]

    cond do
      not is_list(batch["steps"]) ->
        {:error, "execution_batch.steps must be an array"}

      not is_list(plan["steps"]) ->
        {:error, "dispatch_plan.steps must be an array"}

      not is_list(plan["warnings"]) ->
        {:error, "dispatch_plan.warnings must be an array"}

      not nonempty_string?(runtime["authority_mode"]) ->
        {:error, "runtime_manifest.authority_mode is required"}

      not is_list(runtime["target_clusters"]) ->
        {:error, "runtime_manifest.target_clusters must be an array"}

      true ->
        :ok
    end
  end

  defp build_receipt(handoff_id, payload) do
    overrides = list(payload["dispatch_overrides"])
    override_ref = map(payload["dispatch_override_ref"])

    override_count =
      if overrides == [], do: integer(override_ref["override_count"]), else: length(overrides)

    override_step_keys = overrides |> Enum.map(&map(&1)["step_key"]) |> Enum.filter(&is_binary/1)
    has_override = override_count > 0

    %{
      "accepted" => true,
      "handoff_id" => handoff_id,
      "workflow_id" => payload["workflow_id"],
      "received_at" => DateTime.utc_now() |> DateTime.to_iso8601(),
      "authority_mode" => payload["runtime_manifest"]["authority_mode"],
      "step_count" => payload["execution_batch"]["steps"] |> length(),
      "chosen_agent_count" => chosen_agent_count(payload["dispatch_plan"]["steps"]),
      "warning_count" => payload["dispatch_plan"]["warnings"] |> length(),
      "target_clusters" => payload["runtime_manifest"]["target_clusters"],
      "has_dispatch_override" => has_override,
      "dispatch_override_count" => override_count,
      "override_acknowledged" => has_override,
      "override_note" => override_note(has_override),
      "override_step_keys" => override_step_keys,
      "override_summary" => override_summary(override_count, override_step_keys)
    }
  end

  defp with_stage(record) do
    elapsed_ms = System.monotonic_time(:millisecond) - record.monotonic_ms
    Map.merge(record.receipt, stage(elapsed_ms))
  end

  defp stage(elapsed_ms) when elapsed_ms < 2_000 do
    %{
      "stage" => "received",
      "status_message" => "handoff accepted and waiting for queue admission"
    }
  end

  defp stage(elapsed_ms) when elapsed_ms < 5_000 do
    %{
      "stage" => "queued",
      "status_message" => "handoff is queued for orchestrator intake"
    }
  end

  defp stage(elapsed_ms) when elapsed_ms < 8_000 do
    %{
      "stage" => "dispatch_planned",
      "status_message" =>
        "dispatch plan has been materialized and is awaiting orchestrator pickup"
    }
  end

  defp stage(_elapsed_ms) do
    %{
      "stage" => "ready_for_orchestra",
      "status_message" => "handoff envelope is ready for orchestrator pickup"
    }
  end

  defp lookup(state, handoff_id, mapper) do
    case Map.fetch(state.records, handoff_id) do
      {:ok, record} -> {:ok, mapper.(record)}
      :error -> :error
    end
  end

  defp trim(records, order) do
    {keep, remove} = Enum.split(order, @max_entries)
    {Map.drop(records, remove), keep}
  end

  defp chosen_agent_count(steps) do
    Enum.count(steps, fn step ->
      step |> map() |> Map.get("chosen_agent_id") |> nonempty_string?()
    end)
  end

  defp handoff_id do
    suffix = System.unique_integer([:positive, :monotonic]) |> Integer.to_string(36)
    "handoff_#{suffix}"
  end

  defp override_note(true) do
    "Dispatch override was acknowledged for audit and UI visibility, but execution still follows the embedded dispatch plan in the current version."
  end

  defp override_note(false), do: nil

  defp override_summary(count, step_keys) when count > 0 do
    "#{count} override step(s): #{Enum.join(step_keys, ", ")}"
  end

  defp override_summary(_count, _step_keys), do: nil
  defp nonempty_string?(value), do: is_binary(value) and String.trim(value) != ""
  defp list(value) when is_list(value), do: value
  defp list(_value), do: []
  defp map(value) when is_map(value), do: value
  defp map(_value), do: %{}
  defp integer(value) when is_integer(value) and value >= 0, do: value
  defp integer(_value), do: 0
end
