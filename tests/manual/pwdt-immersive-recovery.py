# Reopen the saved tall variant after restarting the managed runtime.
# Run this script in native Workbench PWDT. It does not rewrite model geometry.
import json
import sys
from js import window


async def accept_recovery():
    before = ky.state()
    assert before["loadedModelName"] == "Acceptance tetrahedron tall variant"
    assert before["selectedVersionId"]
    completed = await ky.run_current_study()
    assert completed["hasResult"] and completed["jobStatus"] == "completed"
    match = await ky.invoke("selection/query3d", {
        "query": {"kind": "range", "axis": "z", "min": 1.25, "max": 1.25},
    })
    assert match["selectedNodes"] == 1
    assert ky.state()["selectedTruss3dNodeIndices"] == [3]
    try:
        await ky.invoke("selection/query3d", {"query": {"kind": "indices", "indices": [-1]}})
    except Exception as error:
        assert "invalid_indices" in str(error), str(error)
    else:
        raise AssertionError("Invalid node indices must be rejected")
    assert ky.state()["selectedTruss3dNodeIndices"] == [3]
    assert ky.state()["hasResult"], "Selection failure cannot discard solved results"
    # A WKWebView with no fullscreen API uses the reversible window-local path.
    # Browsers that require a user gesture should enter fullscreen using its button.
    try:
        await ky.invoke("viewport/setUiState", {"immersiveViewport": True, "toolTab": "batch"})
        await ky.wait_until(lambda s: s["immersiveViewport"] and
            ky.selector_exists("viewportStage") and
            ky.require_selector("viewportStage").getBoundingClientRect().height > 100,
            timeout=30, interval=0.1)
        panel = ky.require_selector("viewportPanel").getBoundingClientRect()
        stage = ky.require_selector("viewportStage").getBoundingClientRect()
        ky.log("NATIVE_LAYOUT_MEASURE", json.dumps({"window": [window.innerWidth, window.innerHeight],
            "panel": [panel.width, panel.height], "stage": [stage.width, stage.height]}))
        assert ky.state()["immersiveViewport"]
        assert abs(panel.width - window.innerWidth) < 1
        assert abs(panel.height - window.innerHeight) < 1
        assert stage.height > 100 and stage.bottom <= window.innerHeight + 1
        ky.log("NATIVE_RECOVERY_PASS", json.dumps({
            "python": sys.version.split()[0], "modelVersion": before["selectedVersionId"],
            "jobStatus": completed["jobStatus"], "selectedNodes": [3],
            "window": [window.innerWidth, window.innerHeight],
            "panel": [panel.width, panel.height], "stage": [stage.width, stage.height],
            "invalidQueryPreservedResult": True,
        }))
    finally:
        await ky.invoke("viewport/setUiState", {"immersiveViewport": False})
        await ky.open_sidebar("system")


await accept_recovery()
