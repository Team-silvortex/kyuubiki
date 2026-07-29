"use client";

export { ensurePyodideRuntime } from "./workbench-script-pyodide-loader.ts";
export { DEFAULT_WORKBENCH_PYTHON } from "./workbench-script-python-default.ts";

export function buildWorkbenchPythonPrelude(): string {
  return `
import json
from js import __kyuubikiBridge

class _KyuubikiBridge:
    def state(self):
        return json.loads(__kyuubikiBridge.state_json())

    def actions(self):
        return json.loads(__kyuubikiBridge.actions_json())

    def macros(self):
        return json.loads(__kyuubikiBridge.macros_json())

    def recipes(self):
        return json.loads(__kyuubikiBridge.recipes_json())

    def ui_contract(self):
        return json.loads(__kyuubikiBridge.ui_contract_json())

    def log(self, *parts):
        __kyuubikiBridge.log(" ".join(str(part) for part in parts))

    def action(self, action_id):
        for action in self.actions():
            if action.get("id") == action_id:
                return action
        return None

    def has_action(self, action_id):
        return self.action(action_id) is not None

    def require_action(self, action_id):
        action = self.action(action_id)
        if action is None:
            raise KeyError(f"Unknown Workbench frontend action: {action_id}")
        return action

    def macro(self, macro_id):
        for macro in self.macros():
            if macro.get("id") == macro_id:
                return macro
        return None

    def has_macro(self, macro_id):
        return self.macro(macro_id) is not None

    def require_macro(self, macro_id):
        macro = self.macro(macro_id)
        if macro is None:
            raise KeyError(f"Unknown Workbench frontend macro: {macro_id}")
        return macro

    def recipe(self, recipe_id):
        for recipe in self.recipes():
            if recipe.get("id") == recipe_id:
                return recipe
        return None

    def has_recipe(self, recipe_id):
        return self.recipe(recipe_id) is not None

    def require_recipe(self, recipe_id):
        recipe = self.recipe(recipe_id)
        if recipe is None:
            raise KeyError(f"Unknown Pwdt recipe: {recipe_id}")
        return recipe

    def recipes_matching(self, category=None, risk=None):
        matches = []
        for recipe in self.recipes():
            if category is not None and recipe.get("category") != category:
                continue
            if risk is not None and recipe.get("risk") != risk:
                continue
            matches.append(recipe)
        return matches

    def actions_by_category(self, category):
        return self.actions_matching(category=category)

    def actions_matching(self, category=None, risk=None):
        matches = []
        for action in self.actions():
            if category is not None and action.get("category") != category:
                continue
            if risk is not None and action.get("risk") != risk:
                continue
            matches.append(action)
        return matches

    def selector_keys(self):
        contract = self.ui_contract()
        direct = list(contract.get("selectors", {}).keys())
        parameterized = [entry.get("key") for entry in contract.get("parameterizedSelectors", [])]
        return [key for key in direct + parameterized if key]

    def automation_parity_report(self):
        action_categories = {}
        for action in self.actions():
            category = action.get("category", "unknown")
            action_categories[category] = action_categories.get(category, 0) + 1
        return {
            "action_count": len(self.actions()),
            "macro_count": len(self.macros()),
            "recipe_count": len(self.recipes()),
            "selector_count": len(self.ui_contract().get("selectors", {})),
            "parameterized_selector_count": len(self.ui_contract().get("parameterizedSelectors", [])),
            "action_categories": action_categories,
            "high_risk_actions": [
                action.get("id")
                for action in self.actions()
                if action.get("requiresConfirmation")
            ],
            "selector_contract_version": self.ui_contract().get("contractVersion"),
            "product_owned_static_ui": self.ui_contract().get("shellExtensible") is False,
        }

    def ui_selector(self, key, value=None):
        contract = self.ui_contract()
        selectors = contract.get("selectors", {})
        if key in selectors:
            return selectors[key]
        for entry in contract.get("parameterizedSelectors", []):
            if entry.get("key") != key:
                continue
            if value is None:
                raise ValueError(f"Selector '{key}' requires a value for {entry.get('parameter')}")
            return str(entry.get("template", "")).replace("\${" + str(entry.get("parameter")) + "}", str(value))
        raise KeyError(f"Unknown selector key: {key}")

    def query_selector(self, key, value=None):
        from js import document
        return document.querySelector(self.ui_selector(key, value))

    def query_selector_all(self, key, value=None):
        from js import document
        return document.querySelectorAll(self.ui_selector(key, value))

    def selector_exists(self, key, value=None):
        return self.query_selector(key, value) is not None

    def state_value(self, key, default=None):
        return self.state().get(key, default)

    def timeout_seconds(self, params=None, default=90.0):
        if params is None:
            return float(default)
        if params.get("timeoutSeconds") is not None:
            return float(params.get("timeoutSeconds"))
        if params.get("timeoutMs") is not None:
            return float(params.get("timeoutMs")) / 1000.0
        return float(default)

    def require_selector(self, key, value=None):
        node = self.query_selector(key, value)
        if node is None:
            raise RuntimeError(f"UI selector not found: {key}")
        return node

    async def invoke(self, action, payload=None):
        if payload is None:
            payload = {}
        self.require_action(action)
        result = await __kyuubikiBridge.invoke(action, json.dumps(payload))
        return json.loads(result)

    async def open_sidebar(self, section):
        return await self.invoke("nav/setSidebarSection", {"section": section})

    async def open_tabs(self, **tabs):
        return await self.invoke("nav/setTabs", tabs)

    async def configure(self, **settings):
        return await self.invoke("settings/patch", settings)

    async def refresh_all(self):
        return await self.invoke("runtime/refreshAll")

    async def create_project(self, name, description=""):
        result = await self.invoke("project/create", {"name": name, "description": description})
        return result.get("projectId")

    async def select_project(self, project_id):
        return await self.invoke("project/select", {"projectId": project_id})

    async def ensure_project(self, name="Pwdt automation study", description="Created from Pwdt"):
        project_id = self.state().get("selectedProjectId")
        if project_id:
            return project_id
        return await self.create_project(name, description)

    async def set_study_kind(self, study_kind):
        await self.invoke("nav/setStudyKind", {"studyKind": study_kind})
        return self.state().get("studyKind")

    async def build_parametric_truss_2d(self, bays=6, span=18, height=3.5, load_y=-1500, model_name=None, material=None):
        await self.set_study_kind("truss_2d")
        await self.open_sidebar("model")
        await self.open_tabs(modelTab="tools", modelToolsPage="generate")
        if model_name is not None or material is not None:
            meta = {}
            if model_name is not None:
                meta["loadedModelName"] = model_name
            if material is not None:
                meta["activeMaterial"] = str(material)
            await self.invoke("model/setWorkspaceMeta", meta)
        await self.invoke("state/setParametric", {
            "bays": bays,
            "span": span,
            "height": height,
            "loadY": load_y,
        })
        await self.invoke("model/generateTruss")
        return self.state()

    async def prepare_electrostatic_plane_triangle_study(self, model_name=None, material=None):
        await self.set_study_kind("electrostatic_plane_triangle_2d")
        await self.open_sidebar("model")
        await self.open_tabs(modelTab="tools", modelToolsPage="study")
        if model_name is not None or material is not None:
            meta = {}
            if model_name is not None:
                meta["loadedModelName"] = model_name
            if material is not None:
                meta["activeMaterial"] = str(material)
            await self.invoke("model/setWorkspaceMeta", meta)
        return self.state()

    async def prepare_electrostatic_plane_quad_study(self, model_name=None, material=None):
        await self.set_study_kind("electrostatic_plane_quad_2d")
        await self.open_sidebar("model")
        await self.open_tabs(modelTab="tools", modelToolsPage="study")
        if model_name is not None or material is not None:
            meta = {}
            if model_name is not None:
                meta["loadedModelName"] = model_name
            if material is not None:
                meta["activeMaterial"] = str(material)
            await self.invoke("model/setWorkspaceMeta", meta)
        return self.state()

    async def prepare_heat_plane_triangle_study(self, model_name=None, material=None):
        await self.set_study_kind("heat_plane_triangle_2d")
        await self.open_sidebar("model")
        await self.open_tabs(modelTab="tools", modelToolsPage="study")
        if model_name is not None or material is not None:
            meta = {}
            if model_name is not None:
                meta["loadedModelName"] = model_name
            if material is not None:
                meta["activeMaterial"] = str(material)
            await self.invoke("model/setWorkspaceMeta", meta)
        return self.state()

    async def prepare_heat_plane_quad_study(self, model_name=None, material=None):
        await self.set_study_kind("heat_plane_quad_2d")
        await self.open_sidebar("model")
        await self.open_tabs(modelTab="tools", modelToolsPage="study")
        if model_name is not None or material is not None:
            meta = {}
            if model_name is not None:
                meta["loadedModelName"] = model_name
            if material is not None:
                meta["activeMaterial"] = str(material)
            await self.invoke("model/setWorkspaceMeta", meta)
        return self.state()

    async def save_model(self, name=None, material=None, save_as=False):
        if name is not None or material is not None:
            meta = {}
            if name is not None:
                meta["loadedModelName"] = name
            if material is not None:
                meta["activeMaterial"] = str(material)
            await self.invoke("model/setWorkspaceMeta", meta)
        action = "model/saveAs" if save_as else "model/save"
        return await self.invoke(action)

    async def run_current_study(self, timeout=90.0, interval=0.5):
        await self.invoke("job/run")
        return await self.wait_for_job_done(timeout=timeout, interval=interval)

    async def open_results(self, project_id=None, model_version_id=None):
        payload = {"activeTab": "results"}
        if project_id is not None:
            payload["projectId"] = project_id
        if model_version_id is not None:
            payload["modelVersionId"] = model_version_id
        await self.invoke("data/setFilters", payload)
        return self.state()

    async def project_heat_to_thermo_quad_study(self):
        return await self.invoke("state/projectHeatToThermo")

    async def project_heat_to_thermo_triangle_study(self):
        return await self.invoke("state/projectHeatToThermo")

    async def project_electrostatic_to_heat_quad_study(self):
        return await self.invoke("state/projectElectrostaticToHeat")

    async def project_electrostatic_to_heat_triangle_study(self):
        return await self.invoke("state/projectElectrostaticToHeat")

    async def run_closed_loop_truss_study(self, params=None):
        if params is None:
            params = {}
        project_id = await self.ensure_project(
            params.get("projectName", "Pwdt automation study"),
            params.get("projectDescription", "Created from Pwdt"),
        )
        await self.build_parametric_truss_2d(
            bays=params.get("bays", 6),
            span=params.get("span", 18),
            height=params.get("height", 3.5),
            load_y=params.get("loadY", -1500),
            model_name=params.get("modelName"),
            material=params.get("activeMaterial"),
        )
        save_result = await self.save_model(
            name=params.get("modelName"),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        await self.open_results(project_id=project_id)
        return {
            "ok": run_state.get("jobStatus") == "completed",
            "projectId": project_id,
            "saveResult": save_result,
            "jobStatus": run_state.get("jobStatus"),
            "resultCount": run_state.get("resultCount"),
        }

    async def run_heat_to_thermo_quad_study(self, params=None):
        if params is None:
            params = {}
        project_id = await self.ensure_project(
            params.get("projectName", "Pwdt heat-to-thermo quad"),
            params.get("projectDescription", "Created from Pwdt"),
        )
        await self.prepare_heat_plane_quad_study(
            model_name=params.get("heatModelName"),
            material=params.get("activeMaterial"),
        )
        heat_save_result = await self.save_model(
            name=params.get("heatModelName"),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        heat_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        thermo_projection = await self.project_heat_to_thermo_quad_study()
        thermo_save_result = await self.save_model(
            name=params.get("thermoModelName", params.get("heatModelName")),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        thermo_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        await self.open_results(project_id=project_id)
        return {
            "ok": (
                heat_run_state.get("jobStatus") == "completed"
                and thermo_projection.get("studyKind") == "thermal_plane_quad_2d"
                and thermo_run_state.get("jobStatus") == "completed"
            ),
            "projectId": project_id,
            "heatSaveResult": heat_save_result,
            "heatJobStatus": heat_run_state.get("jobStatus"),
            "thermoProjection": thermo_projection,
            "thermoSaveResult": thermo_save_result,
            "thermoJobStatus": thermo_run_state.get("jobStatus"),
            "resultCount": thermo_run_state.get("resultCount"),
        }

    async def run_heat_to_thermo_triangle_study(self, params=None):
        if params is None:
            params = {}
        project_id = await self.ensure_project(
            params.get("projectName", "Pwdt heat-to-thermo triangle"),
            params.get("projectDescription", "Created from Pwdt"),
        )
        await self.prepare_heat_plane_triangle_study(
            model_name=params.get("heatModelName"),
            material=params.get("activeMaterial"),
        )
        heat_save_result = await self.save_model(
            name=params.get("heatModelName"),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        heat_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        thermo_projection = await self.project_heat_to_thermo_triangle_study()
        thermo_save_result = await self.save_model(
            name=params.get("thermoModelName", params.get("heatModelName")),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        thermo_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        await self.open_results(project_id=project_id)
        return {
            "ok": (
                heat_run_state.get("jobStatus") == "completed"
                and thermo_projection.get("studyKind") == "thermal_plane_triangle_2d"
                and thermo_run_state.get("jobStatus") == "completed"
            ),
            "projectId": project_id,
            "heatSaveResult": heat_save_result,
            "heatJobStatus": heat_run_state.get("jobStatus"),
            "thermoProjection": thermo_projection,
            "thermoSaveResult": thermo_save_result,
            "thermoJobStatus": thermo_run_state.get("jobStatus"),
            "resultCount": thermo_run_state.get("resultCount"),
        }

    async def run_electrostatic_heat_thermo_quad_study(self, params=None):
        if params is None:
            params = {}
        project_id = await self.ensure_project(
            params.get("projectName", "Pwdt electrostatic-heat-thermo quad"),
            params.get("projectDescription", "Created from Pwdt"),
        )
        await self.prepare_electrostatic_plane_quad_study(
            model_name=params.get("electrostaticModelName"),
            material=params.get("activeMaterial"),
        )
        electrostatic_save_result = await self.save_model(
            name=params.get("electrostaticModelName"),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        electrostatic_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        heat_projection = await self.project_electrostatic_to_heat_quad_study()
        heat_save_result = await self.save_model(
            name=params.get("heatModelName"),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        heat_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        thermo_projection = await self.project_heat_to_thermo_quad_study()
        thermo_save_result = await self.save_model(
            name=params.get("thermoModelName", params.get("heatModelName")),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        thermo_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        await self.open_results(project_id=project_id)
        return {
            "ok": (
                electrostatic_run_state.get("jobStatus") == "completed"
                and heat_projection.get("studyKind") == "heat_plane_quad_2d"
                and heat_run_state.get("jobStatus") == "completed"
                and thermo_projection.get("studyKind") == "thermal_plane_quad_2d"
                and thermo_run_state.get("jobStatus") == "completed"
            ),
            "projectId": project_id,
            "electrostaticSaveResult": electrostatic_save_result,
            "electrostaticJobStatus": electrostatic_run_state.get("jobStatus"),
            "heatProjection": heat_projection,
            "heatSaveResult": heat_save_result,
            "heatJobStatus": heat_run_state.get("jobStatus"),
            "thermoProjection": thermo_projection,
            "thermoSaveResult": thermo_save_result,
            "thermoJobStatus": thermo_run_state.get("jobStatus"),
            "resultCount": thermo_run_state.get("resultCount"),
        }

    async def run_electrostatic_heat_thermo_triangle_study(self, params=None):
        if params is None:
            params = {}
        project_id = await self.ensure_project(
            params.get("projectName", "Pwdt electrostatic-heat-thermo triangle"),
            params.get("projectDescription", "Created from Pwdt"),
        )
        await self.prepare_electrostatic_plane_triangle_study(
            model_name=params.get("electrostaticModelName"),
            material=params.get("activeMaterial"),
        )
        electrostatic_save_result = await self.save_model(
            name=params.get("electrostaticModelName"),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        electrostatic_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        heat_projection = await self.project_electrostatic_to_heat_triangle_study()
        heat_save_result = await self.save_model(
            name=params.get("heatModelName"),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        heat_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        thermo_projection = await self.project_heat_to_thermo_triangle_study()
        thermo_save_result = await self.save_model(
            name=params.get("thermoModelName", params.get("heatModelName")),
            material=params.get("activeMaterial"),
            save_as=True,
        )
        thermo_run_state = await self.run_current_study(timeout=self.timeout_seconds(params))
        await self.open_results(project_id=project_id)
        return {
            "ok": (
                electrostatic_run_state.get("jobStatus") == "completed"
                and heat_projection.get("studyKind") == "heat_plane_triangle_2d"
                and heat_run_state.get("jobStatus") == "completed"
                and thermo_projection.get("studyKind") == "thermal_plane_triangle_2d"
                and thermo_run_state.get("jobStatus") == "completed"
            ),
            "projectId": project_id,
            "electrostaticSaveResult": electrostatic_save_result,
            "electrostaticJobStatus": electrostatic_run_state.get("jobStatus"),
            "heatProjection": heat_projection,
            "heatSaveResult": heat_save_result,
            "heatJobStatus": heat_run_state.get("jobStatus"),
            "thermoProjection": thermo_projection,
            "thermoSaveResult": thermo_save_result,
            "thermoJobStatus": thermo_run_state.get("jobStatus"),
            "resultCount": thermo_run_state.get("resultCount"),
        }

    async def run_recipe(self, recipe_id, params=None):
        self.require_recipe(recipe_id)
        if recipe_id == "recipe/truss2d/closed-loop":
            return await self.run_closed_loop_truss_study(params)
        if recipe_id == "recipe/heat-thermo/quad-closed-loop":
            return await self.run_heat_to_thermo_quad_study(params)
        if recipe_id == "recipe/heat-thermo/triangle-closed-loop":
            return await self.run_heat_to_thermo_triangle_study(params)
        if recipe_id == "recipe/electrostatic-heat-thermo/quad-closed-loop":
            return await self.run_electrostatic_heat_thermo_quad_study(params)
        if recipe_id == "recipe/electrostatic-heat-thermo/triangle-closed-loop":
            return await self.run_electrostatic_heat_thermo_triangle_study(params)
        raise NotImplementedError(f"Pwdt recipe is registered but not executable yet: {recipe_id}")

    async def run_macro(self, macro, payload=None):
        if payload is None:
            payload = {}
        self.require_macro(macro)
        return await self.invoke("macro/run", {"macroId": macro, **payload})

    async def run_steps(self, steps):
        results = []
        for step in steps:
            action = step.get("action")
            payload = step.get("payload", {})
            self.require_action(action)
            results.append(await self.invoke(action, payload))
        return results

    async def run_macro_definition(self, macro):
        return await self.run_steps(macro.get("steps", []))

    async def sleep(self, seconds=0.0):
        await __kyuubikiBridge.sleep(seconds)

    async def wait_until(self, predicate, timeout=30.0, interval=0.25):
        elapsed = 0.0
        while elapsed <= timeout:
            current = self.state()
            if predicate(current):
                return current
            await self.sleep(interval)
            elapsed += interval
        raise TimeoutError(f"Condition not met within {timeout} seconds")

    async def wait_for_job_done(self, timeout=90.0, interval=0.5):
        terminal = {"completed", "failed", "cancelled"}
        return await self.wait_until(
            lambda current: current.get("jobStatus") in terminal,
            timeout=timeout,
            interval=interval,
        )

    async def wait_for_message(self, text, timeout=30.0, interval=0.25):
        needle = str(text)
        return await self.wait_until(
            lambda current: needle in str(current.get("message", "")),
            timeout=timeout,
            interval=interval,
        )

ky = _KyuubikiBridge()
state = ky.state()
actions = ky.actions()
macros = ky.macros()
recipes = ky.recipes()
ui_contract = ky.ui_contract()
`;
}
