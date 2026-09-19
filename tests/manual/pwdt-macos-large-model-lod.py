# Run in the installed macOS Workbench PWDT console, not native Python.
# Requires an otherwise disposable in-memory draft. No save or solve is issued.
# Run this file once, then await lod_focus(), inspect the viewport, and run
# await lod_gpu_sweep(), then await lod_finish(). Input is generated in memory.
import gc
import json
import sys
import time
from js import window

LOD_COUNT = 100_000
LOD_TARGETS = list(range(91_910, 91_916))
lod_records = {}


def lod_probe():
    shell = window.document.querySelector(".viewport-3d-shell")
    assert shell, "3D viewport is not mounted"
    data = shell.dataset
    ids = [int(item.getAttribute("data-truss3d-node"))
           for item in shell.querySelectorAll("[data-truss3d-node]")]
    canvas = shell.querySelector("canvas")
    assert canvas and canvas.width > 0 and canvas.height > 0
    gl = canvas.getContext("webgl")
    assert gl and not gl.isContextLost(), "Native WebGL is unavailable"
    error = gl.getError()
    assert error == gl.NO_ERROR, f"WebGL error: {error}"
    result = {"nodes": int(data.lodNodes), "elements": int(data.lodElements),
              "visits": int(data.lodVisits), "zoom": float(data.cameraZoom),
              "limited": data.lodActive == "true", "ids": ids}
    assert 0 < result["nodes"] <= 1200 and result["elements"] <= 2400
    assert result["visits"] <= 14_400
    return result


def lod_emit(label, record):
    compact = {key: value for key, value in record.items() if key != "ids"}
    lod_records[label] = compact
    ky.log(label, json.dumps(compact))


async def lod_prepare():
    assert ky.state()["selectedModelId"] is None, "Open a disposable blank draft first"
    assert window.location.hostname == "127.0.0.1"
    assert window.localStorage.getItem("kyuubiki-workbench-api-base-url") is None
    started = time.perf_counter()
    nodes = [{"id": f"lod-node-{i}", "x": i % 1000, "y": 0, "z": i // 1000,
              "fix_x": i == 0, "fix_y": i == 0, "fix_z": i == 0,
              "load_x": 0, "load_y": 0, "load_z": 0} for i in range(LOD_COUNT)]
    elements = [{"id": f"lod-member-{i}", "node_i": i, "node_j": i + 1,
                 "area": 0.01, "youngs_modulus": 70e9} for i in range(LOD_COUNT - 1)]
    await ky.invoke("state/replaceTruss3dModel", {"nodes": nodes, "elements": elements})
    del nodes, elements
    gc.collect()
    await ky.invoke("viewport/set3dView", {"preset": "front", "projection": "ortho"})
    await ky.invoke("viewport/toggleFlags", {"grid": False, "nodes": True, "labels": False})
    await ky.wait_until(lambda _: window.document.querySelector('.viewport-3d-shell[data-lod-active="true"]'),
                        timeout=30, interval=0.1)
    await ky.sleep(0.25)
    initial = lod_probe()
    initial["prepare_ms"] = round((time.perf_counter() - started) * 1000)
    initial["python"] = sys.version.split()[0]
    assert max(initial["ids"]) > 95_000, "Overview samples only the input prefix"
    # Selection validates against the complete model, not the render subset.
    tail = await ky.invoke("selection/query3d", {"query": {"kind": "indices", "indices": [LOD_COUNT - 1]}})
    assert tail["selectedNodes"] == 1
    await ky.invoke("selection/set3d", {"nodeIndices": [], "anchorNodeIndex": None})
    await ky.sleep(0.25)
    idle_start = lod_probe()
    await ky.sleep(1)
    idle_end = lod_probe()
    assert idle_start == idle_end, "Idle changed the bounded resident scene"
    initial["tail_selectable"] = True
    initial["idle_stable"] = True
    lod_emit("NATIVE_LOD_OVERVIEW", initial)


