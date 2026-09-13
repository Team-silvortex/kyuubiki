# Run in installed Workbench PWDT with the bounded Rust checkpoint fault proxy.
# Fill the disposable project and printed proxy URL; run prepare exactly once.
# Quit and reopen the native App, then run recover with the SAME proxy URL.
# The shell may discard browser preferences on exit. Recover explicitly selects
# the SAME loopback proxy again, never a new authority, then restores the unset
# override in finally. The checkpoint journal itself is native metadata storage.
import json
import time
import uuid
from js import window

PHASE = "prepare"
EXPECTED_PROJECT_ID = ""
PROXY_URL = ""
API_OVERRIDE = "kyuubiki-workbench-api-base-url"


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


async def prepare():
    assert EXPECTED_PROJECT_ID and ky.state()["selectedProjectId"] == EXPECTED_PROJECT_ID
    assert window.localStorage.getItem(API_OVERRIDE) is None
    keep_override = False
    try:
        window.localStorage.setItem(API_OVERRIDE, PROXY_URL)
        await ky.refresh_all()
        assert not (await ky.invoke("model/listPendingSaves"))["pending"]
        await ky.invoke("state/replaceTruss3dModel", tetrahedron(1.25))
        baseline = await ky.save_model(name="Acceptance response fault reopen baseline", material="210",
                                       save_as=True, request_id="macos-reopen-" + uuid.uuid4().hex)
        assert baseline["ok"] and not baseline.get("contextChanged"), baseline
        await ky.invoke("state/applyModelBatch", {
            "query": {"kind": "indices", "indices": [3]},
            "operation": {"kind": "translate", "offset": {"x": 0, "y": 0, "z": 0.25}},
        })
        for save_as in (False, True):
            before = ky.state()
            started = time.perf_counter()
            try:
                await ky.save_model(name="Acceptance response fault " + ("fork" if save_as else "reopen version"),
                                    material="210", save_as=save_as)
            except Exception as error:
                seconds = round(time.perf_counter() - started, 3)
                if not save_as:
                    assert "request timed out" in str(error) and 12 <= seconds < 22
                ky.log("REOPEN_FAULT", json.dumps({"saveAs": save_as, "seconds": seconds, "error": str(error)[:120]}))
            else:
                raise AssertionError("Expected a lost committed response, not success")
            for key in ("selectedModelId", "selectedVersionId", "loadedModelName"):
                assert ky.state()[key] == before[key], key
        pending = (await ky.invoke("model/listPendingSaves"))["pending"]
        assert len(pending) == 2 and {entry["operation"] for entry in pending} == {"model", "version"}
        keep_override = True
        ky.log("REOPEN_READY", json.dumps({"pending": len(pending), "baseline": ky.state()["selectedModelId"]}))
    finally:
        if not keep_override:
            window.localStorage.removeItem(API_OVERRIDE)


async def recover():
    assert window.localStorage.getItem(API_OVERRIDE) in (None, PROXY_URL)
    window.localStorage.setItem(API_OVERRIDE, PROXY_URL)
    try:
        pending = (await ky.invoke("model/listPendingSaves"))["pending"]
        assert len(pending) == 2
        # Open the original version first, then the independent fork for the solve.
        for entry in sorted(pending, key=lambda item: item["operation"] != "version"):
            before = ky.state()["selectedVersionId"]
            checked = await ky.invoke("model/checkPendingSave", {"key": entry["key"]})
            receipt = checked["checkpoint"]
            assert receipt["status"] == "committed"
            assert ky.state()["selectedVersionId"] == before
            opened = await ky.invoke("model/openRecoveredSave", {"key": entry["key"]})
            assert opened["ok"] and ky.state()["selectedVersionId"] == receipt["version_id"]
            await ky.invoke("model/acknowledgeSave", {"key": entry["key"]})
            ky.log("REOPEN_CONFIRMED", json.dumps(receipt))
        assert not (await ky.invoke("model/listPendingSaves"))["pending"]
    finally:
        window.localStorage.removeItem(API_OVERRIDE)
        ky.log("REOPEN_API_OVERRIDE_RESTORED")
    completed = await ky.run_current_study()
    assert completed["jobStatus"] == "completed" and completed["hasResult"], completed
    ky.log("REOPEN_PASS", json.dumps({"version": ky.state()["selectedVersionId"], "job": completed}))


assert window.location.origin == "http://127.0.0.1:3000"
assert PROXY_URL.startswith("http://127.0.0.1:") and PROXY_URL.rsplit(":", 1)[1].isdigit()
assert "apiBaseUrl" not in window.location.search and "kyuubikiApiBaseUrl" not in window.location.search
if PHASE == "prepare":
    await prepare()
elif PHASE == "recover":
    await recover()
else:
    raise ValueError("PHASE must be prepare or recover")
