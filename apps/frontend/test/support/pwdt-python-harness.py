"""Execute the shipped PWDT prelude against an in-memory JavaScript bridge."""

import asyncio
import json
import sys
import types

request = json.load(sys.stdin)
calls = []
logs = []
snapshot = {"selectedProjectId": None, "jobStatus": None, "resultCount": 0}
save_count = 0
run_count = 0


async def invoke(action, encoded):
    global save_count, run_count
    payload = json.loads(encoded)
    calls.append(action)
    result = {"ok": True, "action": action}
    if action == "project/create":
        snapshot["selectedProjectId"] = "created-project"
        result["projectId"] = "created-project"
        if request.get("stopAt") == 0:
            snapshot["selectedProjectId"] = "other-project"
            result["contextChanged"] = True
    if action == "nav/setStudyKind":
        snapshot["studyKind"] = payload.get("studyKind")
    if action == "model/saveAs":
        save_count += 1
        result["modelId"] = f"saved-{save_count}"
        if request.get("stopAt") == save_count:
            snapshot["selectedProjectId"] = "other-project"
            result["contextChanged"] = True
    if action == "job/run":
        run_count += 1
        snapshot["jobStatus"] = request.get("jobStatus", "completed") if request.get("stopRunAt") == run_count else "completed"
        if snapshot["jobStatus"] == "completed":
            snapshot["resultCount"] += 1
    if action in {"state/projectElectrostaticToHeat", "state/projectHeatToThermo"}:
        suffix = "quad_2d" if "quad" in snapshot["studyKind"] else "triangle_2d"
        prefix = "heat_plane_" if action == "state/projectElectrostaticToHeat" else "thermal_plane_"
        snapshot["studyKind"] = prefix + suffix
        result["studyKind"] = snapshot["studyKind"]
    return json.dumps(result)


async def sleep(seconds):
    await asyncio.sleep(0)


bridge = types.SimpleNamespace(
    invoke=invoke,
    state_json=lambda: json.dumps(snapshot),
    actions_json=lambda: json.dumps(request["actions"]),
    macros_json=lambda: "[]",
    recipes_json=lambda: json.dumps(request["recipes"]),
    ui_contract_json=lambda: '{"selectors": {}, "parameterizedSelectors": []}',
    log=lambda message: logs.append(message),
    sleep=sleep,
)
js = types.ModuleType("js")
setattr(js, "__kyuubikiBridge", bridge)
sys.modules["js"] = js
namespace = {}
try:
    exec(request["prelude"], namespace)
    ky = namespace["ky"]
    ky.log("bridge", "ready")
    asyncio.run(ky.sleep())
    if request.get("recipe"):
        result = asyncio.run(ky.run_recipe(request["recipe"], {"timeoutMs": 50}))
    else:
        result = {"initialized": True, "actions": len(ky.actions())}
    outcome = {"result": result}
except Exception as error:
    outcome = {"error": str(error), "errorType": type(error).__name__}
print(json.dumps({**outcome, "calls": calls, "snapshot": snapshot, "logs": logs}))
