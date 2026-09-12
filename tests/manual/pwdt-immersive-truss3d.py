# Run in the installed Workbench's PWDT Python editor, not a headless SDK.
# Starts from a fresh workspace and creates one small, isolated test project.
# Then use the fullscreen editor to move node 3 by +0.25 along Z, undo/redo,
# save, reload, and solve again. See the native acceptance report for checks.
import json
import sys


async def accept_baseline():
    for action in ("selection/query3d", "state/replaceTruss3dModel", "model/saveAs"):
        ky.require_action(action)
    assert not ky.state().get("selectedModelId"), "Open a fresh workspace first"
    project_id = await ky.create_project(
        "Acceptance - immersive 3D",
        "Native PWDT acceptance: four-node tetrahedral truss, SI units",
    )
    points = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, 1)]
    pairs = [(0, 1), (1, 2), (2, 0), (0, 3), (1, 3), (2, 3)]
    model = {
        "nodes": [
            {"id": f"node-{i}", "x": x, "y": y, "z": z,
             "fix_x": i < 3, "fix_y": i < 3, "fix_z": i < 3,
             "load_x": 0, "load_y": 0, "load_z": -1000 if i == 3 else 0}
            for i, (x, y, z) in enumerate(points)
        ],
        "elements": [
            {"id": f"member-{i}", "node_i": start, "node_j": end,
             "area": 0.01, "youngs_modulus": 210e9}
            for i, (start, end) in enumerate(pairs)
        ],
    }
    await ky.invoke("state/replaceTruss3dModel", model)
    saved = await ky.save_model(name="Acceptance tetrahedron baseline", save_as=True)
    assert saved.get("ok"), saved
    completed = await ky.run_current_study()
    assert completed["hasResult"], completed
    selection = await ky.invoke("selection/query3d", {
        "query": {"kind": "range", "axis": "z", "min": 1, "max": 1},
    })
    assert selection["selectedNodes"] == 1, selection
    selected = ky.state()
    assert selected["selectedTruss3dNodeIndices"] == [3], selected
    assert selected["hasResult"], "Selecting nodes must not invalidate results"
    ky.log("NATIVE_BASELINE_PASS", json.dumps({
        "python": sys.version, "projectId": project_id,
        "saved": saved, "state": selected,
        "expected_apex_ux_uy_uz_m": -1000 / (210e9 * 0.01),
    }))
    await ky.open_sidebar("model")
    await ky.open_tabs(modelTab="tools", modelToolsPage="studio")


await accept_baseline()
