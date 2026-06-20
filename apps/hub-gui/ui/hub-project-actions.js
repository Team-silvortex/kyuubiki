export async function runHubProjectAction(action, options) {
  switch (action) {
    case "open-workbench":
      options.setOperationOutput(await options.invokeTauri("launch_workbench_gui"));
      options.setSection("projects");
      options.setBusy(false, "ready");
      return true;
    case "open-installer":
      options.setOperationOutput(await options.invokeTauri("launch_installer_gui"));
      options.setSection("deploy");
      options.setBusy(false, "ready");
      return true;
    case "open-docs-index":
      options.setOperationOutput(await options.invokeTauri("open_docs_index"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-current-line-doc":
      options.setOperationOutput(await options.invokeTauri("open_current_line_doc"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-operations-doc":
      options.setOperationOutput(await options.invokeTauri("open_operations_doc"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-troubleshooting-doc":
      options.setOperationOutput(await options.invokeTauri("open_troubleshooting_doc"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-accuracy-plan-doc":
      options.setOperationOutput(await options.invokeTauri("open_accuracy_plan_doc"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-accuracy-baselines-doc":
      options.setOperationOutput(await options.invokeTauri("open_accuracy_baselines_doc"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-testing-and-ci-doc":
      options.setOperationOutput(await options.invokeTauri("open_testing_and_ci_doc"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-direct-mesh-baseline":
      options.setOperationOutput(await options.invokeTauri("open_direct_mesh_baseline"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "open-direct-mesh-output-dir":
      options.setOperationOutput(await options.invokeTauri("open_direct_mesh_output_dir"));
      options.setSection("projects");
      options.setProjectsPage("guides");
      options.setBusy(false, "ready");
      return true;
    case "project-inspect":
      await options.runProjectBundleAction({
        action: "project inspect",
        command: "project_bundle_inspect",
        payload: options.currentProjectBundlePayload(),
        outputTarget: options.setProjectBundleOutput,
      });
      return true;
    case "project-validate":
      await options.runProjectBundleAction({
        action: "project validate",
        command: "project_bundle_validate",
        payload: options.currentProjectBundlePayload(),
        outputTarget: options.setProjectBundleOutput,
      });
      return true;
    case "project-normalize":
      await options.runProjectBundleAction({
        action: "project normalize",
        command: "guarded_mutation_action",
        payload: options.currentProjectBundleOutputPayload(),
        outputTarget: options.setProjectBundleOutput,
      });
      return true;
    case "project-unpack":
      await options.runProjectBundleAction({
        action: "project unpack",
        command: "guarded_mutation_action",
        payload: options.currentProjectBundleOutputPayload(),
        outputTarget: options.setProjectBundleOutput,
      });
      return true;
    case "project-pack":
      await options.runProjectBundleAction({
        action: "project pack",
        command: "guarded_mutation_action",
        payload: options.currentProjectBundleOutputPayload(),
        outputTarget: options.setProjectBundleOutput,
      });
      return true;
    case "project-diff":
      await options.runProjectBundleAction({
        action: "project diff",
        command: "project_bundle_diff",
        payload: options.currentProjectBundleComparePayload(),
        outputTarget: options.setProjectBundleOutput,
      });
      return true;
    default:
      return false;
  }
}
