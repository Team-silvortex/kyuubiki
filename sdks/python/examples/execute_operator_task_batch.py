from __future__ import annotations

import json
import os
import sys
from pathlib import Path

from kyuubiki_sdk import ControlPlaneClient, KyuubikiAuth


def main() -> None:
    if len(sys.argv) != 2:
        raise SystemExit("usage: python examples/execute_operator_task_batch.py batch.json")

    base_url = os.getenv("KYUUBIKI_BASE_URL", "http://127.0.0.1:4000")
    token = os.getenv("KYUUBIKI_TOKEN")
    auth = KyuubikiAuth.access_token(token) if token else None
    batch = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))

    client = ControlPlaneClient(base_url, auth=auth)
    result = client.execute_operator_task_batch(batch)
    print(json.dumps(result, indent=2))


if __name__ == "__main__":
    main()
