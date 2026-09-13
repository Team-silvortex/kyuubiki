# Run in installed Workbench PWDT after reopening the existing tall tetrahedron.
# Creates one small fork with two versions; it never overwrites the source model.
import json
import sys


async def accept_checkpoint():
    before = ky.state()
    assert before["loadedModelName"] == "Acceptance tetrahedron tall variant"
    assert before["studyKind"] == "truss_3d"
    assert before["selectedProjectId"] and before["selectedVersionId"]
    created = await ky.save_model(name="Acceptance checkpoint h1.25", save_as=True)
    assert created["ok"] and created["modelId"] != before["selectedModelId"]
    first_version = ky.state()["selectedVersionId"]
    await ky.invoke("state/applyModelBatch", {
        "query": {"kind": "indices", "indices": [3]},
        "operation": {"kind": "translate", "offset": {"x": 0, "y": 0, "z": 0.25}},
    })
    saved = await ky.save_model(name="Acceptance checkpoint h1.50", material="210")
    assert saved["ok"] and saved["versionId"] != first_version
    state = ky.state()
    try:
        await ky.save_model(name=" ", material="70")
    except Exception as error:
        assert "model:invalid_name" in str(error), str(error)
    else:
        raise AssertionError("An empty name must not create a checkpoint")
    assert ky.state()["loadedModelName"] == state["loadedModelName"]
    assert ky.state()["activeMaterial"] == state["activeMaterial"]
    assert ky.state()["selectedVersionId"] == saved["versionId"]
    await ky.refresh_all()
    assert ky.state()["selectedVersionId"] == saved["versionId"]
    completed = await ky.run_current_study()
    assert completed["hasResult"] and completed["jobStatus"] == "completed"
    ky.log("NATIVE_CHECKPOINT_PASS", json.dumps({
        "python": sys.version.split()[0], "projectId": state["selectedProjectId"],
        "sourceModelId": before["selectedModelId"], "sourceVersionId": before["selectedVersionId"],
        "modelId": created["modelId"], "firstVersionId": first_version, "versionId": saved["versionId"],
        "invalidNamePreservedMetadata": True, "jobStatus": completed["jobStatus"],
        "expected_uz_m": -1000 * 1.5 / (210e9 * 0.01),
        "expected_ux_uy_m": -1000 * 1.5 * 1.5 / (210e9 * 0.01),
    }))


await accept_checkpoint()
