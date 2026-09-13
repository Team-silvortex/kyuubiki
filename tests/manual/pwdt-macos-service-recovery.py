# Run in installed macOS Workbench PWDT, not a native Python/headless process.
# Select a disposable acceptance project and set its exact ID below. Run prepare,
# stop the idle Installer-managed services externally, run offline, restart those
# services, then run recover. Keep this WebView open between all three phases.
# No fault proxy, request mock, auto-retry, database replacement or backup is used.
import builtins
import json
import sys
import uuid

PHASE = "prepare"
EXPECTED_PROJECT_ID = ""
SESSION_SLOT = "_kyuubiki_native_service_recovery"
NAME = "Native service recovery"


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


def assert_selection(receipt):
    current = ky.state()
    for key, value in receipt["selection"].items():
        assert current[key] == value, f"Selection changed: {key}"


async def accept_service_recovery():
    assert EXPECTED_PROJECT_ID, "Set the exact disposable project ID first"
    assert ky.state()["selectedProjectId"] == EXPECTED_PROJECT_ID
    receipt = getattr(builtins, SESSION_SLOT, None)
    if PHASE == "prepare":
        assert receipt is None or receipt["phase"] == "complete", "Finish the active acceptance first"
        await ky.invoke("state/replaceTruss3dModel", tetrahedron(1.25))
        created = await ky.save_model(name=NAME, material="210", save_as=True,
                                      request_id="macos-service-" + uuid.uuid4().hex)
        assert created["ok"] and not created.get("contextChanged"), created
        completed = await ky.run_current_study()
        assert completed["jobStatus"] == "completed" and completed["hasResult"], completed
        await ky.invoke("state/applyModelBatch", {
            "query": {"kind": "indices", "indices": [3]},
            "operation": {"kind": "translate", "offset": {"x": 0, "y": 0, "z": 0.25}},
        })
        current = ky.state()
        receipt = {"phase": "prepared", "selection": {key: current[key] for key in (
            "selectedProjectId", "selectedModelId", "selectedVersionId",
            "loadedModelName", "activeMaterial",
        )}}
        setattr(builtins, SESSION_SLOT, receipt)
        ky.log("MACOS_SERVICE_PREPARED", json.dumps(receipt["selection"]))
    elif PHASE == "offline":
        assert receipt and receipt["phase"] == "prepared", "Run prepare first in this WebView"
        assert_selection(receipt)
        await ky.refresh_all()
        assert ky.state()["healthStatus"] is None, "Stop the managed services before the offline phase"
        try:
            await ky.save_model(name=NAME + " h1.50", material="210")
        except Exception as error:
            message = str(error)
            assert "Control plane is offline" in message, message
            ky.log("MACOS_OFFLINE_SAVE_BLOCKED", message)
        else:
            raise AssertionError("Offline save must not succeed")
        assert_selection(receipt)
        receipt["phase"] = "offline"
        ky.log("MACOS_OFFLINE_DRAFT_RETAINED", receipt["selection"]["selectedVersionId"])
    elif PHASE == "recover":
        assert receipt and receipt["phase"] == "offline", "Run offline first"
        assert_selection(receipt)
        await ky.refresh_all()
        assert_selection(receipt)
        saved = await ky.save_model(name=NAME + " h1.50", material="210")
        assert saved["ok"] and not saved.get("contextChanged"), saved
        assert saved["versionId"] != receipt["selection"]["selectedVersionId"]
        # Mark the confirmed write before the follow-up solve. Never resave after
        # a solve failure: inspect the committed version and run the study only.
        receipt["phase"] = "saved"
        receipt["savedVersionId"] = saved["versionId"]
        completed = await ky.run_current_study()
        assert completed["jobStatus"] == "completed" and completed["hasResult"], completed
        receipt["phase"] = "complete"
        ky.log("MACOS_SERVICE_RECOVERY_PASS", json.dumps({
            "python": sys.version.split()[0], "project": EXPECTED_PROJECT_ID,
            "model": receipt["selection"]["selectedModelId"],
            "v1": receipt["selection"]["selectedVersionId"], "v2": saved["versionId"],
            "status": completed["jobStatus"],
        }))
    else:
        raise ValueError("PHASE must be prepare, offline or recover")


await accept_service_recovery()
