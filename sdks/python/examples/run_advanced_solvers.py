from __future__ import annotations

import json
import os

from kyuubiki_sdk import (
    KyuubikiSession,
    build_contact_gap_1d_workflow,
    build_modal_frame_2d_workflow,
    build_nonlinear_spring_1d_workflow,
)


def modal_frame_payload() -> dict:
    return {
        "nodes": [
            {"id": "n0", "x": 0.0, "y": 0.0, "fix_x": True, "fix_y": True, "fix_rz": True, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
            {"id": "n1", "x": 2.0, "y": 0.0, "fix_x": False, "fix_y": False, "fix_rz": False, "load_x": 0.0, "load_y": 0.0, "moment_z": 0.0},
        ],
        "elements": [
            {
                "id": "e0",
                "node_i": 0,
                "node_j": 1,
                "area": 0.01,
                "youngs_modulus": 210.0e9,
                "moment_of_inertia": 8.333e-6,
                "section_modulus": 1.667e-4,
                "density": 7850.0,
            }
        ],
        "mode_count": 2,
    }


def nonlinear_spring_payload() -> dict:
    return {
        "nodes": [
            {"id": "fixed", "x": 0.0, "fix_x": True, "load_x": 0.0},
            {"id": "tip", "x": 1.0, "fix_x": False, "load_x": 100.0},
        ],
        "elements": [
            {"id": "nl0", "node_i": 0, "node_j": 1, "stiffness": 1000.0, "cubic_stiffness": 50000.0}
        ],
        "load_steps": 6,
        "max_iterations": 32,
        "tolerance": 1.0e-9,
    }


def contact_gap_payload() -> dict:
    return {
        "nodes": [
            {"id": "fixed", "x": 0.0, "fix_x": True, "load_x": 0.0},
            {"id": "tip", "x": 1.0, "fix_x": False, "load_x": 100.0},
        ],
        "elements": [
            {"id": "spring", "node_i": 0, "node_j": 1, "stiffness": 1000.0, "cubic_stiffness": 0.0}
        ],
        "contacts": [
            {"id": "stop", "node": 1, "gap": 0.05, "normal_stiffness": 10000.0}
        ],
        "load_steps": 6,
        "max_iterations": 32,
        "tolerance": 1.0e-9,
    }


def main() -> None:
    base_url = os.getenv("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")
    rpc_host = os.getenv("KYUUBIKI_RPC_HOST", "127.0.0.1")
    rpc_port = int(os.getenv("KYUUBIKI_RPC_PORT", "5001"))
    session = KyuubikiSession.from_endpoints(base_url, rpc_host=rpc_host, rpc_port=rpc_port)

    print("direct modal_frame_2d:")
    print(json.dumps(session.solve_direct("modal_frame_2d", modal_frame_payload()), indent=2))

    print("\ndirect nonlinear_spring_1d:")
    print(json.dumps(session.solve_direct("nonlinear_spring_1d", nonlinear_spring_payload()), indent=2))

    print("\ndirect contact_gap_1d:")
    print(json.dumps(session.solve_direct("contact_gap_1d", contact_gap_payload()), indent=2))

    print("\nmodal workflow graph:")
    print(json.dumps(build_modal_frame_2d_workflow(), indent=2))

    print("\nnonlinear workflow graph:")
    print(json.dumps(build_nonlinear_spring_1d_workflow(orchestrated=False), indent=2))

    print("\ncontact workflow graph:")
    print(json.dumps(build_contact_gap_1d_workflow(), indent=2))


if __name__ == "__main__":
    main()
