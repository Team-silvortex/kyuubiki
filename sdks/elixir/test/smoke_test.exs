defmodule KyuubikiSdk.SmokeTest do
  use ExUnit.Case, async: false

  alias KyuubikiSdk.AgentClient
  alias KyuubikiSdk.Session

  setup do
    parent = self()

    {:ok, listener} =
      :gen_tcp.listen(0, [:binary, packet: 0, active: false, reuseaddr: true])

    {:ok, port} = :inet.port(listener)

    acceptor =
      spawn_link(fn ->
        accept_loop(listener, parent)
      end)

    on_exit(fn ->
      Process.exit(acceptor, :kill)
      :gen_tcp.close(listener)
    end)

    {:ok, base_url: "http://127.0.0.1:#{port}"}
  end

  test "agent client runs a study and browses chunks", %{base_url: base_url} do
    session = Session.new(base_url: base_url)

    {:ok, outcome} =
      AgentClient.run_study(session, "truss_2d", %{"nodes" => [], "elements" => []},
        timeout: 5_000
      )

    assert get_in(outcome, [:terminal, "job", "status"]) == "completed"
    assert is_map(outcome.result)

    {:ok, page} =
      AgentClient.browse_result_chunks(session, "job-smoke", "nodes", offset: 0, limit: 2)

    assert page["returned"] == 2
    assert page["total"] == 3
  end

  test "control plane lists and fetches workflow operators", %{base_url: base_url} do
    client = KyuubikiSdk.ControlPlaneClient.new(base_url)

    {:ok, operators} = KyuubikiSdk.ControlPlaneClient.list_workflow_operators(client)
    assert Enum.at(operators["operators"], 0)["id"] == "solver.truss_2d"

    {:ok, workflow} =
      KyuubikiSdk.ControlPlaneClient.fetch_workflow_catalog_workflow(
        client,
        "workflow.test-graph"
      )

    assert workflow["workflow"]["graph"]["id"] == "workflow.test-graph"

    {:ok, filtered} =
      KyuubikiSdk.ControlPlaneClient.list_workflow_operators(client,
        domain: "structural",
        family: "solver"
      )

    assert Enum.at(filtered["operators"], 0)["family"] == "solver"

    {:ok, operator} =
      KyuubikiSdk.ControlPlaneClient.fetch_workflow_operator(client, "solver.truss_2d")

    assert operator["operator"]["kind"] == "solver"
  end

  test "agent client runs workflow jobs", %{base_url: base_url} do
    session = Session.new(base_url: base_url)

    {:ok, catalog_outcome} =
      AgentClient.run_workflow_catalog(
        session,
        "workflow.test-graph",
        %{"mesh" => %{"project_id" => "demo"}},
        timeout: 5_000
      )

    assert get_in(catalog_outcome, [:terminal, "job", "status"]) == "completed"

    assert get_in(catalog_outcome, [:result, "result", "artifacts", "mesh.result", "artifact_id"]) ==
             "artifact.catalog.result"

    assert get_in(catalog_outcome, [
             :validated_outputs,
             "artifacts",
             "output.mesh_result",
             "artifact_id"
           ]) ==
             "artifact.catalog.result"

    assert get_in(catalog_outcome, [:workflow_runtime, "run_id"]) == "run-workflow-catalog"

    assert get_in(catalog_outcome, [
             :workflow_progression,
             "snapshots",
             Access.at(0),
             "current_node"
           ]) == "output"

    graph_definition = %{
      "schema_version" => "kyuubiki.workflow-graph/v1",
      "id" => "workflow.test-inline",
      "name" => "Inline Graph",
      "version" => "1.0.0",
      "entry_nodes" => ["input"],
      "output_nodes" => ["output"],
      "nodes" => [
        %{
          "id" => "input",
          "kind" => "input",
          "inputs" => [],
          "outputs" => [%{"id" => "mesh", "artifact_type" => "mesh.input"}]
        },
        %{
          "id" => "solve",
          "kind" => "solve",
          "operator_id" => "solver.truss_2d",
          "inputs" => [],
          "outputs" => []
        },
        %{
          "id" => "output",
          "kind" => "output",
          "inputs" => [%{"id" => "mesh_result", "artifact_type" => "mesh.result"}],
          "outputs" => []
        }
      ],
      "edges" => []
    }

    {:ok, graph_outcome} =
      AgentClient.run_workflow_graph(
        session,
        graph_definition,
        %{"mesh" => %{"project_id" => "demo"}},
        timeout: 5_000
      )

    assert get_in(graph_outcome, [:terminal, "job", "status"]) == "completed"

    assert get_in(graph_outcome, [:result, "result", "artifacts", "mesh.result", "artifact_id"]) ==
             "artifact.graph.result"

    assert get_in(graph_outcome, [:output_manifest, "graph_id"]) == "workflow.test-inline"

    assert get_in(graph_outcome, [
             :validated_outputs,
             "artifacts",
             "output.mesh_result",
             "artifact_id"
           ]) ==
             "artifact.graph.result"

    assert get_in(graph_outcome, [:workflow_runtime, "current_node"]) == "output"

    assert get_in(graph_outcome, [:workflow_progression, "latest", "run_id"]) ==
             "run-workflow-graph"
  end

  test "session supports expanded solve kinds", %{base_url: base_url} do
    session = Session.new(base_url: base_url)

    {:ok, axial} =
      Session.submit_job(session, "axial_bar_1d", %{"nodes" => [], "elements" => []})

    assert axial["job"]["job_id"] == "job-axial"

    {:ok, thermal_frame} =
      Session.submit_job(session, "thermal_frame_3d", %{"nodes" => [], "elements" => []})

    assert thermal_frame["job"]["job_id"] == "job-thermal-frame-3d"
  end

  test "session supports direct rpc for advanced solve kinds" do
    {:ok, listener} =
      :gen_tcp.listen(0, [:binary, packet: 0, active: false, reuseaddr: true])

    {:ok, port} = :inet.port(listener)

    server =
      spawn_link(fn ->
        for expected <- [
              "solve_modal_frame_2d",
              "solve_nonlinear_spring_1d",
              "solve_contact_gap_1d",
              "solve_acoustic_bar_1d",
              "solve_stokes_flow_plane_quad_2d"
            ] do
          {:ok, socket} = :gen_tcp.accept(listener)
          {:ok, <<size::unsigned-big-32>>} = :gen_tcp.recv(socket, 4)
          {:ok, payload} = :gen_tcp.recv(socket, size)
          request = Jason.decode!(payload)
          assert request["method"] == expected

          response =
            Jason.encode!(%{
              ok: true,
              result: %{
                "solver" => String.replace_prefix(expected, "solve_", ""),
                "input" => request["params"]
              }
            })

          :ok = :gen_tcp.send(socket, <<byte_size(response)::unsigned-big-32, response::binary>>)
          :gen_tcp.close(socket)
        end
      end)

    on_exit(fn ->
      Process.exit(server, :kill)
      :gen_tcp.close(listener)
    end)

    session = Session.new(rpc_host: "127.0.0.1", rpc_port: port)

    {:ok, modal} =
      Session.solve_direct(session, "modal_frame_2d", %{"nodes" => [], "elements" => []})

    {:ok, nonlinear} =
      Session.solve_direct(session, "nonlinear_spring_1d", %{"nodes" => [], "elements" => []})

    {:ok, contact} =
      Session.solve_direct(session, "contact_gap_1d", %{
        "nodes" => [],
        "elements" => [],
        "contacts" => []
      })

    {:ok, acoustic} =
      Session.solve_direct(session, "acoustic_bar_1d", %{"nodes" => [], "elements" => []})

    {:ok, stokes} =
      Session.solve_direct(session, "stokes_flow_quad_2d", %{"nodes" => [], "elements" => []})

    assert modal["solver"] == "modal_frame_2d"
    assert nonlinear["solver"] == "nonlinear_spring_1d"
    assert contact["solver"] == "contact_gap_1d"
    assert acoustic["solver"] == "acoustic_bar_1d"
    assert stokes["solver"] == "stokes_flow_plane_quad_2d"
  end

  defp accept_loop(listener, parent) do
    {:ok, socket} = :gen_tcp.accept(listener)
    handle_socket(socket)
    send(parent, :handled_request)
    accept_loop(listener, parent)
  end

  defp handle_socket(socket) do
    {:ok, request} = :gen_tcp.recv(socket, 0)
    [request_line | _rest] = String.split(request, "\r\n")
    [method, path, _version] = String.split(request_line, " ")

    response =
      case {method, path} do
        {"POST", "/api/v1/fem/truss-2d/jobs"} ->
          json_response(202, %{"job" => %{"job_id" => "job-smoke", "status" => "queued"}})

        {"POST", "/api/v1/fem/axial-bar/jobs"} ->
          json_response(202, %{"job" => %{"job_id" => "job-axial", "status" => "queued"}})

        {"POST", "/api/v1/fem/thermal-frame-3d/jobs"} ->
          json_response(202, %{
            "job" => %{"job_id" => "job-thermal-frame-3d", "status" => "queued"}
          })

        {"POST", "/api/v1/workflows/catalog/workflow.test-graph/jobs"} ->
          json_response(202, %{
            "job" => %{"job_id" => "workflow-catalog-job", "status" => "queued"}
          })

        {"GET", "/api/v1/workflows/catalog/workflow.test-graph"} ->
          json_response(200, %{
            "workflow" => %{
              "id" => "workflow.test-graph",
              "graph" => %{
                "schema_version" => "kyuubiki.workflow-graph/v1",
                "id" => "workflow.test-graph",
                "name" => "Test Graph",
                "version" => "1.0.0",
                "entry_nodes" => ["input"],
                "output_nodes" => ["output"],
                "nodes" => [
                  %{
                    "id" => "input",
                    "kind" => "input",
                    "inputs" => [],
                    "outputs" => [%{"id" => "mesh", "artifact_type" => "mesh.input"}]
                  },
                  %{
                    "id" => "output",
                    "kind" => "output",
                    "inputs" => [%{"id" => "mesh_result", "artifact_type" => "mesh.result"}],
                    "outputs" => []
                  }
                ],
                "edges" => []
              }
            }
          })

        {"POST", "/api/v1/workflows/graph/jobs"} ->
          json_response(202, %{"job" => %{"job_id" => "workflow-graph-job", "status" => "queued"}})

        {"GET", "/api/v1/operators"} ->
          json_response(200, %{
            "operators" => [
              %{
                "id" => "solver.truss_2d",
                "version" => "1.0.0",
                "domain" => "structural",
                "family" => "solver",
                "kind" => "solver",
                "summary" => "Smoke operator"
              }
            ]
          })

        {"GET", "/api/v1/operators?domain=structural&family=solver"} ->
          json_response(200, %{
            "operators" => [
              %{
                "id" => "solver.truss_2d",
                "version" => "1.0.0",
                "domain" => "structural",
                "family" => "solver",
                "kind" => "solver",
                "summary" => "Smoke operator"
              }
            ]
          })

        {"GET", "/api/v1/operators/solver.truss_2d"} ->
          json_response(200, %{
            "operator" => %{
              "id" => "solver.truss_2d",
              "version" => "1.0.0",
              "domain" => "structural",
              "family" => "solver",
              "kind" => "solver",
              "summary" => "Smoke operator"
            }
          })

        {"GET", "/api/v1/jobs/job-smoke"} ->
          json_response(200, %{
            "job" => %{"job_id" => "job-smoke", "status" => "completed", "progress" => 1.0}
          })

        {"GET", "/api/v1/jobs/workflow-catalog-job"} ->
          json_response(200, %{
            "job" => %{
              "job_id" => "workflow-catalog-job",
              "status" => "completed",
              "progress" => 1.0,
              "current_node" => "output",
              "completed_nodes" => ["input", "output"],
              "progress_events" => [%{"node_id" => "output", "status" => "completed"}]
            }
          })

        {"GET", "/api/v1/jobs/workflow-graph-job"} ->
          json_response(200, %{
            "job" => %{
              "job_id" => "workflow-graph-job",
              "status" => "completed",
              "progress" => 1.0,
              "current_node" => "output",
              "completed_nodes" => ["input", "solve", "output"],
              "progress_events" => [%{"node_id" => "solve", "status" => "completed"}]
            }
          })

        {"GET", "/api/v1/results/job-smoke"} ->
          json_response(200, %{
            "job_id" => "job-smoke",
            "result" => %{
              "nodes" => [
                %{"index" => 0, "id" => "n0"},
                %{"index" => 1, "id" => "n1"},
                %{"index" => 2, "id" => "n2"}
              ],
              "elements" => [%{"index" => 0, "id" => "e0"}],
              "max_displacement" => 1.0e-6,
              "max_stress" => 7.0e4
            }
          })

        {"GET", "/api/v1/results/workflow-catalog-job"} ->
          json_response(200, %{
            "job_id" => "workflow-catalog-job",
            "result" => %{
              "workflow_id" => "workflow.test-graph",
              "run_id" => "run-workflow-catalog",
              "status" => "completed",
              "current_node" => "output",
              "completed_nodes" => ["input", "output"],
              "progress_events" => [%{"node_id" => "output", "status" => "completed"}],
              "artifacts" => %{"mesh.result" => %{"artifact_id" => "artifact.catalog.result"}}
            }
          })

        {"GET", "/api/v1/results/workflow-graph-job"} ->
          json_response(200, %{
            "job_id" => "workflow-graph-job",
            "result" => %{
              "workflow_id" => "workflow.test-inline",
              "run_id" => "run-workflow-graph",
              "status" => "completed",
              "current_node" => "output",
              "completed_nodes" => ["input", "solve", "output"],
              "progress_events" => [%{"node_id" => "solve", "status" => "completed"}],
              "artifacts" => %{"mesh.result" => %{"artifact_id" => "artifact.graph.result"}}
            }
          })

        {"GET", "/api/v1/results/job-smoke/chunks/nodes?offset=0&limit=2"} ->
          json_response(200, %{
            "job_id" => "job-smoke",
            "kind" => "nodes",
            "offset" => 0,
            "limit" => 2,
            "returned" => 2,
            "total" => 3,
            "items" => [%{"index" => 0, "id" => "n0"}, %{"index" => 1, "id" => "n1"}]
          })

        _ ->
          json_response(404, %{"error" => "not_found"})
      end

    :ok = :gen_tcp.send(socket, response)
    :gen_tcp.close(socket)
  end

  defp json_response(status, payload) do
    body = Jason.encode!(payload)

    reason =
      case status do
        200 -> "OK"
        202 -> "Accepted"
        404 -> "Not Found"
        _ -> "OK"
      end

    [
      "HTTP/1.1 ",
      Integer.to_string(status),
      " ",
      reason,
      "\r\nServer: kyuubiki-smoke\r\nDate: Tue, 15 Apr 2026 00:00:00 GMT\r\nContent-Type: application/json\r\nContent-Length: ",
      Integer.to_string(byte_size(body)),
      "\r\nConnection: close\r\n\r\n",
      body
    ]
    |> IO.iodata_to_binary()
  end
end
