# Run only in installed macOS Workbench PWDT, with the Rust example proxy running.
# Set its printed URL and the exact disposable project ID. All writes create new
# acceptance models; existing models are not saved. The API override is restored
# in finally. Keep this WebView open. Stop the proxy after inspecting its receipts.
import builtins
import json
import time
import uuid
from js import window

EXPECTED_PROJECT_ID = ""
PROXY_URL = ""
API_OVERRIDE = "kyuubiki-workbench-api-base-url"
PREFIX = "Acceptance response fault "


def tetrahedron(height):
    points = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, height)]
    pairs = [(0, 1), (1, 2), (2, 0), (0, 3), (1, 3), (2, 3)]
    return {
        "nodes": [
            {"id": f"n{i}", "x": x, "y": y, "z": z,
             "fix_x": i < 3, "fix_y": i < 3, "fix_z": i < 3,
             "load_x": 0, "load_y": 0, "load_z": -1000 if i == 3 else 0}
            for i, (x, y, z) in enumerate(points)
        ],
        "elements": [
            {"id": f"e{i}", "node_i": a, "node_j": b,
             "area": 0.01, "youngs_modulus": 210e9}
            for i, (a, b) in enumerate(pairs)
        ],
    }


async def check_lost_response(save_as):
    before = ky.state()
    options = dict(name=PREFIX + ("fork" if save_as else "version"), material="210", save_as=save_as)
    started = time.perf_counter()
    try:
        await ky.save_model(**options)
    except Exception as error:
        elapsed = time.perf_counter() - started
        if not save_as:
            assert "request timed out" in str(error), str(error)
            assert 12 <= elapsed < 22, f"Body timeout did not release promptly: {elapsed}"
        else:
            assert "Control plane is offline" not in str(error), str(error)
        ky.log("MACOS_COMMITTED_RESPONSE_FAILED", json.dumps({
            "saveAs": save_as, "seconds": round(elapsed, 3), "error": str(error)[:180],
        }))
    else:
        raise AssertionError("The proxy must hide the first committed response")
    for key in ("selectedProjectId", "selectedModelId", "selectedVersionId", "loadedModelName", "activeMaterial"):
        assert ky.state()[key] == before[key], f"Failed write changed local selection: {key}"
    saved = await ky.save_model(**options)
    assert saved["ok"] and not saved.get("contextChanged"), saved
    current = ky.state()
    return {"model": current["selectedModelId"], "version": current["selectedVersionId"]}


async def accept_response_faults():
    assert EXPECTED_PROJECT_ID and ky.state()["selectedProjectId"] == EXPECTED_PROJECT_ID
    assert window.location.origin == "http://127.0.0.1:3000"
    assert "apiBaseUrl" not in window.location.search and "kyuubikiApiBaseUrl" not in window.location.search
    assert PROXY_URL.startswith("http://127.0.0.1:") and PROXY_URL.rsplit(":", 1)[1].isdigit()
    previous = window.localStorage.getItem(API_OVERRIDE)
    receipt = {"project": EXPECTED_PROJECT_ID, "phase": "running"}
    builtins._kyuubiki_response_fault_receipt = receipt
    try:
        window.localStorage.setItem(API_OVERRIDE, PROXY_URL)
        await ky.refresh_all()
        await ky.invoke("state/replaceTruss3dModel", tetrahedron(1.25))
        baseline = await ky.save_model(name=PREFIX + "baseline", material="210", save_as=True,
                                       request_id="macos-body-" + uuid.uuid4().hex)
        assert baseline["ok"] and not baseline.get("contextChanged"), baseline
        receipt["baseline"] = {"model": ky.state()["selectedModelId"], "version": ky.state()["selectedVersionId"]}
        await ky.invoke("state/applyModelBatch", {
            "query": {"kind": "indices", "indices": [3]},
            "operation": {"kind": "translate", "offset": {"x": 0, "y": 0, "z": 0.25}},
        })
        receipt["checkpoint"] = await check_lost_response(False)
        receipt["fork"] = await check_lost_response(True)
        assert receipt["checkpoint"]["model"] == receipt["baseline"]["model"]
        assert receipt["fork"]["model"] != receipt["baseline"]["model"]
        receipt["phase"] = "recovered"
    finally:
        if previous is None:
            window.localStorage.removeItem(API_OVERRIDE)
        else:
            window.localStorage.setItem(API_OVERRIDE, previous)
        ky.log("MACOS_API_OVERRIDE_RESTORED")
    # The fault proxy rejects job submissions. Solve through the restored real
    # backend; if it fails, run only the study again, never this creation script.
    completed = await ky.run_current_study()
    assert completed["jobStatus"] == "completed" and completed["hasResult"], completed
    receipt["phase"] = "complete"
    ky.log("MACOS_RESPONSE_FAULT_PASS", json.dumps(receipt))


await accept_response_faults()
