defmodule KyuubikiWeb.Library.MemoryBackend do
  @moduledoc false

  use Agent

  def start_link(_opts) do
    Agent.start_link(fn -> %{projects: %{}, models: %{}, versions: %{}} end, name: __MODULE__)
  end

  def list_projects do
    Agent.get(__MODULE__, fn state ->
      projects =
        state.projects
        |> Map.values()
        |> Enum.sort_by(& &1["updated_at"], {:desc, DateTime})
        |> Enum.map(fn project ->
          Map.put(project, "models", list_models_for_state(state, project["project_id"]))
        end)

      {:ok, projects}
    end)
  end

  def get_project(project_id) do
    Agent.get(__MODULE__, fn state ->
      case Map.get(state.projects, project_id) do
        nil -> :error
        project -> {:ok, Map.put(project, "models", list_models_for_state(state, project_id))}
      end
    end)
  end

  def create_project(attrs) do
    timestamp = DateTime.utc_now(:second)
    project = attrs |> Map.put("inserted_at", timestamp) |> Map.put("updated_at", timestamp)

    Agent.update(__MODULE__, fn state ->
      put_in(state, [:projects, project["project_id"]], project)
    end)

    {:ok, project}
  end

  def update_project(project_id, attrs) do
    Agent.get_and_update(__MODULE__, fn state ->
      case Map.get(state.projects, project_id) do
        nil ->
          {:error, state}

        project ->
          updated =
            project |> Map.merge(attrs) |> Map.put("updated_at", DateTime.utc_now(:second))

          {{:ok, updated}, put_in(state, [:projects, project_id], updated)}
      end
    end)
  end

  def delete_project(project_id) do
    Agent.get_and_update(__MODULE__, fn state ->
      case Map.get(state.projects, project_id) do
        nil ->
          {:error, state}

        project ->
          model_ids =
            state.models
            |> Map.values()
            |> Enum.filter(&(&1["project_id"] == project_id))
            |> Enum.map(& &1["model_id"])

          version_ids =
            state.versions
            |> Map.values()
            |> Enum.filter(&(&1["project_id"] == project_id))
            |> Enum.map(& &1["version_id"])

          next_state =
            state
            |> update_in([:projects], &Map.delete(&1, project_id))
            |> update_in([:models], fn models ->
              Enum.reduce(model_ids, models, &Map.delete(&2, &1))
            end)
            |> update_in([:versions], fn versions ->
              Enum.reduce(version_ids, versions, &Map.delete(&2, &1))
            end)

          {{:ok, project}, next_state}
      end
    end)
  end

  def list_models(project_id) do
    Agent.get(__MODULE__, fn state -> {:ok, list_models_for_state(state, project_id)} end)
  end

  def get_model(model_id) do
    Agent.get(__MODULE__, fn state ->
      case Map.get(state.models, model_id) do
        nil -> :error
        model -> {:ok, Map.put(model, "versions", list_versions_for_state(state, model_id))}
      end
    end)
  end

  def create_model(attrs) do
    Agent.get_and_update(__MODULE__, fn state ->
      if Map.has_key?(state.projects, attrs["project_id"]) do
        timestamp = DateTime.utc_now(:second)

        model =
          attrs
          |> Map.put("inserted_at", timestamp)
          |> Map.put("updated_at", timestamp)

        version =
          build_version(attrs["project_id"], model, %{
            "version_id" => random_id(),
            "name" => "Initial version"
          })

        updated_model =
          model
          |> Map.put("latest_version_id", version["version_id"])
          |> Map.put("latest_version_number", version["version_number"])

        next_state =
          state
          |> put_in([:models, updated_model["model_id"]], updated_model)
          |> put_in([:versions, version["version_id"]], version)

        {{:ok, updated_model}, next_state}
      else
        {{:error, {:project_not_found, attrs["project_id"]}}, state}
      end
    end)
  end

  def update_model(model_id, attrs) do
    Agent.get_and_update(__MODULE__, fn state ->
      case Map.get(state.models, model_id) do
        nil ->
          {:error, state}

        model ->
          updated = model |> Map.merge(attrs) |> Map.put("updated_at", DateTime.utc_now(:second))
          {{:ok, updated}, put_in(state, [:models, model_id], updated)}
      end
    end)
  end

  def delete_model(model_id) do
    Agent.get_and_update(__MODULE__, fn state ->
      case Map.get(state.models, model_id) do
        nil ->
          {:error, state}

        model ->
          version_ids =
            state.versions
            |> Map.values()
            |> Enum.filter(&(&1["model_id"] == model_id))
            |> Enum.map(& &1["version_id"])

          next_state =
            state
            |> update_in([:models], &Map.delete(&1, model_id))
            |> update_in([:versions], fn versions ->
              Enum.reduce(version_ids, versions, &Map.delete(&2, &1))
            end)

          {{:ok, model}, next_state}
      end
    end)
  end

  def list_versions(model_id) do
    Agent.get(__MODULE__, fn state -> {:ok, list_versions_for_state(state, model_id)} end)
  end

  def get_version(version_id) do
    Agent.get(__MODULE__, fn state ->
      case Map.get(state.versions, version_id) do
        nil -> :error
        version -> {:ok, version}
      end
    end)
  end

  def create_version(attrs) do
    Agent.get_and_update(__MODULE__, fn state ->
      case Map.get(state.models, attrs["model_id"]) do
        nil ->
          {{:error, {:model_not_found, attrs["model_id"]}}, state}

        model ->
          version = build_version(model["project_id"], model, attrs)

          updated_model =
            model
            |> Map.put("name", attrs["name"] || model["name"])
            |> Map.put("kind", attrs["kind"] || model["kind"])
            |> Map.put("material", attrs["material"] || model["material"])
            |> Map.put(
              "model_schema_version",
              attrs["model_schema_version"] || model["model_schema_version"]
            )
            |> Map.put("payload", attrs["payload"])
            |> Map.put("latest_version_id", version["version_id"])
            |> Map.put("latest_version_number", version["version_number"])
            |> Map.put("updated_at", DateTime.utc_now(:second))

          next_state =
            state
            |> put_in([:models, updated_model["model_id"]], updated_model)
            |> put_in([:versions, version["version_id"]], version)

          {{:ok, version}, next_state}
      end
    end)
  end

  def update_version(version_id, attrs) do
    Agent.get_and_update(__MODULE__, fn state ->
      case Map.get(state.versions, version_id) do
        nil ->
          {:error, state}

        version ->
          updated =
            version |> Map.merge(attrs) |> Map.put("updated_at", DateTime.utc_now(:second))

          {{:ok, updated}, put_in(state, [:versions, version_id], updated)}
      end
    end)
  end

  def delete_version(version_id) do
    Agent.get_and_update(__MODULE__, fn state ->
      case Map.get(state.versions, version_id) do
        nil ->
          {:error, state}

        version ->
          {{:ok, version}, update_in(state, [:versions], &Map.delete(&1, version_id))}
      end
    end)
  end

  def reset do
    Agent.update(__MODULE__, fn _ -> %{projects: %{}, models: %{}, versions: %{}} end)
    :ok
  end

  defp list_models_for_state(state, project_id) do
    state.models
    |> Map.values()
    |> Enum.filter(&(&1["project_id"] == project_id))
    |> Enum.sort_by(& &1["updated_at"], {:desc, DateTime})
  end

  defp list_versions_for_state(state, model_id) do
    state.versions
    |> Map.values()
    |> Enum.filter(&(&1["model_id"] == model_id))
    |> Enum.sort_by(& &1["version_number"], :desc)
  end

  defp build_version(project_id, model, attrs) do
    timestamp = DateTime.utc_now(:second)
    next_number = model["latest_version_number"] || 0

    %{
      "version_id" => attrs["version_id"] || random_id(),
      "project_id" => project_id,
      "model_id" => model["model_id"],
      "name" => attrs["name"] || model["name"],
      "version_number" => next_number + 1,
      "kind" => attrs["kind"] || model["kind"],
      "material" => attrs["material"] || model["material"],
      "model_schema_version" => attrs["model_schema_version"] || model["model_schema_version"],
      "payload" => attrs["payload"] || model["payload"],
      "inserted_at" => timestamp,
      "updated_at" => timestamp
    }
  end

  defp random_id do
    :crypto.strong_rand_bytes(8) |> Base.encode16(case: :lower)
  end
end
