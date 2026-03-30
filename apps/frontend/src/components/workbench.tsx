"use client";

import { useEffect, useState, useTransition } from "react";
import { MATERIAL_PRESETS } from "@/lib/materials";
import { parsePlaygroundModel } from "@/lib/model-import";
import {
  fetchHealth,
  runAxialBarJob,
  type AxialBarJobInput,
  type AxialBarJobPayload,
  type HealthPayload,
} from "@/lib/api";

type Language = "en" | "zh";
type Theme = "linen" | "marine" | "graphite";

type FormState = {
  length: number;
  area: number;
  elements: number;
  tipForce: number;
  material: string;
  youngsModulusGpa: number;
};

const defaultState: FormState = {
  length: 1.2,
  area: 0.01,
  elements: 6,
  tipForce: 1800,
  material: "210",
  youngsModulusGpa: 210,
};

const SETTINGS_KEY = "kyuubiki-workbench-settings";

const copy = {
  en: {
    eyebrow: "Kyuubiki Structural Workbench",
    title: "A focused browser workbench for orchestrated FEM runs",
    subtitle:
      "Next.js owns the UI. The Elixir orchestrator handles jobs. Rust executes the numerical solve over TCP.",
    tabs: ["Workspace", "Studies", "Materials", "Reports"],
    navigator: "Navigator",
    modelName: "Model",
    analysisCase: "Analysis case",
    objectBody: "Axial member",
    objectMesh: "Linear mesh",
    objectLoad: "Tip force",
    objectConstraint: "Fixed support",
    importModel: "Import model",
    importHint: "JSON parameter file for the axial-bar study",
    sampleJson: "Sample model",
    backend: "Backend",
    apiOnline: "API online",
    apiOffline: "API offline",
    viewport: "Study View",
    viewportLabel: "Axial response preview",
    undeformed: "Undeformed",
    deformed: "Deformed",
    orchestration: "Orchestration",
    orchestrator: "Elixir orchestrator",
    solverAgent: "Rust solver agent",
    reportView: "Results Console",
    solverPanel: "Study Controls",
    settings: "Settings",
    theme: "Theme",
    language: "Language",
    themes: {
      linen: "Linen Draft",
      marine: "Marine Grid",
      graphite: "Graphite Lab",
    },
    languages: {
      en: "English",
      zh: "中文",
    },
    length: "Span (m)",
    area: "Area (m²)",
    material: "Material",
    modulus: "Young's modulus (GPa)",
    elements: "Elements",
    tipForce: "Tip force (N)",
    solve: "Run study",
    solving: "Dispatching job...",
    statusReady: "ready",
    statusBusy: "busy",
    activeStudy: "Active study",
    studyType: "1D axial member",
    orchestrationState: "Job state",
    worker: "Worker",
    maxStress: "Max stress",
    reaction: "Reaction",
    tipDisp: "Tip displacement",
    nodes: "Nodes",
    messages: "Messages",
    messagesHint: "Operational feed from the workbench and orchestrator.",
    elementTable: "Element results",
    span: "Span",
    stress: "Stress (Pa)",
    axialForce: "Axial force (N)",
    importFailed: "Import failed",
    importedModel: "Imported model",
    initialLoaded: "Workbench connected to the orchestrator",
    initialFailed: "Unable to reach the orchestrator",
    dispatching: "Submitting FEM job to the orchestrator",
    completed: "completed",
    defaultModel: "manual-study",
    serviceUi: "Unified Next.js workbench",
  },
  zh: {
    eyebrow: "Kyuubiki 结构分析工作台",
    title: "一个聚焦、正式的浏览器 FEM 工作台",
    subtitle: "Next.js 负责界面，Elixir 负责编排任务，Rust 通过 TCP 负责数值求解。",
    tabs: ["工作区", "研究", "材料", "报告"],
    navigator: "导航器",
    modelName: "模型",
    analysisCase: "分析案例",
    objectBody: "轴向构件",
    objectMesh: "线性网格",
    objectLoad: "端部载荷",
    objectConstraint: "固定约束",
    importModel: "导入模型",
    importHint: "导入轴向杆件分析的 JSON 参数文件",
    sampleJson: "样例模型",
    backend: "后端",
    apiOnline: "API 在线",
    apiOffline: "API 离线",
    viewport: "研究视图",
    viewportLabel: "轴向响应预览",
    undeformed: "原始形态",
    deformed: "变形形态",
    orchestration: "编排链路",
    orchestrator: "Elixir 编排器",
    solverAgent: "Rust 求解代理",
    reportView: "结果控制台",
    solverPanel: "研究控制",
    settings: "设置",
    theme: "主题",
    language: "语言",
    themes: {
      linen: "纸面浅色",
      marine: "海图网格",
      graphite: "石墨实验室",
    },
    languages: {
      en: "English",
      zh: "中文",
    },
    length: "跨度 (m)",
    area: "截面积 (m²)",
    material: "材料",
    modulus: "弹性模量 (GPa)",
    elements: "单元数",
    tipForce: "端部载荷 (N)",
    solve: "运行研究",
    solving: "正在分发任务...",
    statusReady: "就绪",
    statusBusy: "处理中",
    activeStudy: "当前研究",
    studyType: "一维轴向构件",
    orchestrationState: "任务状态",
    worker: "执行器",
    maxStress: "最大应力",
    reaction: "反力",
    tipDisp: "端部位移",
    nodes: "节点数",
    messages: "消息",
    messagesHint: "来自工作台与编排器的运行消息。",
    elementTable: "单元结果",
    span: "区间",
    stress: "应力 (Pa)",
    axialForce: "轴力 (N)",
    importFailed: "导入失败",
    importedModel: "已导入模型",
    initialLoaded: "工作台已连接到编排器",
    initialFailed: "无法连接到编排器",
    dispatching: "正在向编排器提交 FEM 任务",
    completed: "已完成",
    defaultModel: "手动研究",
    serviceUi: "统一 Next.js 工作台",
  },
} as const;

