defmodule KyuubikiWeb.Library.PostgresBackend do
  @moduledoc false

  import Ecto.Query

  alias KyuubikiWeb.Storage
  alias KyuubikiWeb.Storage.{ModelRecord, ModelVersionRecord, ProjectRecord}

  def list_projects do
    query =
      ProjectRecord
      |> order_by([project], desc: project.updated_at)

    projects =
      query
      |> repo_all()
      |> Enum.map(&serialize_project/1)
      |> Enum.map(fn project ->
        Map.put(project, "models", list_models_for_project(project["project_id"]))
      end)

    {:ok, projects}
  end

  def get_project(project_id) do
    case repo_get(ProjectRecord, project_id) do
      nil ->
        :error

      project ->
        {:ok,
         project |> serialize_project() |> Map.put("models", list_models_for_project(project_id))}
    end
  end

  def create_project(attrs) do
    record =
      %ProjectRecord{}
      |> Ecto.Changeset.change(%{
        project_id: attrs["project_id"],
        name: attrs["name"],
        description: attrs["description"]
      })
      |> repo_insert!()

    {:ok, serialize_project(record)}
  end

  def update_project(project_id, attrs) do
    case repo_get(ProjectRecord, project_id) do
      nil ->
        :error

      record ->
        updated =
          record
          |> Ecto.Changeset.change(normalize_attrs(attrs))
          |> repo_update!()

        {:ok, serialize_project(updated)}
    end
  end

  def delete_project(project_id) do
    case repo_get(ProjectRecord, project_id) do
      nil -> :error
      record -> {:ok, serialize_project(repo_delete!(record))}
    end
  end

  def list_models(project_id) do
    {:ok, list_models_for_project(project_id)}
  end

  def get_model(model_id) do
    case repo_get(ModelRecord, model_id) do
      nil ->
        :error

      model ->
        {:ok,
         model |> serialize_model() |> Map.put("versions", list_versions_for_model(model_id))}
    end
  end

  def create_model(attrs) do
    case repo_get(ProjectRecord, attrs["project_id"]) do
      nil ->
        {:error, {:project_not_found, attrs["project_id"]}}

      _project ->
        model =
          %ModelRecord{}
          |> Ecto.Changeset.change(%{
            model_id: attrs["model_id"],
            project_id: attrs["project_id"],
            name: attrs["name"],
            kind: attrs["kind"],
            material: attrs["material"],
            model_schema_version: attrs["model_schema_version"],
            payload: attrs["payload"],
            latest_version_number: 0
          })
          |> repo_insert!()

        {:ok, version} =
          create_version(%{
            "model_id" => model.model_id,
            "name" => "Initial version",
            "payload" => attrs["payload"],
            "kind" => attrs["kind"],
            "material" => attrs["material"],
            "model_schema_version" => attrs["model_schema_version"]
          })

        {:ok,
         repo_get!(ModelRecord, model.model_id)
         |> serialize_model()
         |> Map.put("versions", [version])}
    end
  end

  def update_model(model_id, attrs) do
    case repo_get(ModelRecord, model_id) do
      nil ->
        :error

      record ->
        updated =
          record
          |> Ecto.Changeset.change(normalize_attrs(attrs))
          |> repo_update!()

        {:ok, serialize_model(updated)}
    end
  end

  def delete_model(model_id) do
    case repo_get(ModelRecord, model_id) do
      nil -> :error
      record -> {:ok, serialize_model(repo_delete!(record))}
    end
  end

  def list_versions(model_id) do
    {:ok, list_versions_for_model(model_id)}
  end

  def get_version(version_id) do
    case repo_get(ModelVersionRecord, version_id) do
      nil -> :error
      version -> {:ok, serialize_version(version)}
    end
  end

  def create_version(attrs) do
    case repo_get(ModelRecord, attrs["model_id"]) do
      nil ->
        {:error, {:model_not_found, attrs["model_id"]}}

      model ->
        version_number = (model.latest_version_number || 0) + 1
        timestamp = DateTime.utc_now()

        version =
          %ModelVersionRecord{}
          |> Ecto.Changeset.change(%{
            version_id: attrs["version_id"] || random_id(),
            project_id: model.project_id,
            model_id: model.model_id,
            name: attrs["name"] || model.name,
            version_number: version_number,
            kind: attrs["kind"] || model.kind,
            material: attrs["material"] || model.material,
            model_schema_version: attrs["model_schema_version"] || model.model_schema_version,
            payload: attrs["payload"] || model.payload,
            inserted_at: timestamp,
            updated_at: timestamp
          })
          |> repo_insert!()

        model
        |> Ecto.Changeset.change(%{
          name: attrs["name"] || model.name,
          kind: attrs["kind"] || model.kind,
          material: attrs["material"] || model.material,
          model_schema_version: attrs["model_schema_version"] || model.model_schema_version,
          payload: attrs["payload"] || model.payload,
          latest_version_id: version.version_id,
          latest_version_number: version.version_number,
          updated_at: timestamp
        })
        |> repo_update!()

        {:ok, serialize_version(version)}
    end
  end

  def update_version(version_id, attrs) do
    case repo_get(ModelVersionRecord, version_id) do
      nil ->
        :error

      version ->
        updated =
          version
          |> Ecto.Changeset.change(normalize_attrs(attrs))
          |> repo_update!()

        {:ok, serialize_version(updated)}
    end
  end

  def delete_version(version_id) do
    case repo_get(ModelVersionRecord, version_id) do
      nil -> :error
      version -> {:ok, serialize_version(repo_delete!(version))}
    end
  end

  def reset do
    repo_delete_all(ModelVersionRecord)
    repo_delete_all(ModelRecord)
    repo_delete_all(ProjectRecord)
    :ok
  end

  defp list_models_for_project(project_id) do
    query =
      ModelRecord
      |> where([model], model.project_id == ^project_id)
      |> order_by([model], desc: model.updated_at)

    repo_all(query)
    |> Enum.map(&serialize_model/1)
  end

  defp list_versions_for_model(model_id) do
    query =
      ModelVersionRecord
      |> where([version], version.model_id == ^model_id)
      |> order_by([version], desc: version.version_number)

    repo_all(query)
    |> Enum.map(&serialize_version/1)
  end

  defp serialize_project(record) do
    %{
      "project_id" => record.project_id,
      "name" => record.name,
      "description" => record.description,
      "inserted_at" => DateTime.to_iso8601(record.inserted_at),
      "updated_at" => DateTime.to_iso8601(record.updated_at)
    }
  end

  defp serialize_model(record) do
    %{
      "model_id" => record.model_id,
      "project_id" => record.project_id,
      "name" => record.name,
      "kind" => record.kind,
      "material" => record.material,
      "model_schema_version" => record.model_schema_version,
      "payload" => record.payload,
      "latest_version_id" => record.latest_version_id,
      "latest_version_number" => record.latest_version_number,
      "inserted_at" => DateTime.to_iso8601(record.inserted_at),
      "updated_at" => DateTime.to_iso8601(record.updated_at)
    }
  end

  defp serialize_version(record) do
    %{
      "version_id" => record.version_id,
      "project_id" => record.project_id,
      "model_id" => record.model_id,
      "name" => record.name,
      "version_number" => record.version_number,
      "kind" => record.kind,
      "material" => record.material,
      "model_schema_version" => record.model_schema_version,
      "payload" => record.payload,
      "inserted_at" => DateTime.to_iso8601(record.inserted_at),
      "updated_at" => DateTime.to_iso8601(record.updated_at)
    }
  end

  defp random_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end

  defp normalize_attrs(attrs) do
    Enum.into(attrs, %{}, fn
      {key, value} when is_binary(key) -> {String.to_existing_atom(key), value}
      {key, value} -> {key, value}
    end)
  end

  defp repo do
    Storage.repo_module!()
  end

  defp repo_get(schema, id), do: apply(repo(), :get, [schema, id])
  defp repo_get!(schema, id), do: apply(repo(), :get!, [schema, id])
  defp repo_all(queryable), do: apply(repo(), :all, [queryable])
  defp repo_insert!(changeset), do: apply(repo(), :insert!, [changeset])
  defp repo_update!(changeset), do: apply(repo(), :update!, [changeset])
  defp repo_delete!(struct), do: apply(repo(), :delete!, [struct])
  defp repo_delete_all(queryable), do: apply(repo(), :delete_all, [queryable])
end
