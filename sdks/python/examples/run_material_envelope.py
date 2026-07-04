from __future__ import annotations

import json
import os

from kyuubiki_sdk import (
    ControlPlaneClient,
    KyuubikiAuth,
    material_study_envelope_catalog_request,
)


def main() -> None:
    base_url = os.getenv("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")
    token = os.getenv("KYUUBIKI_TOKEN")
    auth = KyuubikiAuth.access_token(token) if token else None

    client = ControlPlaneClient(base_url, auth=auth)
    request = material_study_envelope_catalog_request()
    job = client.submit_workflow_catalog_job(
        request["workflow_id"],
        request["input_artifacts"],
    )

    print(json.dumps(job, indent=2, sort_keys=True))


if __name__ == "__main__":
    main()
