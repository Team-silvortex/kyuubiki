defmodule KyuubikiWeb.Jobs.Job do
  @moduledoc """
  In-memory representation of the durable job contract from `schemas/job.schema.json`.
  """

  @statuses ~w(
    queued
    preprocessing
    partitioning
    solving
    postprocessing
    completed
    failed
    cancelled
  )a
  @active_stage_order %{
    queued: 0,
    preprocessing: 1,
    partitioning: 2,
    solving: 3,
    postprocessing: 4
  }

  @enforce_keys [:job_id, :project_id, :simulation_case_id]
  defstruct [
    :job_id,
    :project_id,
    :model_version_id,
    :simulation_case_id,
    :worker_id,
    :message,
    :queue_timeout_ms,
    :execution_timeout_ms,
    :execution_started_at,
    status: :queued,
    progress: 0.0,
    residual: nil,
    iteration: nil,
    created_at: nil,
    updated_at: nil
  ]

  @type status :: unquote(Enum.reduce(@statuses, &{:|, [], [&1, &2]}))

  @type t :: %__MODULE__{
          job_id: String.t(),
          project_id: String.t(),
          model_version_id: String.t() | nil,
          simulation_case_id: String.t(),
          worker_id: String.t() | nil,
          message: String.t() | nil,
          queue_timeout_ms: pos_integer() | nil,
          execution_timeout_ms: pos_integer() | nil,
          execution_started_at: DateTime.t() | nil,
          status: status(),
          progress: float(),
          residual: float() | nil,
          iteration: non_neg_integer() | nil,
          created_at: DateTime.t(),
          updated_at: DateTime.t()
        }

  @spec statuses() :: [status()]
  def statuses, do: @statuses

  @spec status_detail(t()) :: map()
  def status_detail(%__MODULE__{} = job) do
    active = job.status in [:queued, :preprocessing, :partitioning, :solving, :postprocessing]
    terminal = job.status in [:completed, :failed, :cancelled]
    failure_class = failure_class(job.status, job.message)

    %{
      "lifecycle" => if(active, do: "active", else: "terminal"),
      "active" => active,
      "terminal" => terminal,
      "failure_class" => failure_class,
      "recoverable" => recoverable_failure_class?(failure_class),
      "timing" => timing_detail(job)
    }
  end

  @spec new(map()) :: {:ok, t()} | {:error, term()}
  def new(attrs) when is_map(attrs) do
    now = DateTime.utc_now()

    with {:ok, job_id} <- fetch_required_string(attrs, :job_id),
         {:ok, project_id} <- fetch_required_string(attrs, :project_id),
         {:ok, simulation_case_id} <- fetch_required_string(attrs, :simulation_case_id),
         {:ok, status} <- fetch_status(attrs, :status, :queued),
         {:ok, progress} <- fetch_progress(attrs, :progress, 0.0),
         :ok <- validate_status_progress(status, progress),
         {:ok, created_at} <- fetch_datetime(attrs, :created_at, now),
         {:ok, updated_at} <- fetch_datetime(attrs, :updated_at, now),
         :ok <- validate_datetime_order(created_at, updated_at) do
      {:ok,
       %__MODULE__{
         job_id: job_id,
         project_id: project_id,
         model_version_id: fetch_optional_string(attrs, :model_version_id),
         simulation_case_id: simulation_case_id,
         worker_id: fetch_optional_string(attrs, :worker_id),
         message: fetch_optional_string(attrs, :message),
         queue_timeout_ms: fetch_optional_positive_integer(attrs, :queue_timeout_ms),
         execution_timeout_ms: fetch_optional_positive_integer(attrs, :execution_timeout_ms),
         execution_started_at: fetch_optional_datetime(attrs, :execution_started_at),
         status: status,
         progress: progress,
         residual: fetch_optional_number(attrs, :residual),
         iteration: fetch_optional_integer(attrs, :iteration),
         created_at: created_at,
         updated_at: updated_at
       }}
    end
  end

  @spec apply_progress(t(), KyuubikiWeb.Jobs.ProgressEvent.t()) ::
          {:ok, t()} | {:error, term()}
  def apply_progress(%__MODULE__{} = job, %KyuubikiWeb.Jobs.ProgressEvent{} = event) do
    with :ok <- validate_job_match(job, event) do
      if idempotent_terminal_replay?(job, event) do
        {:ok, job}
      else
        with :ok <- validate_terminal_transition(job, event),
             :ok <- validate_event_order(job, event),
             :ok <- validate_stage_transition(job.status, event.stage) do
          emitted_at = event.emitted_at || DateTime.utc_now()

          {:ok,
           %__MODULE__{
             job
             | status: event.stage,
               progress: event.progress,
               message: event.message || job.message,
               residual: event.residual || job.residual,
               iteration: event.iteration || job.iteration,
               execution_started_at: execution_started_at(job, event.stage, emitted_at),
               updated_at: emitted_at
           }}
        end
      end
    end
  end

  @spec to_persisted_map(t()) :: map()
  def to_persisted_map(%__MODULE__{} = job) do
    %{
      "job_id" => job.job_id,
      "project_id" => job.project_id,
      "model_version_id" => job.model_version_id,
      "simulation_case_id" => job.simulation_case_id,
      "worker_id" => job.worker_id,
      "message" => job.message,
      "queue_timeout_ms" => job.queue_timeout_ms,
      "execution_timeout_ms" => job.execution_timeout_ms,
      "execution_started_at" => format_datetime(job.execution_started_at),
      "status" => Atom.to_string(job.status),
      "progress" => job.progress,
      "residual" => job.residual,
      "iteration" => job.iteration,
      "created_at" => format_datetime(job.created_at),
      "updated_at" => format_datetime(job.updated_at)
    }
  end

  @spec from_persisted_map(map()) :: {:ok, t()} | {:error, term()}
  def from_persisted_map(attrs) when is_map(attrs) do
    with {:ok, created_at} <- parse_datetime(Map.get(attrs, "created_at")),
         {:ok, updated_at} <- parse_datetime(Map.get(attrs, "updated_at")) do
      new(%{
        job_id: Map.get(attrs, "job_id"),
        project_id: Map.get(attrs, "project_id"),
        model_version_id: Map.get(attrs, "model_version_id"),
        simulation_case_id: Map.get(attrs, "simulation_case_id"),
        worker_id: Map.get(attrs, "worker_id"),
        message: Map.get(attrs, "message"),
        queue_timeout_ms: Map.get(attrs, "queue_timeout_ms"),
        execution_timeout_ms: Map.get(attrs, "execution_timeout_ms"),
        execution_started_at: parse_optional_datetime(Map.get(attrs, "execution_started_at")),
        status: Map.get(attrs, "status"),
        progress: Map.get(attrs, "progress"),
        residual: Map.get(attrs, "residual"),
        iteration: Map.get(attrs, "iteration"),
        created_at: created_at,
        updated_at: updated_at
      })
    end
  end

  defp fetch_required_string(attrs, key) do
    case Map.get(attrs, key) do
      value when is_binary(value) and byte_size(value) > 0 -> {:ok, value}
      _ -> {:error, {:invalid_or_missing, key}}
    end
  end

  defp fetch_optional_string(attrs, key) do
    case Map.get(attrs, key) do
      value when is_binary(value) and byte_size(value) > 0 -> value
      _ -> nil
    end
  end

  defp fetch_optional_number(attrs, key) do
    case Map.get(attrs, key) do
      value when is_integer(value) -> value * 1.0
      value when is_float(value) -> value
      _ -> nil
    end
  end

  defp fetch_optional_integer(attrs, key) do
    case Map.get(attrs, key) do
      value when is_integer(value) and value >= 0 -> value
      _ -> nil
    end
  end

  defp fetch_optional_positive_integer(attrs, key) do
    case Map.get(attrs, key) do
      value when is_integer(value) and value > 0 -> value
      _ -> nil
    end
  end

  defp fetch_optional_datetime(attrs, key) do
    case Map.get(attrs, key) do
      %DateTime{} = value -> value
      _ -> nil
    end
  end

  defp fetch_datetime(attrs, key, default) do
    case Map.get(attrs, key, default) do
      %DateTime{} = value -> {:ok, value}
      _ -> {:error, {:invalid_datetime, key}}
    end
  end

  defp validate_datetime_order(created_at, updated_at) do
    if DateTime.compare(updated_at, created_at) == :lt,
      do: {:error, :updated_at_precedes_created_at},
      else: :ok
  end

  defp execution_started_at(job, :queued, _emitted_at), do: job.execution_started_at

  defp execution_started_at(%{execution_started_at: nil}, _stage, emitted_at), do: emitted_at
  defp execution_started_at(job, _stage, _emitted_at), do: job.execution_started_at

  defp validate_job_match(%{job_id: job_id}, %{job_id: job_id}), do: :ok

  defp validate_job_match(job, event),
    do: {:error, {:job_id_mismatch, job.job_id, event.job_id}}

  defp idempotent_terminal_replay?(%{status: status, progress: progress}, event)
       when status in [:completed, :failed, :cancelled],
       do: event.stage == status and event.progress == progress

  defp idempotent_terminal_replay?(_job, _event), do: false

  defp validate_terminal_transition(%{status: status, progress: progress}, event)
       when status in [:completed, :failed, :cancelled] do
    if event.stage == status and event.progress == progress,
      do: :ok,
      else: {:error, {:terminal_job_mutation, status, event.stage}}
  end

  defp validate_terminal_transition(_job, _event), do: :ok

  defp validate_event_order(job, event) do
    cond do
      event.progress < job.progress ->
        {:error, {:progress_regression, job.progress, event.progress}}

      DateTime.compare(event.emitted_at, job.updated_at) == :lt ->
        {:error, {:stale_progress_event, job.updated_at, event.emitted_at}}

      true ->
        :ok
    end
  end

  defp validate_stage_transition(current, next)
       when is_map_key(@active_stage_order, current) and is_map_key(@active_stage_order, next) do
    if Map.fetch!(@active_stage_order, next) < Map.fetch!(@active_stage_order, current),
      do: {:error, {:stage_regression, current, next}},
      else: :ok
  end

  defp validate_stage_transition(_current, _next), do: :ok

  defp timing_detail(job) do
    queued? = job.status == :queued
    effective_timeout_ms = if queued?, do: job.queue_timeout_ms, else: job.execution_timeout_ms
    deadline_origin = if queued?, do: job.created_at, else: job.execution_started_at

    %{
      "phase" => if(queued?, do: "queue", else: "execution"),
      "queue_wait_ms" => elapsed_ms(job.created_at, job.execution_started_at || job.updated_at),
      "execution_elapsed_ms" => elapsed_ms(job.execution_started_at, job.updated_at),
      "total_elapsed_ms" => elapsed_ms(job.created_at, job.updated_at),
      "queue_timeout_ms" => job.queue_timeout_ms,
      "execution_timeout_ms" => job.execution_timeout_ms,
      "effective_timeout_ms" => effective_timeout_ms,
      "job_submission_deadline" => deadline(job.created_at, job.queue_timeout_ms),
      "execution_started_at" => format_datetime(job.execution_started_at),
      "effective_deadline" => deadline(deadline_origin, effective_timeout_ms)
    }
  end

  defp deadline(%DateTime{} = origin, timeout_ms)
       when is_integer(timeout_ms) and timeout_ms > 0 do
    origin |> DateTime.add(timeout_ms, :millisecond) |> DateTime.to_iso8601()
  end

  defp deadline(_origin, _timeout_ms), do: nil

  defp elapsed_ms(%DateTime{} = started_at, %DateTime{} = ended_at) do
    max(DateTime.diff(ended_at, started_at, :millisecond), 0)
  end

  defp elapsed_ms(_started_at, _ended_at), do: nil

  defp fetch_status(attrs, key, default) do
    status =
      attrs
      |> Map.get(key, default)
      |> normalize_status()

    if status in @statuses do
      {:ok, status}
    else
      {:error, {:invalid_status, status}}
    end
  end

  defp normalize_status(value) when value in @statuses, do: value

  defp normalize_status(value) when is_binary(value) do
    Enum.find(@statuses, fn status -> Atom.to_string(status) == value end) || value
  end

  defp normalize_status(value), do: value

  defp failure_class(:failed, message) when is_binary(message) do
    cond do
      String.contains?(message, "watchdog marked job stalled") -> "watchdog_stalled"
      String.contains?(message, "watchdog timed out job") -> "watchdog_timeout"
      String.contains?(message, "execution timed out") -> "execution_timeout"
      true -> "runtime_failure"
    end
  end

  defp failure_class(:cancelled, message) when is_binary(message) do
    if String.contains?(message, "cancelled by operator"),
      do: "operator_cancelled",
      else: "cancelled"
  end

  defp failure_class(:cancelled, _message), do: "cancelled"
  defp failure_class(_status, _message), do: nil

  defp recoverable_failure_class?(failure_class) do
    failure_class in [
      "watchdog_stalled",
      "watchdog_timeout",
      "execution_timeout",
      "operator_cancelled"
    ]
  end

  defp fetch_progress(attrs, key, default) do
    case Map.get(attrs, key, default) do
      value when is_integer(value) and value >= 0 and value <= 1 -> {:ok, value * 1.0}
      value when is_float(value) and value >= 0.0 and value <= 1.0 -> {:ok, value}
      _ -> {:error, {:invalid_progress, key}}
    end
  rescue
    ArgumentError -> {:error, {:invalid_progress, key}}
  end

  defp validate_status_progress(:completed, 1.0), do: :ok

  defp validate_status_progress(:completed, _progress),
    do: {:error, :completed_progress_must_equal_one}

  defp validate_status_progress(_status, _progress), do: :ok

  defp format_datetime(%DateTime{} = value), do: DateTime.to_iso8601(value)
  defp format_datetime(_value), do: nil

  defp parse_datetime(nil), do: {:ok, DateTime.utc_now()}

  defp parse_datetime(value) when is_binary(value) do
    case DateTime.from_iso8601(value) do
      {:ok, datetime, _offset} -> {:ok, datetime}
      _ -> {:error, :invalid_datetime}
    end
  end

  defp parse_optional_datetime(nil), do: nil

  defp parse_optional_datetime(value) when is_binary(value) do
    case DateTime.from_iso8601(value) do
      {:ok, datetime, _offset} -> datetime
      _ -> nil
    end
  end

  defp parse_optional_datetime(_value), do: nil
end