function toApiInput(state: FormState): AxialBarJobInput {
  return {
    length: state.length,
    area: state.area,
    elements: state.elements,
    tip_force: state.tipForce,
    youngs_modulus_gpa: state.youngsModulusGpa,
  };
}

function scientific(value: number, digits = 3): string {
  return value.toExponential(digits);
}

function fixed(value: number, digits = 2): string {
  return value.toFixed(digits);
}

function localMaterialLabel(value: string, language: Language): string {
  const labels = {
    en: {
      "210": "Steel",
      "70": "Aluminum",
      "116": "Titanium",
      "30": "Concrete",
      "135": "Carbon fiber",
      custom: "Custom",
    },
    zh: {
      "210": "钢",
      "70": "铝",
      "116": "钛",
      "30": "混凝土",
      "135": "碳纤维",
      custom: "自定义",
    },
  } as const;

  return labels[language][value as keyof (typeof labels)[Language]] ?? labels[language].custom;
}

function safeStorageGet(): { theme?: Theme; language?: Language } {
  if (typeof window === "undefined") {
    return {};
  }

  try {
    const raw = window.localStorage.getItem(SETTINGS_KEY);
    return raw ? (JSON.parse(raw) as { theme?: Theme; language?: Language }) : {};
  } catch {
    return {};
  }
}

