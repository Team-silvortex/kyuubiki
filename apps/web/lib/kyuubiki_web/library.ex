defmodule KyuubikiWeb.Library do
  @moduledoc """
  CRUD facade for persisted projects, models, and checkpointed model versions.
  """

  @default_model_schema_version "kyuubiki.model/v1"

  def list_projects do
    backend().list_projects()
  end

  def get_project(project_id) when is_binary(project_id) do
    backend().get_project(project_id)
  end

  def create_project(attrs) when is_map(attrs) do
    attrs
    |> normalize_project_attrs()
    |> case do
      {:ok, normalized} -> backend().create_project(normalized)
      {:error, _reason} = error -> error
    end
  end

  def update_project(project_id, attrs) when is_binary(project_id) and is_map(attrs) do
    {:ok, normalized} = normalize_project_update_attrs(attrs)
    backend().update_project(project_id, normalized)
  end

  def delete_project(project_id) when is_binary(project_id) do
    backend().delete_project(project_id)
  end

  def list_models(project_id) when is_binary(project_id) do
    backend().list_models(project_id)
  end

  def get_model(model_id) when is_binary(model_id) do
    backend().get_model(model_id)
  end

  def create_model(project_id, attrs) when is_binary(project_id) and is_map(attrs) do
    attrs
    |> Map.put("project_id", project_id)
    |> normalize_model_attrs()
    |> case do
      {:ok, normalized} -> backend().create_model(normalized)
      {:error, _reason} = error -> error
    end
  end

  def update_model(model_id, attrs) when is_binary(model_id) and is_map(attrs) do
    {:ok, normalized} = normalize_model_update_attrs(attrs)
    backend().update_model(model_id, normalized)
  end

  def delete_model(model_id) when is_binary(model_id) do
    backend().delete_model(model_id)
  end

  def list_versions(model_id) when is_binary(model_id) do
    backend().list_versions(model_id)
  end

  def get_version(version_id) when is_binary(version_id) do
    backend().get_version(version_id)
  end

  def create_version(model_id, attrs) when is_binary(model_id) and is_map(attrs) do
    attrs
    |> Map.put("model_id", model_id)
    |> normalize_version_attrs()
    |> case do
      {:ok, normalized} -> backend().create_version(normalized)
      {:error, _reason} = error -> error
    end
  end

  def update_version(version_id, attrs) when is_binary(version_id) and is_map(attrs) do
    {:ok, normalized} = normalize_version_update_attrs(attrs)
    backend().update_version(version_id, normalized)
  end

  def delete_version(version_id) when is_binary(version_id) do
    backend().delete_version(version_id)
  end

  def reset do
    backend().reset()
  end

  defp backend do
    if KyuubikiWeb.Storage.postgres?() do
      KyuubikiWeb.Library.PostgresBackend
    else
      KyuubikiWeb.Library.MemoryBackend
    end
  end

  defp normalize_project_attrs(attrs) do
    with {:ok, name} <- fetch_required_string(attrs, ["name", :name]) do
      {:ok,
       %{
         "project_id" => random_id(),
         "name" => name,
         "description" => fetch_optional_string(attrs, ["description", :description])
       }}
    end
  end

  defp normalize_project_update_attrs(attrs) do
    {:ok,
     %{}
     |> put_optional_string("name", attrs, ["name", :name])
     |> put_optional_string("description", attrs, ["description", :description])}
  end

  defp normalize_model_attrs(attrs) do
    with {:ok, project_id} <- fetch_required_string(attrs, ["project_id", :project_id]),
         {:ok, name} <- fetch_required_string(attrs, ["name", :name]),
         {:ok, kind} <- fetch_required_string(attrs, ["kind", :kind]),
         {:ok, payload} <- fetch_required_map(attrs, ["payload", :payload]) do
      {:ok,
       %{
         "model_id" => random_id(),
         "project_id" => project_id,
         "name" => name,
         "kind" => kind,
         "material" => fetch_optional_string(attrs, ["material", :material]),
         "model_schema_version" =>
           fetch_optional_string(attrs, ["model_schema_version", :model_schema_version]) ||
             @default_model_schema_version,
         "payload" => payload
       }}
    end
  end

  defp normalize_model_update_attrs(attrs) do
    {:ok,
     %{}
     |> put_optional_string("name", attrs, ["name", :name])
     |> put_optional_string("kind", attrs, ["kind", :kind])
     |> put_optional_string("material", attrs, ["material", :material])
     |> put_optional_string("model_schema_version", attrs, [
       "model_schema_version",
       :model_schema_version
     ])
     |> put_optional_map("payload", attrs, ["payload", :payload])}
  end

  defp normalize_version_attrs(attrs) do
    with {:ok, model_id} <- fetch_required_string(attrs, ["model_id", :model_id]),
         {:ok, payload} <- fetch_required_map(attrs, ["payload", :payload]) do
      {:ok,
       %{
         "version_id" => random_id(),
         "model_id" => model_id,
         "name" => fetch_optional_string(attrs, ["name", :name]),
         "kind" => fetch_optional_string(attrs, ["kind", :kind]),
         "material" => fetch_optional_string(attrs, ["material", :material]),
         "model_schema_version" =>
           fetch_optional_string(attrs, ["model_schema_version", :model_schema_version]) ||
             @default_model_schema_version,
         "payload" => payload
       }}
    end
  end

  defp normalize_version_update_attrs(attrs) do
    {:ok,
     %{}
     |> put_optional_string("name", attrs, ["name", :name])
     |> put_optional_string("kind", attrs, ["kind", :kind])
     |> put_optional_string("material", attrs, ["material", :material])
     |> put_optional_string("model_schema_version", attrs, [
       "model_schema_version",
       :model_schema_version
     ])
     |> put_optional_map("payload", attrs, ["payload", :payload])}
  end

  defp put_optional_string(acc, key, attrs, keys) do
    case fetch_optional_string(attrs, keys) do
      nil -> acc
      value -> Map.put(acc, key, value)
    end
  end

  defp put_optional_map(acc, key, attrs, keys) do
    case fetch_optional_map(attrs, keys) do
      nil -> acc
      value -> Map.put(acc, key, value)
    end
  end

  defp fetch_required_string(attrs, keys) do
    case fetch_optional_string(attrs, keys) do
      nil -> {:error, :missing_parameter}
      "" -> {:error, :invalid_parameter}
      value -> {:ok, value}
    end
  end

  defp fetch_optional_string(attrs, keys) do
    case find_value(attrs, keys) do
      nil -> nil
      value when is_binary(value) -> String.trim(value)
      value -> to_string(value)
    end
  end

  defp fetch_required_map(attrs, keys) do
    case fetch_optional_map(attrs, keys) do
      nil -> {:error, :missing_parameter}
      value -> {:ok, value}
    end
  end

  defp fetch_optional_map(attrs, keys) do
    case find_value(attrs, keys) do
      value when is_map(value) -> value
      _ -> nil
    end
  end

  defp find_value(attrs, [key | rest]) do
    case Map.fetch(attrs, key) do
      {:ok, value} -> value
      :error -> find_value(attrs, rest)
    end
  end

  defp find_value(_attrs, []), do: nil

  defp random_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end
end
