defmodule KyuubikiWeb.Jobs.ProgressEvent do
  @moduledoc """
  Streamed runtime event consumed by the orchestration layer.
  """

  alias KyuubikiWeb.Jobs.Job

  @job_id_max_bytes 128
  @message_max_bytes 4_096

  @enforce_keys [:job_id, :stage, :progress]
  defstruct [
    :job_id,
    :stage,
    :progress,
    :residual,
    :iteration,
    :peak_memory,
    :message,
    :emitted_at
  ]

  @type t :: %__MODULE__{
          job_id: String.t(),
          stage: Job.status(),
          progress: float(),
          residual: float() | nil,
          iteration: non_neg_integer() | nil,
          peak_memory: non_neg_integer() | nil,
          message: String.t() | nil,
          emitted_at: DateTime.t() | nil
        }

  @spec new(map()) :: {:ok, t()} | {:error, term()}
  def new(attrs) when is_map(attrs) do
    with {:ok, job_id} <- fetch_required_string(attrs, :job_id),
         {:ok, stage} <- fetch_stage(attrs),
         {:ok, progress} <- fetch_progress(attrs),
         :ok <- validate_stage_progress(stage, progress),
         {:ok, residual} <- fetch_optional_non_negative_number(attrs, :residual),
         {:ok, iteration} <- fetch_optional_integer(attrs, :iteration),
         {:ok, peak_memory} <- fetch_optional_integer(attrs, :peak_memory),
         {:ok, message} <- fetch_optional_message(attrs, :message),
         {:ok, emitted_at} <- fetch_emitted_at(attrs) do
      {:ok,
       %__MODULE__{
         job_id: job_id,
         stage: stage,
         progress: progress,
         residual: residual,
         iteration: iteration,
         peak_memory: peak_memory,
         message: message,
         emitted_at: emitted_at
       }}
    end
  end

  defp fetch_required_string(attrs, key) do
    case Map.get(attrs, key) do
      value
      when is_binary(value) and byte_size(value) > 0 and byte_size(value) <= @job_id_max_bytes ->
        if String.trim(value) != "" and not Regex.match?(~r/[[:cntrl:]]/u, value),
          do: {:ok, value},
          else: {:error, {:invalid_or_missing, key}}

      _ ->
        {:error, {:invalid_or_missing, key}}
    end
  end

  defp fetch_stage(attrs) do
    case Map.get(attrs, :stage) do
      value when is_atom(value) ->
        if value in Job.statuses() do
          {:ok, value}
        else
          {:error, {:invalid_stage, value}}
        end

      value when is_binary(value) ->
        atom = Enum.find(Job.statuses(), fn status -> Atom.to_string(status) == value end)

        if atom do
          {:ok, atom}
        else
          {:error, {:invalid_stage, value}}
        end

      value ->
        {:error, {:invalid_stage, value}}
    end
  end

  defp fetch_progress(attrs) do
    case Map.get(attrs, :progress) do
      value when is_integer(value) and value >= 0 and value <= 1 -> {:ok, value * 1.0}
      value when is_float(value) and value >= 0.0 and value <= 1.0 -> {:ok, value}
      _ -> {:error, :invalid_progress}
    end
  end

  defp validate_stage_progress(:completed, 1.0), do: :ok

  defp validate_stage_progress(:completed, _progress),
    do: {:error, :completed_progress_must_equal_one}

  defp validate_stage_progress(_stage, _progress), do: :ok

  defp fetch_optional_message(attrs, key) do
    case Map.get(attrs, key) do
      nil -> {:ok, nil}
      value when is_binary(value) and byte_size(value) == 0 -> {:ok, nil}
      value when is_binary(value) and byte_size(value) <= @message_max_bytes -> {:ok, value}
      value when is_binary(value) -> {:error, {:message_too_large, byte_size(value)}}
      _ -> {:error, {:invalid_message, key}}
    end
  end

  defp fetch_optional_non_negative_number(attrs, key) do
    case Map.get(attrs, key) do
      nil -> {:ok, nil}
      value when is_integer(value) and value >= 0 -> {:ok, value * 1.0}
      value when is_float(value) and value >= 0.0 -> {:ok, value}
      _ -> {:error, {:invalid_non_negative_number, key}}
    end
  end

  defp fetch_optional_integer(attrs, key) do
    case Map.get(attrs, key) do
      nil -> {:ok, nil}
      value when is_integer(value) and value >= 0 -> {:ok, value}
      _ -> {:error, {:invalid_non_negative_integer, key}}
    end
  end

  defp fetch_emitted_at(attrs) do
    case Map.get(attrs, :emitted_at) do
      nil -> {:ok, DateTime.utc_now()}
      %DateTime{} = value -> {:ok, value}
      value when is_binary(value) -> parse_datetime(value)
      _ -> {:error, :invalid_emitted_at}
    end
  end

  defp parse_datetime(value) do
    case DateTime.from_iso8601(value) do
      {:ok, datetime, _offset} -> {:ok, datetime}
      _ -> {:error, :invalid_emitted_at}
    end
  end
end
