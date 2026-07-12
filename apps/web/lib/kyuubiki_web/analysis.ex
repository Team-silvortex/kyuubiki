defmodule KyuubikiWeb.Analysis do
  @moduledoc """
  Asynchronous orchestration for FEM study jobs.
  """

  alias KyuubikiWeb.AnalysisExports
  alias KyuubikiWeb.AnalysisJobRecords
  alias KyuubikiWeb.AnalysisJobSupport
  alias KyuubikiWeb.AnalysisSolverSubmissions
  alias KyuubikiWeb.Orchestra.Engine, as: OrchestraEngine
  alias KyuubikiWeb.Orchestra.WorkflowJobRunner
  alias KyuubikiWeb.WorkflowGraphResponse

  @material_envelope_catalog_workflow_id "workflow.material-study-envelope-ranking-json"
  @material_envelope_max_rows 128

  defdelegate submit_axial_bar(params), to: AnalysisSolverSubmissions
  defdelegate submit_acoustic_bar_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_bar_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_heat_bar_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_transient_heat_bar_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_electrostatic_bar_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_magnetostatic_bar_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_electrostatic_plane_triangle_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_electrostatic_plane_quad_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_magnetostatic_plane_triangle_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_magnetostatic_plane_quad_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_heat_plane_triangle_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_heat_plane_quad_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_stokes_flow_plane_quad_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_stokes_flow_plane_triangle_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_truss_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_truss_3d(params), to: AnalysisSolverSubmissions
  defdelegate submit_beam_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_beam_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_torsion_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_spring_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_transient_spring_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_harmonic_spring_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_nonlinear_spring_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_contact_gap_1d(params), to: AnalysisSolverSubmissions
  defdelegate submit_spring_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_spring_3d(params), to: AnalysisSolverSubmissions
  defdelegate submit_truss_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_truss_3d(params), to: AnalysisSolverSubmissions
  defdelegate submit_plane_triangle_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_plane_triangle_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_plane_quad_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_plane_quad_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_frame_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_modal_frame_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_frame_3d(params), to: AnalysisSolverSubmissions
  defdelegate submit_solid_tetra_3d(params), to: AnalysisSolverSubmissions
  defdelegate submit_modal_frame_3d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_frame_2d(params), to: AnalysisSolverSubmissions
  defdelegate submit_thermal_frame_3d(params), to: AnalysisSolverSubmissions

  defdelegate list_workflow_catalog(filters \\ %{}), to: OrchestraEngine
  defdelegate list_operator_catalog(filters \\ %{}), to: OrchestraEngine
  defdelegate fetch_workflow_catalog_entry(workflow_id), to: OrchestraEngine
  defdelegate fetch_operator_catalog_entry(operator_id), to: OrchestraEngine
  defdelegate run_workflow_graph(params), to: OrchestraEngine

  @spec submit_catalog_workflow(String.t(), map()) :: {:ok, map()} | {:error, term()}
  def submit_catalog_workflow(workflow_id, params)
      when is_binary(workflow_id) and is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    with {:ok, graph} <- OrchestraEngine.workflow_graph_by_id(workflow_id),
         response_options <-
           WorkflowGraphResponse.resolve_options(graph, Map.get(normalized, "response_options")),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         :ok <- validate_catalog_input_artifacts(workflow_id, input_artifacts),
         {:ok, payload} <-
           submit_workflow_graph(%{
             "graph" => graph,
             "input_artifacts" => input_artifacts,
             "response_options" => response_options,
             "tags" => Map.get(normalized, "tags", []),
             "requested_agent_id" => Map.get(normalized, "requested_agent_id"),
             "requested_capability" => Map.get(normalized, "requested_capability"),
             "control_mode" => Map.get(normalized, "control_mode"),
             "orch_id" => Map.get(normalized, "orch_id"),
             "orch_session_id" => Map.get(normalized, "orch_session_id")
           }) do
      {:ok, payload}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
    end
  end

  defp validate_catalog_input_artifacts(@material_envelope_catalog_workflow_id, input_artifacts) do
    with %{"material_rows" => %{"rows" => rows}} when is_list(rows) <- input_artifacts,
         true <- rows != [],
         true <- length(rows) <= @material_envelope_max_rows,
         true <- Enum.all?(rows, &valid_material_envelope_row?/1) do
      :ok
    else
      false -> {:error, :invalid_material_envelope_catalog_request}
      _ -> {:error, :invalid_material_envelope_catalog_request}
    end
  end

  defp validate_catalog_input_artifacts(_workflow_id, _input_artifacts), do: :ok

  defp valid_material_envelope_row?(%{"case_id" => case_id, "summaries" => summaries})
       when is_binary(case_id) and byte_size(case_id) in 1..128 and is_map(summaries) do
    map_size(summaries) > 0
  end

  defp valid_material_envelope_row?(_row), do: false

  @spec submit_workflow_graph(map()) :: {:ok, map()} | {:error, term()}
  def submit_workflow_graph(params) when is_map(params) do
    normalized = AnalysisJobSupport.stringify_keys(params)

    with %{} = graph <- Map.get(normalized, "graph"),
         response_options <-
           WorkflowGraphResponse.resolve_options(graph, Map.get(normalized, "response_options")),
         %{} = input_artifacts <- Map.get(normalized, "input_artifacts"),
         {:ok, job_context} <- AnalysisJobSupport.derive_job_context(params),
         {:ok, job} <- AnalysisJobSupport.create_job(job_context) do
      orchestration_context = WorkflowJobRunner.orchestration_context_from_params(params)

      :ok =
        WorkflowJobRunner.initialize_runtime(
          job.job_id,
          graph,
          orchestration_context,
          response_options
        )

      WorkflowJobRunner.start(
        job.job_id,
        graph,
        input_artifacts,
        orchestration_context,
        response_options
      )

      {:ok, AnalysisJobSupport.serialize_payload(job)}
    else
      nil -> {:error, :invalid_workflow_graph_request}
      [] -> {:error, :invalid_workflow_graph_request}
      {:error, _reason} = error -> error
      _ -> {:error, :invalid_workflow_graph_request}
    end
  end

  @spec fetch_job(String.t()) :: {:ok, map()} | {:error, term()}
  def fetch_job(job_id) when is_binary(job_id), do: AnalysisJobRecords.fetch_job(job_id)

  @spec list_jobs() :: map()
  def list_jobs, do: AnalysisJobRecords.list_jobs()

  def update_job(job_id, attrs) when is_binary(job_id) and is_map(attrs),
    do: AnalysisJobRecords.update_job(job_id, attrs)

  def cancel_job(job_id) when is_binary(job_id), do: AnalysisJobRecords.cancel_job(job_id)

  def delete_job(job_id) when is_binary(job_id), do: AnalysisJobRecords.delete_job(job_id)

  def list_results, do: AnalysisJobRecords.list_results()

  def fetch_result(job_id) when is_binary(job_id), do: AnalysisJobRecords.fetch_result(job_id)

  def fetch_result_chunk(job_id, kind, params \\ %{})
      when is_binary(job_id) and is_binary(kind) and is_map(params) do
    AnalysisJobRecords.fetch_result_chunk(job_id, kind, params)
  end

  def update_result(job_id, result) when is_binary(job_id) and is_map(result),
    do: AnalysisJobRecords.update_result(job_id, result)

  def delete_result(job_id) when is_binary(job_id), do: AnalysisJobRecords.delete_result(job_id)

  def create_security_event(attrs) when is_map(attrs),
    do: AnalysisExports.create_security_event(attrs)

  def list_security_events(filters \\ %{}) when is_map(filters),
    do: AnalysisExports.list_security_events(filters)

  def export_security_events(filters \\ %{}) when is_map(filters),
    do: AnalysisExports.export_security_events(filters)

  def export_security_events_csv(filters \\ %{}) when is_map(filters),
    do: AnalysisExports.export_security_events_csv(filters)

  def export_database, do: AnalysisExports.export_database()
end
