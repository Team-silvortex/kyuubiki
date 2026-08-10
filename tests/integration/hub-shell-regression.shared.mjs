import assert from "node:assert/strict";
import {
  assertActionInvokes,
  assertLanguageChange,
  assertNoPageErrors,
  assertTauriInvocations,
  overlaps,
  visibleRects,
} from "./desktop-shell-regression.shared.mjs";

export async function assertHubRegression(page, viewport) {
  await page.setViewportSize(viewport);
  await page.goto(page.url(), { waitUntil: "networkidle", timeout: 60_000 });
  await page.waitForSelector('html[data-hub-ready="true"] .hub-head #home-mainline-entry');

  const homeLayout = await page.evaluate(() => {
    const header = document.querySelector(".hub-head")?.getBoundingClientRect();
    const mainline = document.querySelector("#home-mainline-entry")?.getBoundingClientRect();
    const mainlineTitle = document.querySelector("#home-mainline-title")?.getBoundingClientRect();
    const mainlineCopy = document.querySelector("#home-mainline-copy")?.getBoundingClientRect();
    const eventBar = document.querySelector(".hub-event-bar")?.getBoundingClientRect();
    const aside = document.querySelector(".hub-head__aside");
    const navigation = Array.from(document.querySelectorAll(".hub-nav__item")).map((element) => {
      const rect = element.getBoundingClientRect();
      return { bottom: rect.bottom, top: rect.top };
    });
    const steps = Array.from(document.querySelectorAll(".hub-mainline-step")).map((element) => {
      const rect = element.getBoundingClientRect();
      return {
        bottom: rect.bottom,
        left: rect.left,
        right: rect.right,
        top: rect.top,
      };
    });
    return {
      headerHeight: header?.height || 0,
      headerBottom: header?.bottom || 0,
      headerWidth: header?.width || 0,
      asideText: aside?.textContent?.replace(/\s+/g, " ").trim() || "",
      mainlineBottom: mainline?.bottom || 0,
      mainlineInHeader: document.querySelector("#home-mainline-entry")?.parentElement
        ?.classList.contains("hub-head") || false,
      mainlineTop: mainline?.top || 0,
      mainlineWidth: mainline?.width || 0,
      eventTop: eventBar?.top || window.innerHeight,
      mainlineCopy: mainlineCopy ? {
        bottom: mainlineCopy.bottom,
        left: mainlineCopy.left,
        right: mainlineCopy.right,
        top: mainlineCopy.top,
      } : null,
      mainlineTitle: mainlineTitle ? {
        bottom: mainlineTitle.bottom,
        left: mainlineTitle.left,
        right: mainlineTitle.right,
        top: mainlineTitle.top,
      } : null,
      navigation,
      steps,
      viewportHeight: window.innerHeight,
      assistantPromptCount: document.querySelectorAll("#assistant-local-prompt").length,
      docsTitleCount: document.querySelectorAll("#assistant-docs-label").length,
    };
  });
  assert.ok(
    homeLayout.headerHeight >= homeLayout.viewportHeight * 0.38,
    "Hub executable runway should keep priority over secondary guide content",
  );
  assert.equal(homeLayout.mainlineInHeader, true, "Hub mainline should live in the primary header");
  assert.equal(homeLayout.steps.length, 4, "Hub should expose four stable mainline steps");
  assert.match(
    homeLayout.asideText,
    /Language.+Action status.+Start local stack.+Validate env/i,
    "Hub controls should be hydrated before the shell is considered ready",
  );
  assert.equal(
    overlaps(homeLayout.mainlineTitle, homeLayout.mainlineCopy),
    false,
    "Hub mainline title and explanation should not overlap",
  );
  assert.ok(
    homeLayout.mainlineWidth >= homeLayout.headerWidth * 0.9,
    "Hub mainline should span the primary workspace",
  );
  assert.ok(
    homeLayout.mainlineTop > 0
      && homeLayout.mainlineBottom <= homeLayout.headerBottom + 1
      && homeLayout.mainlineBottom < homeLayout.eventTop,
    "Hub mainline should remain fully visible in the initial viewport",
  );
  assert.ok(
    homeLayout.navigation.length >= 5
      && homeLayout.navigation.every((item) => item.top >= 0 && item.bottom < homeLayout.eventTop),
    "Hub primary navigation should remain visible above the event bar",
  );
  for (let index = 1; index < homeLayout.steps.length; index += 1) {
    assert.equal(
      overlaps(homeLayout.steps[index - 1], homeLayout.steps[index]),
      false,
      "Hub mainline steps should not overlap",
    );
  }
  assert.equal(homeLayout.assistantPromptCount, 1, "Hub assistant prompt should mount once");
  assert.equal(homeLayout.docsTitleCount, 1, "Hub assistant docs should mount once");

  await page.locator("#projects-tab-guides").click();
  await page.waitForSelector('[data-projects-pane="guides"]:not(.hidden) #guides-gate-status-value');
  await page.waitForFunction(() => {
    const status = document.querySelector("#guides-gate-status-value")?.textContent?.trim();
    return status && status !== "loading";
  });

  assert.equal(await page.locator("#guides-gate-status-value").textContent(), "warn");
  assert.equal(await page.locator("#guides-gate-warning-count").textContent(), "1");
  assert.equal(await page.locator("#guides-gate-failing-count").textContent(), "0");
  assert.equal(await page.locator("#guides-gate-lane-count").textContent(), "3");

  const reasons = await page.locator("#guides-gate-reasons").textContent();
  assert.match(reasons, /Workflow catalog:/);
  assert.match(reasons, /median regression 308%/);

  const rects = await visibleRects(page, [
    '[data-projects-pane="guides"]:not(.hidden) .hub-card:nth-of-type(1)',
    '[data-projects-pane="guides"]:not(.hidden) .hub-card:nth-of-type(2)',
  ]);
  rects.forEach((rect) => {
    assert.equal(rect.exists, true, `${rect.selector} should exist`);
    assert.ok(rect.width > 40, `${rect.selector} should have width`);
    assert.ok(rect.height > 40, `${rect.selector} should have height`);
  });
  assert.equal(overlaps(rects[0], rects[1]), false, "Hub guides cards should not overlap");
  await assertLanguageChange(page, "zh");

  await page.locator("#projects-tab-start").click();
  await page.waitForSelector("#home-mainline-step4");
  await assertActionInvokes(
    page,
    "open-workbench",
    "launch_workbench_gui",
    undefined,
    "#home-mainline-step4",
  );

  await page.locator("#projects-tab-guides").click();
  await page.waitForSelector('[data-projects-pane="guides"]:not(.hidden) #guides-gate-status-value');
  await assertActionInvokes(page, "open-docs-index", "open_docs_index");

  await page.locator("#projects-tab-bundles").click();
  await page.waitForSelector('[data-projects-pane="bundles"]:not(.hidden) #project-bundle-path');
  await page.locator("#project-bundle-path").fill("");
  await assertActionInvokes(
    page,
    "project-create",
    "guarded_mutation_action",
    "project_bundle_create",
    "#bundles-action-create",
    { acceptConfirmation: true },
  );
  assert.equal(
    await page.locator("#project-bundle-path").inputValue(),
    "/tmp/ui-created.kyuubiki",
    "created bundle must become the active path for the next operation",
  );
  await page.waitForFunction(
    () => document.querySelector("#project-bundle-output")?.textContent?.includes("ui-created.kyuubiki"),
  );
  await assertActionInvokes(
    page,
    "project-inspect",
    "project_bundle_inspect",
    undefined,
    "#bundles-action-inspect",
  );
  await assertActionInvokes(
    page,
    "project-validate",
    "project_bundle_validate",
    undefined,
    "#bundles-action-validate",
  );
  await page.locator("#project-bundle-out-path").fill("/tmp/ui-output.kyuubiki");
  await page.locator("#project-bundle-compare-path").fill("/tmp/ui-compare.kyuubiki");
  await assertActionInvokes(
    page,
    "project-normalize",
    "guarded_mutation_action",
    "project_bundle_normalize",
    "#bundles-action-normalize",
    { acceptConfirmation: true },
  );
  await assertActionInvokes(
    page,
    "project-unpack",
    "guarded_mutation_action",
    "project_bundle_unpack",
    "#bundles-action-unpack",
    { acceptConfirmation: true },
  );
  await assertActionInvokes(
    page,
    "project-pack",
    "guarded_mutation_action",
    "project_bundle_pack",
    "#bundles-action-pack",
    { acceptConfirmation: true },
  );
  await assertActionInvokes(
    page,
    "project-diff",
    "project_bundle_diff",
    undefined,
    "#bundles-action-diff",
  );

  await assertTauriInvocations(page, ["hub_environment", "hub_regression_gate_report"]);
  await assertNoPageErrors(page);
}
