# Run in installed Workbench PWDT with an existing acceptance project selected.
# Requires data revision 2 and the matching checkpoint backend. Replaces only the
# working geometry and creates one fork with two versions. Retain the prefix and
# selected project across a full runtime/WebView restart to verify durable replay.
# A new independent acceptance run needs a new prefix; never use a research model.
import json
import sys

REQUEST_PREFIX = "native-checkpoint-20260913"


def tetrahedron(height):
    points = [(0, 0, 0), (1, 0, 0), (0, 1, 0), (0, 0, height)]
    pairs = [(0, 1), (1, 2), (2, 0), (0, 3), (1, 3), (2, 3)]
    return {
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


async def accept_checkpoint_retry():
    project_id = ky.state()["selectedProjectId"]
    assert project_id, "Select the isolated acceptance project first"
    await ky.invoke("state/replaceTruss3dModel", tetrahedron(1.25))
    initial = dict(name="Acceptance retry h1.25", material="210", save_as=True,
                   request_id=REQUEST_PREFIX + "-model")
    created = await ky.save_model(**initial)
    assert created["ok"] and not created.get("contextChanged"), created
    model_id = created["modelId"]
    first_version = ky.state()["selectedVersionId"]
    repeated = await ky.save_model(**initial)
    assert repeated["ok"] and repeated["modelId"] == model_id, repeated
    assert ky.state()["selectedVersionId"] == first_version

    await ky.invoke("state/applyModelBatch", {
        "query": {"kind": "indices", "indices": [3]},
        "operation": {"kind": "translate", "offset": {"x": 0, "y": 0, "z": 0.25}},
    })
    checkpoint = dict(name="Acceptance retry h1.50", material="210",
                      request_id=REQUEST_PREFIX + "-version")
    saved = await ky.save_model(**checkpoint)
    assert saved["ok"] and not saved.get("contextChanged"), saved
    version_id = saved["versionId"]
    assert version_id != first_version
    repeated = await ky.save_model(**checkpoint)
    assert repeated["ok"] and repeated["versionId"] == version_id, repeated

    before_conflict = ky.state()
    try:
        await ky.save_model(**{**checkpoint, "name": "Must not replace checkpoint"})
    except Exception as error:
        assert "checkpoint_request_conflict" in str(error), str(error)
    else:
        raise AssertionError("Changed content must not reuse a committed request ID")
    for key in ("loadedModelName", "activeMaterial", "selectedVersionId"):
        assert ky.state()[key] == before_conflict[key], key

    # Replaying the initial receipt may select v1 locally, but must not rewind
    # the server's newer latest-version pointer. Independently inspect that API.
    await ky.invoke("state/replaceTruss3dModel", tetrahedron(1.25))
    repeated = await ky.save_model(**initial)
    assert repeated["modelId"] == model_id
    assert ky.state()["selectedVersionId"] == first_version
    await ky.invoke("state/replaceTruss3dModel", tetrahedron(1.50))
    repeated = await ky.save_model(**checkpoint)
    assert repeated["versionId"] == version_id
    await ky.refresh_all()
    completed = await ky.run_current_study()
    assert completed["hasResult"] and completed["jobStatus"] == "completed"
    ky.log("NATIVE_RETRY_PASS", json.dumps({
        "python": sys.version.split()[0], "project": project_id,
        "model": model_id, "v1": first_version, "v2": version_id,
        "conflictRejected": True, "job": completed["jobStatus"],
    }))


await accept_checkpoint_retry()
