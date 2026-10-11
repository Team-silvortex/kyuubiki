defmodule KyuubikiWeb.AnalysisExports do
  @moduledoc """
  Security event and project data export helpers for analysis APIs.
  """

  alias KyuubikiWeb.AnalysisJobRecords
  alias KyuubikiWeb.Library
  alias KyuubikiWeb.SecurityEvents.Store, as: SecurityEventStore
  alias KyuubikiWeb.Storage.ReadSnapshot
  alias KyuubikiWeb.Storage.FailureBoundary

  @spec create_security_event(map()) :: {:ok, map()} | {:error, term()}
  def create_security_event(attrs) when is_map(attrs) do
    case SecurityEventStore.create(attrs) do
      {:ok, event} -> {:ok, %{"event" => event}}
      {:error, _reason} = error -> error
    end
  end

  @spec list_security_events(map()) :: map() | {:error, term()}
  def list_security_events(filters \\ %{}) when is_map(filters) do
    FailureBoundary.run(fn -> %{"events" => SecurityEventStore.list(filters)} end)
  end

  @spec export_security_events(map()) :: map() | {:error, term()}
  def export_security_events(filters \\ %{}) when is_map(filters),
    do: FailureBoundary.run(fn -> read_security_events(filters) end)

  defp read_security_events(filters) do
    events = SecurityEventStore.list(filters)

    summary =
      Enum.reduce(
        events,
        %{"total" => length(events), "by_source" => %{}, "by_risk" => %{}, "by_status" => %{}},
        fn event, acc ->
          acc
          |> put_in(
            ["by_source", event["source"]],
            get_in(acc, ["by_source", event["source"]]) |> Kernel.||(0) |> Kernel.+(1)
          )
          |> put_in(
            ["by_risk", event["risk"]],
            get_in(acc, ["by_risk", event["risk"]]) |> Kernel.||(0) |> Kernel.+(1)
          )
          |> put_in(
            ["by_status", event["status"]],
            get_in(acc, ["by_status", event["status"]]) |> Kernel.||(0) |> Kernel.+(1)
          )
        end
      )

    %{
      "exported_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601(),
      "schema" => %{
        "name" => "kyuubiki.security-events.export/v1",
        "version" => 1,
        "fields" => [
          %{"name" => "event_id", "type" => "string"},
          %{"name" => "event_type", "type" => "string"},
          %{"name" => "source", "type" => "string"},
          %{"name" => "action", "type" => "string"},
          %{"name" => "risk", "type" => "string"},
          %{"name" => "status", "type" => "string"},
          %{"name" => "note", "type" => "string|null"},
          %{"name" => "context", "type" => "object"},
          %{"name" => "occurred_at", "type" => "datetime"},
          %{"name" => "inserted_at", "type" => "datetime|null"},
          %{"name" => "updated_at", "type" => "datetime|null"}
        ]
      },
      "filters" => normalize_export_filters(filters),
      "summary" => summary,
      "events" => events
    }
  end

  @spec export_security_events_csv(map()) :: String.t() | {:error, term()}
  def export_security_events_csv(filters \\ %{}) when is_map(filters),
    do: FailureBoundary.run(fn -> read_security_events_csv(filters) end)

  defp read_security_events_csv(filters) do
    events = SecurityEventStore.list(filters)

    rows = [
      [
        "event_id",
        "event_type",
        "source",
        "action",
        "risk",
        "status",
        "note",
        "study_kind",
        "project_id",
        "model_version_id",
        "occurred_at",
        "inserted_at",
        "updated_at"
      ]
      | Enum.map(events, fn event ->
          context = event["context"] || %{}

          [
            event["event_id"],
            event["event_type"],
            event["source"],
            event["action"],
            event["risk"],
            event["status"],
            event["note"],
            context["study_kind"],
            context["project_id"],
            context["model_version_id"],
            event["occurred_at"],
            event["inserted_at"],
            event["updated_at"]
          ]
        end)
    ]

    rows
    |> Enum.map_join("\n", fn row -> Enum.map_join(row, ",", &csv_escape/1) end)
    |> Kernel.<>("\n")
  end

  @spec export_database() :: map() | {:error, term()}
  def export_database, do: ReadSnapshot.run(&read_database/0)

  defp read_database do
    with {:ok, projects} <- Library.list_projects(),
         %{"jobs" => jobs} <- AnalysisJobRecords.list_jobs(),
         %{"results" => results} <- AnalysisJobRecords.list_results(),
         %{"events" => events} <- list_security_events() do
      models = Enum.flat_map(projects, &Map.get(&1, "models", []))

      with {:ok, model_versions} <- list_model_versions(models) do
        %{
          "exported_at" => DateTime.utc_now(:second) |> DateTime.to_iso8601(),
          "projects" => projects,
          "models" => models,
          "model_versions" => model_versions,
          "jobs" => jobs,
          "results" => results,
          "security_events" => events
        }
      end
    end
  end

  defp list_model_versions(models) do
    Enum.reduce_while(models, {:ok, []}, fn model, {:ok, collected} ->
      case Library.list_versions(model["model_id"]) do
        {:ok, versions} -> {:cont, {:ok, [versions | collected]}}
        {:error, _} = error -> {:halt, error}
      end
    end)
    |> case do
      {:ok, chunks} -> {:ok, chunks |> Enum.reverse() |> List.flatten()}
      {:error, _} = error -> error
    end
  end

  defp normalize_export_filters(filters) do
    filters
    |> Map.take([
      "source",
      "risk",
      "status",
      "action",
      "study_kind",
      "project_id",
      "model_version_id",
      "occurred_after",
      "occurred_before",
      "limit"
    ])
    |> Enum.reject(fn {_key, value} -> is_nil(value) or value == "" end)
    |> Map.new()
  end

  defp csv_escape(nil), do: ""

  defp csv_escape(value) when is_binary(value) do
    escaped = String.replace(value, "\"", "\"\"")

    if String.contains?(escaped, [",", "\"", "\n", "\r"]) do
      ~s("#{escaped}")
    else
      escaped
    end
  end

  defp csv_escape(value), do: value |> to_string() |> csv_escape()
end
