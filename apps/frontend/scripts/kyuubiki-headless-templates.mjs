const HEADLESS_TEMPLATES = [
  {
    id: "solve_wait_result",
    title: "Solve From Version",
    description: "Start from a saved model version and go straight to final result.",
    runtimeStyle: "service_only",
    category: "solver",
    tags: ["solve", "result", "version"],
    workflow: {
      id: "template.solve_wait_result",
      steps: [
        {
          action: "solve_and_wait_from_model_version",
          payload: {
            model_version_id: "ver_123",
            endpoints: ["http://127.0.0.1:7001"],
            timeout_ms: 60000,
          },
        },
      ],
    },
  },
  {
    id: "workflow_submit_monitor",
    title: "Workflow Submit",
    description: "Submit a workflow job and keep follow-up polling and result fetch explicit.",
    runtimeStyle: "service_only",
    category: "orchestration",
    tags: ["workflow", "job", "polling"],
    workflow: {
      id: "template.workflow_submit_monitor",
      steps: [
        {
          action: "workflow_submit_catalog",
          payload: {
            workflow_id: "wf_demo",
            input_artifacts: {},
          },
        },
        {
          action: "job_wait",
          payload: {
            job_id: "{{steps.1.result.job_id}}",
            interval_ms: 1000,
            timeout_ms: 60000,
          },
        },
        {
          action: "result_fetch",
          payload: {
            job_id: "{{steps.1.result.job_id}}",
          },
        },
      ],
    },
  },
  {
    id: "direct_mesh_pipeline",
    title: "Direct Mesh Solve",
    description: "Resolve from a raw mesh payload and keep the job follow-up explicit.",
    runtimeStyle: "service_only",
    category: "mesh",
    tags: ["mesh", "direct", "solve"],
    workflow: {
      id: "template.direct_mesh_pipeline",
      steps: [
        {
          action: "direct_mesh_solve",
          payload: {
            study_kind: "truss_3d",
            input: { nodes: [], elements: [] },
            endpoints: ["http://127.0.0.1:7001"],
          },
        },
        {
          action: "job_wait",
          payload: {
            job_id: "{{steps.1.result.job_id}}",
            interval_ms: 1000,
            timeout_ms: 60000,
          },
        },
        {
          action: "result_fetch",
          payload: {
            job_id: "{{steps.1.result.job_id}}",
          },
        },
      ],
    },
  },
  {
    id: "browser_capture_review",
    title: "Browser Capture Review",
    description: "Open a page, wait for a stable target, then capture a review snapshot.",
    runtimeStyle: "browser_only",
    category: "browser",
    tags: ["browser", "snapshot", "review"],
    workflow: {
      id: "template.browser_capture_review",
      steps: [
        {
          action: "open_page",
          payload: {
            url: "https://example.com",
            waitUntil: "domcontentloaded",
          },
        },
        {
          action: "wait",
          payload: {
            selector: "body",
            timeout: 1500,
          },
        },
        {
          action: "snapshot",
          payload: {
            file: "browser-review.png",
            fullPage: true,
          },
        },
      ],
    },
  },
  {
    id: "browser_submit_then_poll",
    title: "Browser Submit And Poll",
    description: "Drive a browser-side submit action, then switch to service-side job polling and result fetch.",
    runtimeStyle: "hybrid",
    category: "hybrid",
    tags: ["browser", "service", "job"],
    workflow: {
      id: "template.browser_submit_then_poll",
      steps: [
        {
          action: "open_page",
          payload: {
            url: "https://example.com/jobs",
            waitUntil: "domcontentloaded",
          },
        },
        {
          action: "click",
          payload: {
            selector: "[data-run-job]",
          },
        },
        {
          action: "job_wait",
          payload: {
            job_id: "job_123",
            interval_ms: 1000,
            timeout_ms: 60000,
          },
        },
        {
          action: "result_fetch",
          payload: {
            job_id: "job_123",
          },
        },
      ],
    },
  },
];

