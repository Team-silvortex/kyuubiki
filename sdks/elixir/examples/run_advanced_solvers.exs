alias KyuubikiSdk.Session
alias KyuubikiSdk

base_url = System.get_env("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")
rpc_host = System.get_env("KYUUBIKI_RPC_HOST", "127.0.0.1")
rpc_port = String.to_integer(System.get_env("KYUUBIKI_RPC_PORT", "5001"))

session = KyuubikiSdk.new_session(base_url: base_url, rpc_host: rpc_host, rpc_port: rpc_port)

modal_payload = %{
  "nodes" => [
    %{
      "id" => "n0",
      "x" => 0.0,
      "y" => 0.0,
      "fix_x" => true,
      "fix_y" => true,
      "fix_rz" => true,
      "load_x" => 0.0,
      "load_y" => 0.0,
      "moment_z" => 0.0
    },
    %{
      "id" => "n1",
      "x" => 2.0,
      "y" => 0.0,
      "fix_x" => false,
      "fix_y" => false,
      "fix_rz" => false,
      "load_x" => 0.0,
      "load_y" => 0.0,
      "moment_z" => 0.0
    }
  ],
  "elements" => [
    %{
      "id" => "e0",
      "node_i" => 0,
      "node_j" => 1,
      "area" => 0.01,
      "youngs_modulus" => 210.0e9,
      "moment_of_inertia" => 8.333e-6,
      "section_modulus" => 1.667e-4,
      "density" => 7850.0
    }
  ],
  "mode_count" => 2
}

nonlinear_payload = %{
  "nodes" => [
    %{"id" => "fixed", "x" => 0.0, "fix_x" => true, "load_x" => 0.0},
    %{"id" => "tip", "x" => 1.0, "fix_x" => false, "load_x" => 100.0}
  ],
  "elements" => [
    %{
      "id" => "nl0",
      "node_i" => 0,
      "node_j" => 1,
      "stiffness" => 1000.0,
      "cubic_stiffness" => 50_000.0
    }
  ],
  "load_steps" => 6,
  "max_iterations" => 32,
  "tolerance" => 1.0e-9
}

contact_payload = %{
  "nodes" => [
    %{"id" => "fixed", "x" => 0.0, "fix_x" => true, "load_x" => 0.0},
    %{"id" => "tip", "x" => 1.0, "fix_x" => false, "load_x" => 100.0}
  ],
  "elements" => [
    %{
      "id" => "spring",
      "node_i" => 0,
      "node_j" => 1,
      "stiffness" => 1000.0,
      "cubic_stiffness" => 0.0
    }
  ],
  "contacts" => [
    %{"id" => "stop", "node" => 1, "gap" => 0.05, "normal_stiffness" => 10_000.0}
  ],
  "load_steps" => 6,
  "max_iterations" => 32,
  "tolerance" => 1.0e-9
}

IO.puts("direct modal_frame_2d:")
{:ok, modal} = Session.solve_direct(session, "modal_frame_2d", modal_payload)
IO.puts(Jason.encode_to_iodata!(modal, pretty: true))

IO.puts("\ndirect nonlinear_spring_1d:")
{:ok, nonlinear} = Session.solve_direct(session, "nonlinear_spring_1d", nonlinear_payload)
IO.puts(Jason.encode_to_iodata!(nonlinear, pretty: true))

IO.puts("\ndirect contact_gap_1d:")
{:ok, contact} = Session.solve_direct(session, "contact_gap_1d", contact_payload)
IO.puts(Jason.encode_to_iodata!(contact, pretty: true))

IO.puts("\nmodal workflow graph:")
IO.puts(Jason.encode_to_iodata!(KyuubikiSdk.modal_frame_2d_workflow(), pretty: true))

IO.puts("\nnonlinear workflow graph:")

IO.puts(
  Jason.encode_to_iodata!(KyuubikiSdk.nonlinear_spring_1d_workflow(%{orchestrated: false}),
    pretty: true
  )
)

IO.puts("\ncontact workflow graph:")
IO.puts(Jason.encode_to_iodata!(KyuubikiSdk.contact_gap_1d_workflow(), pretty: true))