export function Workbench() {
  const [form, setForm] = useState<FormState>(defaultState);
  const [result, setResult] = useState<AxialBarJobPayload | null>(null);
  const [health, setHealth] = useState<HealthPayload | null>(null);
  const [loadedModelName, setLoadedModelName] = useState<string>(copy.en.defaultModel);
  const [message, setMessage] = useState<string>(copy.en.initialLoaded);
  const [language, setLanguage] = useState<Language>("en");
  const [theme, setTheme] = useState<Theme>("linen");
  const [isPending, startTransition] = useTransition();
  const t = copy[language];

  useEffect(() => {
    const stored = safeStorageGet();

    if (stored.theme) {
      setTheme(stored.theme);
    }

    if (stored.language) {
      setLanguage(stored.language);
      setLoadedModelName(copy[stored.language].defaultModel);
      setMessage(copy[stored.language].initialLoaded);
    }
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;

    if (typeof window !== "undefined") {
      window.localStorage.setItem(SETTINGS_KEY, JSON.stringify({ theme, language }));
    }
  }, [theme, language]);

  useEffect(() => {
    let active = true;

    startTransition(async () => {
      try {
        const [healthPayload, analysisPayload] = await Promise.all([
          fetchHealth(),
          runAxialBarJob(toApiInput(defaultState)),
        ]);

        if (!active) {
          return;
        }

        setHealth(healthPayload);
        setResult(analysisPayload);
        setMessage(copy[language].initialLoaded);
      } catch (error) {
        if (!active) {
          return;
        }

        setHealth(null);
        setMessage(error instanceof Error ? error.message : copy[language].initialFailed);
      }
    });

    return () => {
      active = false;
    };
  }, [language]);

  const handleFieldChange = (key: keyof FormState, value: number | string) => {
    setForm((current) => ({ ...current, [key]: value }));
  };

  const handleMaterialChange = (value: string) => {
    const preset = MATERIAL_PRESETS.find((item) => item.value === value);

    setForm((current) => ({
      ...current,
      material: value,
      youngsModulusGpa: preset?.modulusGpa ?? current.youngsModulusGpa,
    }));
  };

  const handleLanguageChange = (nextLanguage: Language) => {
    setLanguage(nextLanguage);
    setLoadedModelName((current) =>
      current === copy.en.defaultModel || current === copy.zh.defaultModel
        ? copy[nextLanguage].defaultModel
        : current
    );
  };

  const runAnalysis = () => {
    setMessage(t.dispatching);

    startTransition(async () => {
      try {
        const payload = await runAxialBarJob(toApiInput(form));
        setResult(payload);
        setMessage(`${t.activeStudy}: ${payload.job.job_id} ${t.completed}`);
      } catch (error) {
        setMessage(error instanceof Error ? error.message : t.initialFailed);
      }
    });
  };

  const importModel = async (file: File | undefined) => {
    if (!file) {
      return;
    }

    try {
      const imported = parsePlaygroundModel(await file.text());

      setForm({
        length: imported.length,
        area: imported.area,
        elements: imported.elements,
        tipForce: imported.tipForce,
        material: imported.material,
        youngsModulusGpa: imported.youngsModulusGpa,
      });
      setLoadedModelName(imported.name);
      setMessage(`${t.importedModel}: ${imported.name}`);
    } catch (error) {
      setMessage(error instanceof Error ? `${t.importFailed}: ${error.message}` : t.importFailed);
    }
  };

  const inputLength = result?.result.input.length ?? form.length;
  const nodes = result?.result.nodes ?? [];
  const elements = result?.result.elements ?? [];
  const maxDisplacement = result?.result.max_displacement ?? 0;
  const scale = maxDisplacement === 0 ? 1 : 140 / maxDisplacement;

  return (
    <div className="workbench-shell">
      <header className="hero panel">
        <div className="hero-copy">
          <p className="eyebrow">{t.eyebrow}</p>
          <h1>{t.title}</h1>
          <p className="hero-subtitle">{t.subtitle}</p>
        </div>
        <div className="hero-meta">
          <div className="tab-row">
            {t.tabs.map((tab, index) => (
              <span key={tab} className={`tab-pill${index === 0 ? " tab-pill--active" : ""}`}>
                {tab}
              </span>
            ))}
          </div>
          <div className="service-cards">
            <div className="service-card">
              <span>{t.serviceUi}</span>
              <strong>Next.js</strong>
            </div>
            <div className="service-card">
              <span>{t.orchestrator}</span>
              <strong>{health?.status === "ok" ? t.apiOnline : t.apiOffline}</strong>
            </div>
            <div className="service-card">
              <span>{t.solverAgent}</span>
              <strong>TCP :5001</strong>
            </div>
          </div>
        </div>
      </header>

      <section className="shell-grid">
        <aside className="panel navigator-panel">
          <div className="panel-head">
            <h2>{t.navigator}</h2>
            <span>{loadedModelName}</span>
          </div>
          <div className="tree-section">
            <span className="tree-kicker">{t.modelName}</span>
            <ul className="tree">
              <li>{t.analysisCase}</li>
              <li>{t.objectBody}</li>
              <li>{t.objectMesh}: {form.elements}</li>
              <li>{t.objectLoad}: {fixed(form.tipForce, 0)} N</li>
              <li>{t.objectConstraint}</li>
              <li>{t.material}: {localMaterialLabel(form.material, language)}</li>
            </ul>
          </div>

          <label className="import-box">
            <span>{t.importModel}</span>
            <small>{t.importHint}</small>
            <input
              type="file"
              accept=".json,application/json"
              onChange={(event) => importModel(event.target.files?.[0])}
            />
          </label>

          <a className="sample-link" href="/models/axial-steel-bar.json" target="_blank" rel="noreferrer">
            {t.sampleJson}
          </a>

          <div className="backend-panel">
            <div className="panel-head panel-head--compact">
              <h3>{t.backend}</h3>
              <span>{health?.status ?? "offline"}</span>
            </div>
            <div className="backend-grid">
              <div>
                <span>{t.orchestrator}</span>
                <strong>{health ? "HTTP :4000" : t.apiOffline}</strong>
              </div>
              <div>
                <span>{t.solverAgent}</span>
                <strong>{health?.transport?.solver_agent_tcp ?? 5001}</strong>
              </div>
            </div>
          </div>
        </aside>

        <main className="panel viewport-panel">
          <div className="panel-head">
            <h2>{t.viewport}</h2>
            <span>{result?.job.status ?? "idle"}</span>
          </div>
          <svg viewBox="0 0 980 480" className="viewport-svg" aria-label={t.viewportLabel}>
            <defs>
              <linearGradient id="beamGradient" x1="0%" y1="0%" x2="100%" y2="0%">
                <stop offset="0%" stopColor="var(--accent-cool)" />
                <stop offset="100%" stopColor="var(--accent)" />
              </linearGradient>
            </defs>
            <rect x="18" y="18" width="944" height="444" rx="26" className="viewport-frame" />
            <text x="52" y="60" className="svg-title">
              {t.viewportLabel}
            </text>
            <text x="54" y="118" className="legend-label">
              {t.undeformed}
            </text>
            <text x="54" y="338" className="legend-label">
              {t.deformed}
            </text>
            <line x1="84" y1="160" x2="884" y2="160" className="guide" />
            <line x1="84" y1="360" x2="884" y2="360" className="guide guide--soft" />
            {nodes.length > 0 ? (
              <>
                <polyline
                  points={nodes
                    .map((node) => `${84 + (node.x / inputLength) * 800},160`)
                    .join(" ")}
                  className="bar bar--base"
                />
                <polyline
                  points={nodes
                    .map((node) => {
                      const x = 84 + (node.x / inputLength) * 800 + node.displacement * scale;
                      return `${x},360`;
                    })
                    .join(" ")}
                  className="bar bar--deformed"
                />
                {nodes.map((node) => {
                  const baseX = 84 + (node.x / inputLength) * 800;
                  const deformedX = baseX + node.displacement * scale;

                  return (
                    <g key={node.index}>
                      <circle cx={baseX} cy={160} r="5" className="node-base" />
                      <circle cx={deformedX} cy={360} r="6" className="node-deformed" />
                      <text x={baseX} y={136} textAnchor="middle" className="node-label">
                        n{node.index}
                      </text>
                      <text x={deformedX} y={388} textAnchor="middle" className="node-label">
                        {scientific(node.displacement, 2)}
                      </text>
                    </g>
                  );
                })}
              </>
            ) : null}
          </svg>

          <div className="summary-strip">
            <div>
              <span>{t.activeStudy}</span>
              <strong>{result?.job.job_id ?? t.studyType}</strong>
            </div>
            <div>
              <span>{t.orchestrationState}</span>
              <strong>{result?.job.status ?? "idle"}</strong>
            </div>
            <div>
              <span>{t.worker}</span>
              <strong>{result?.job.worker_id ?? "unassigned"}</strong>
            </div>
            <div>
              <span>{t.tipDisp}</span>
              <strong>{result ? scientific(result.result.tip_displacement) : "--"}</strong>
            </div>
          </div>
        </main>

        <aside className="panel control-panel">
          <div className="panel-head">
            <h2>{t.solverPanel}</h2>
            <span>{isPending ? t.statusBusy : t.statusReady}</span>
          </div>

          <div className="settings-card">
            <h3>{t.settings}</h3>
            <div className="settings-grid">
              <label>
                <span>{t.theme}</span>
                <select value={theme} onChange={(event) => setTheme(event.target.value as Theme)}>
                  <option value="linen">{t.themes.linen}</option>
                  <option value="marine">{t.themes.marine}</option>
                  <option value="graphite">{t.themes.graphite}</option>
                </select>
              </label>
              <label>
                <span>{t.language}</span>
                <select
                  value={language}
                  onChange={(event) => handleLanguageChange(event.target.value as Language)}
                >
                  <option value="en">{t.languages.en}</option>
                  <option value="zh">{t.languages.zh}</option>
                </select>
              </label>
            </div>
          </div>

          <div className="form-grid">
            <label>
              <span>{t.length}</span>
              <input
                type="number"
                value={form.length}
                min={0.1}
                step={0.1}
                onChange={(event) => handleFieldChange("length", Number(event.target.value))}
              />
            </label>
            <label>
              <span>{t.area}</span>
              <input
                type="number"
                value={form.area}
                min={0.0001}
                step={0.0001}
                onChange={(event) => handleFieldChange("area", Number(event.target.value))}
              />
            </label>
            <label>
              <span>{t.material}</span>
              <select value={form.material} onChange={(event) => handleMaterialChange(event.target.value)}>
                {MATERIAL_PRESETS.map((preset) => (
                  <option key={preset.value} value={preset.value}>
                    {localMaterialLabel(preset.value, language)}
                  </option>
                ))}
              </select>
            </label>
            <label>
              <span>{t.modulus}</span>
              <input
                type="number"
                value={form.youngsModulusGpa}
                min={0.1}
                step={0.1}
                onChange={(event) =>
                  handleFieldChange("youngsModulusGpa", Number(event.target.value))
                }
              />
            </label>
            <label>
              <span>{t.elements}</span>
              <input
                type="number"
                value={form.elements}
                min={1}
                max={48}
                step={1}
                onChange={(event) => handleFieldChange("elements", Number(event.target.value))}
              />
            </label>
            <label>
              <span>{t.tipForce}</span>
              <input
                type="number"
                value={form.tipForce}
                step={100}
                onChange={(event) => handleFieldChange("tipForce", Number(event.target.value))}
              />
            </label>
          </div>

          <button className="solve-button" disabled={isPending} onClick={runAnalysis}>
            {isPending ? t.solving : t.solve}
          </button>

          <div className="metrics-grid">
            <div className="metric-card">
              <span>{t.maxStress}</span>
              <strong>{result ? scientific(result.result.max_stress) : "--"}</strong>
            </div>
            <div className="metric-card">
              <span>{t.reaction}</span>
              <strong>{result ? scientific(result.result.reaction_force) : "--"}</strong>
            </div>
            <div className="metric-card">
              <span>{t.nodes}</span>
              <strong>{result?.result.nodes.length ?? 0}</strong>
            </div>
          </div>
        </aside>
      </section>

      <section className="panel bottom-console">
        <div className="panel-head">
          <h2>{t.reportView}</h2>
          <span>{message}</span>
        </div>
        <div className="console-grid">
          <div className="console-card">
            <h3>{t.messages}</h3>
            <p>{t.messagesHint}</p>
            <div className="message-line">{message}</div>
            <div className="orchestration-flow">
              <span>{t.orchestration}</span>
              <strong>Next.js UI</strong>
              <strong>{t.orchestrator}</strong>
              <strong>{t.solverAgent}</strong>
            </div>
          </div>
          <div className="console-card">
            <h3>{t.elementTable}</h3>
            <div className="table-scroll">
              <table>
                <thead>
                  <tr>
                    <th>#</th>
                    <th>{t.span}</th>
                    <th>{t.stress}</th>
                    <th>{t.axialForce}</th>
                  </tr>
                </thead>
                <tbody>
                  {elements.map((element) => (
                    <tr key={element.index}>
                      <td>{element.index}</td>
                      <td>
                        {fixed(element.x1, 2)} - {fixed(element.x2, 2)}
                      </td>
                      <td>{scientific(element.stress)}</td>
                      <td>{scientific(element.axial_force)}</td>
                    </tr>
                  ))}
                  {elements.length === 0 ? (
                    <tr>
                      <td colSpan={4}>--</td>
                    </tr>
                  ) : null}
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </section>
    </div>
  );
}
