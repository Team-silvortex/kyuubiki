from __future__ import annotations

import json
import os

from kyuubiki_sdk import KyuubikiAgentClient, KyuubikiAuth, KyuubikiSession


def minimal_truss_2d_payload() -> dict:
    return {
        "nodes": [
            {
                "id": "n0",
                "x": 0.0,
                "y": 0.0,
                "fix_x": True,
                "fix_y": True,
                "load_x": 0.0,
                "load_y": 0.0,
            },
            {
                "id": "n1",
                "x": 1.0,
                "y": 0.0,
                "fix_x": False,
                "fix_y": True,
                "load_x": 0.0,
                "load_y": 0.0,
            },
            {
                "id": "n2",
                "x": 0.5,
                "y": 0.75,
                "fix_x": False,
                "fix_y": False,
                "load_x": 0.0,
                "load_y": -1000.0,
            },
        ],
        "elements": [
            {
                "id": "e0",
                "node_i": 0,
                "node_j": 1,
                "area": 0.01,
                "youngs_modulus": 7.0e10,
            },
            {
                "id": "e1",
                "node_i": 1,
                "node_j": 2,
                "area": 0.01,
                "youngs_modulus": 7.0e10,
            },
            {
                "id": "e2",
                "node_i": 2,
                "node_j": 0,
                "area": 0.01,
                "youngs_modulus": 7.0e10,
            },
        ],
    }


def main() -> None:
    base_url = os.getenv("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")
    token = os.getenv("KYUUBIKI_TOKEN")
    auth = KyuubikiAuth.access_token(token) if token else None

    session = KyuubikiSession.from_control_plane(base_url, auth=auth)
    agent = KyuubikiAgentClient(session)

    outcome = agent.run_study("truss_2d", minimal_truss_2d_payload(), timeout_s=60.0)
    print("terminal:")
    print(json.dumps(outcome["terminal"], indent=2))

    job_id = outcome["terminal"]["job"]["job_id"]
    print("\nfirst nodes page:")
    print(json.dumps(agent.browse_result_chunks(job_id, "nodes", offset=0, limit=2), indent=2))

    print("\niterating element pages:")
    for page in agent.iter_result_chunks(job_id, "elements", page_size=2):
        print(json.dumps(page, indent=2))


if __name__ == "__main__":
    main()