function normalizeRuntimeStyle(runtimeStyle) {
  if (typeof runtimeStyle !== "string") return null;
  const normalized = runtimeStyle.trim().toLowerCase();
  if (!normalized) return null;
  return normalized;
}

function normalizeTemplateQuery(query) {
  if (typeof query !== "string") return [];
  return query
    .toLowerCase()
    .split(/\s+/)
    .map((token) => token.trim())
    .filter(Boolean);
}

function buildSearchHaystacks(template) {
  return {
    id: template.id.toLowerCase(),
    title: template.title.toLowerCase(),
    description: template.description.toLowerCase(),
    category: template.category.toLowerCase(),
    tags: template.tags.map((tag) => tag.toLowerCase()),
    actions: template.workflow.steps.map((step) => step.action.toLowerCase()),
  };
}

function templateMatchesQuery(template, queryTokens) {
  if (queryTokens.length === 0) return { matched: true, matchFields: [] };
  const haystacks = buildSearchHaystacks(template);
  const fieldNames = Object.keys(haystacks);
  const matchedFields = new Set();
  for (const token of queryTokens) {
    let tokenMatched = false;
    for (const fieldName of fieldNames) {
      const value = haystacks[fieldName];
      const matched = Array.isArray(value)
        ? value.some((entry) => entry.includes(token))
        : value.includes(token);
      if (matched) {
        tokenMatched = true;
        matchedFields.add(fieldName);
      }
    }
    if (!tokenMatched) return { matched: false, matchFields: [] };
  }
  return { matched: true, matchFields: [...matchedFields] };
}

function filterTemplates(options = {}) {
  const runtimeStyle = normalizeRuntimeStyle(options.runtimeStyle);
  const queryTokens = normalizeTemplateQuery(options.query);
  return HEADLESS_TEMPLATES
    .filter((template) => !runtimeStyle || template.runtimeStyle === runtimeStyle)
    .map((template) => ({ template, search: templateMatchesQuery(template, queryTokens) }))
    .filter((entry) => entry.search.matched);
}

export function listHeadlessTemplates(options = {}) {
  return filterTemplates(options).map(({ template, search }) => ({
    id: template.id,
    title: template.title,
    description: template.description,
    runtime_style: template.runtimeStyle,
    category: template.category,
    tags: [...template.tags],
    step_count: template.workflow.steps.length,
    actions: template.workflow.steps.map((step) => step.action),
    matched_fields: [...search.matchFields],
  }));
}

export function getHeadlessTemplate(templateId, options = {}) {
  const normalized = String(templateId ?? "").trim();
  if (!normalized) return null;
  return filterTemplates(options).find((entry) => entry.template.id === normalized)?.template ?? null;
}

export function resolveHeadlessTemplateSelection(options = {}) {
  const templateId = typeof options.templateId === "string" ? options.templateId.trim() : "";
  const runtimeStyle = normalizeRuntimeStyle(options.runtimeStyle);
  if (templateId) return getHeadlessTemplate(templateId, { runtimeStyle });
  const matches = filterTemplates({ runtimeStyle, query: options.query });
  return matches.length === 1 ? matches[0].template : null;
}

export function buildHeadlessTemplateDocument(template, options = {}) {
  const workflowId = typeof options.workflowId === "string" && options.workflowId.trim()
    ? options.workflowId.trim()
    : template.workflow.id;
  return {
    schema_version: "kyuubiki.headless-workflow/v1",
    exported_at: new Date().toISOString(),
    language: "en",
    workflow: {
      id: workflowId,
      steps: template.workflow.steps.map((step) => ({
        action: step.action,
        payload: structuredClone(step.payload),
      })),
    },
    actions: [],
    template: {
      id: template.id,
      title: template.title,
      description: template.description,
      runtime_style: template.runtimeStyle,
      category: template.category,
      tags: [...template.tags],
    },
  };
}