async def lod_focus():
    await ky.open_sidebar("model")
    await ky.invoke("viewport/setUiState", {"immersiveViewport": True})
    await ky.invoke("selection/set3d", {"nodeIndices": LOD_TARGETS, "anchorNodeIndex": LOD_TARGETS[0]})
    started = time.perf_counter()
    await ky.invoke("viewport/focus3d")
    await ky.wait_until(lambda _: float(window.document.querySelector(".viewport-3d-shell").dataset.cameraZoom) > 10,
                        timeout=30, interval=0.05)
    await ky.sleep(0.2)
    focused = lod_probe()
    focused["settled_ms"] = round((time.perf_counter() - started) * 1000)
    assert all(index in focused["ids"] for index in LOD_TARGETS)
    panel = ky.require_selector("viewportPanel").getBoundingClientRect()
    stage = ky.require_selector("viewportStage").getBoundingClientRect()
    assert abs(panel.height - window.innerHeight) < 1
    assert stage.height > 100 and stage.bottom <= window.innerHeight + 1
    focused["panel"] = [panel.width, panel.height]
    focused["stage"] = [stage.width, stage.height]
    lod_emit("NATIVE_LOD_FOCUS", focused)


async def lod_finish():
    await ky.invoke("viewport/reset3d")
    await ky.invoke("viewport/set3dView", {"preset": "top", "projection": "persp"})
    await ky.sleep(0.25)
    final = lod_probe()
    assert final["zoom"] == 1
    await ky.invoke("viewport/setUiState", {"immersiveViewport": False})
    await ky.open_sidebar("system")
    lod_emit("NATIVE_LOD_RESET", final)
    ky.log("NATIVE_LOD_PASS", "100k display only; no native large solve/save or 1M qualification")


async def lod_gpu_sweep():
    # Instrument this canvas only; restore methods and release proxies in finally.
    # These counters verify residency, not frame-time performance.
    from pyodide.ffi import create_proxy
    gl = window.document.querySelector(".viewport-3d-shell canvas").getContext("webgl")
    original_draw, original_buffer = gl.drawArrays, gl.bufferData
    gpu = {"draws": 0, "uploads": 0, "max_points": 0, "max_lines": 0,
           "max_buffer_bytes": 0, "errors": []}

    def draw(mode, first, count):
        original_draw.call(gl, mode, first, count)
        gpu["draws"] += 1
        if mode == gl.POINTS:
            gpu["max_points"] = max(gpu["max_points"], count)
        if mode == gl.LINES:
            gpu["max_lines"] = max(gpu["max_lines"], count // 2)
        error = gl.getError()
        if error != gl.NO_ERROR:
            gpu["errors"].append(error)

    def upload(target, data, usage):
        gpu["uploads"] += 1
        gpu["max_buffer_bytes"] = max(gpu["max_buffer_bytes"], data.byteLength)
        original_buffer.call(gl, target, data, usage)

    draw_proxy, upload_proxy = create_proxy(draw), create_proxy(upload)
    gl.drawArrays, gl.bufferData = draw_proxy, upload_proxy
    try:
        await ky.invoke("selection/set3d", {"nodeIndices": LOD_TARGETS, "anchorNodeIndex": LOD_TARGETS[0]})
        for _ in range(6):
            await ky.invoke("viewport/set3dView", {"preset": "front", "projection": "ortho"})
            await ky.invoke("viewport/focus3d")
            await ky.sleep(0.15)
            assert lod_probe()["zoom"] > 10
            await ky.invoke("viewport/reset3d")
            await ky.sleep(0.15)
            assert lod_probe()["zoom"] == 1
        await ky.sleep(0.25)
        uploads_before = gpu["uploads"]
        await ky.sleep(1)
        gpu["idle_upload_delta"] = gpu["uploads"] - uploads_before
        assert gpu["draws"] > 0 and gpu["uploads"] > 0
        assert gpu["max_points"] <= 1200 and gpu["max_lines"] <= 2670
        assert gpu["max_buffer_bytes"] <= 85_440
        assert gpu["idle_upload_delta"] == 0 and not gpu["errors"]
        lod_emit("NATIVE_LOD_GPU", gpu)
    finally:
        gl.drawArrays, gl.bufferData = original_draw, original_buffer
        draw_proxy.destroy()
        upload_proxy.destroy()


await lod_prepare()
