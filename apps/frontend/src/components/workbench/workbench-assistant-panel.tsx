"use client";

import { useState } from "react";
import type { AssistantPlan } from "@/lib/assistant/openai-compatible";
import { getWorkbenchScriptActionDefinition, isWorkbenchScriptActionHighRisk } from "@/lib/scripting/workbench-script-runtime";

type AssistantMode = "local" | "llm";

type AssistantCard = {
  id: string;
  title: string;
  summary: string;
  actionLabel: string;
  tone: "good" | "watch" | "risk";
  onAction: () => void;
};

type WorkbenchAssistantPanelProps = {
  variant?: "sidebar" | "floating";
  language: "en" | "zh" | "ja";
  mode: AssistantMode;
  onModeChange: (mode: AssistantMode) => void;
  currentStudyLabel: string;
  currentRuntimeLabel: string;
  currentJobLabel: string;
  currentResultLabel: string;
  localCards: AssistantCard[];
  promptPresets: Array<{ id: string; label: string; prompt: string }>;
  llmBaseUrl: string;
  llmApiKey: string;
  llmModel: string;
  transactions: Array<{ id: string; summary: string; createdAt: string; executedActions: string[] }>;
  onLlmBaseUrlChange: (value: string) => void;
  onLlmApiKeyChange: (value: string) => void;
  onLlmModelChange: (value: string) => void;
  onRequestPlan: (prompt: string) => Promise<AssistantPlan>;
  onExecuteLlmAction: (action: string, payload?: Record<string, unknown>, reason?: string) => Promise<void>;
  onExecuteLlmPlan: (actions: AssistantPlan["suggested_actions"], summary: string) => Promise<void>;
  onRollbackTransaction: (id: string) => void;
};

const copy = {
  en: {
    mode: "Mode",
    localMode: "Local",
    llmMode: "LLM",
    context: "Context",
    currentStudy: "Study",
    currentRuntime: "Runtime",
    currentJob: "Job",
    currentResult: "Result",
    floatingWelcomeTitle: "Start with one simple question",
    floatingWelcomeBody: "You do not need material intuition yet. Ask for a safe first sample, starter values, boundary conditions, or help reading the current result fields.",
    localEmpty: "The local assistant does not see an urgent action right now.",
    llmTitle: "Model Assist",
    llmHint: "Connect an OpenAI-compatible endpoint to get higher-level operational plans.",
    baseUrl: "Base URL",
    apiKey: "API key",
    model: "Model",
    prompt: "Request",
    promptPlaceholder: "Example: propose the safest next steps to stabilize and run this model.",
    requestPlan: "Generate plan",
    requesting: "Planning...",
    summary: "Summary",
    rationale: "Rationale",
    suggestedActions: "Suggested actions",
    noSuggestedActions: "The model returned no executable actions.",
    starterPrompts: "Starter prompts",
    usePrompt: "Use prompt",
    runAction: "Run action",
    localEngine: "Rules engine",
    llmEngine: "Remote model",
    notConfigured: "Fill in the endpoint and model before requesting an LLM plan.",
    approveExecution: "I reviewed this plan and approve execution.",
    executePlan: "Execute plan",
    transactions: "Transactions",
    noTransactions: "No assistant transactions yet.",
    rollback: "Rollback",
    confirmationRequired: "Confirmation required",
    highRiskHint: "This action will trigger an extra operator confirmation before execution.",
  },
  zh: {
    mode: "模式",
    localMode: "本地算法",
    llmMode: "大模型",
    context: "上下文",
    currentStudy: "当前研究",
    currentRuntime: "运行模式",
    currentJob: "当前任务",
    currentResult: "当前结果",
    floatingWelcomeTitle: "先从一个简单问题开始",
    floatingWelcomeBody: "你还不用先有材料背景。可以直接问我该开哪个样板、起步参数怎么填、支撑载荷怎么设，或者当前结果字段该怎么看。",
    localEmpty: "本地助手当前没有识别到紧急动作。",
    llmTitle: "模型辅助",
    llmHint: "接入 OpenAI 兼容接口后，可以拿到更高层的操作计划。",
    baseUrl: "接口地址",
    apiKey: "API Key",
    model: "模型",
    prompt: "请求",
    promptPlaceholder: "例如：请给出当前模型最稳妥的下一步修复与运行方案。",
    requestPlan: "生成计划",
    requesting: "规划中...",
    summary: "摘要",
    rationale: "理由",
    suggestedActions: "建议动作",
    noSuggestedActions: "模型没有返回可执行动作。",
    starterPrompts: "入门提问",
    usePrompt: "使用这条",
    runAction: "执行动作",
    localEngine: "规则引擎",
    llmEngine: "远程模型",
    notConfigured: "请先填好接口地址和模型名，再请求大模型计划。",
    approveExecution: "我已经检查这份计划，允许执行。",
    executePlan: "执行整份计划",
    transactions: "事务记录",
    noTransactions: "还没有助手事务记录。",
    rollback: "回滚",
    confirmationRequired: "需要确认",
    highRiskHint: "这个动作在真正执行前，还会额外弹出一次操作员确认。",
  },
  ja: {
    mode: "モード",
    localMode: "ローカル",
    llmMode: "LLM",
    context: "コンテキスト",
    currentStudy: "解析",
    currentRuntime: "実行モード",
    currentJob: "ジョブ",
    currentResult: "結果",
    floatingWelcomeTitle: "まずは一つだけ聞いてみましょう",
    floatingWelcomeBody: "材料の直感がなくても大丈夫です。最初のサンプル、初期値、境界条件、結果の読み方をそのまま聞けます。",
    localEmpty: "今すぐ必要なローカル支援アクションは見つかりませんでした。",
    llmTitle: "モデル支援",
    llmHint: "OpenAI 互換エンドポイントをつなぐと、より高レベルな作業プランを返せます。",
    baseUrl: "Base URL",
    apiKey: "API キー",
    model: "モデル",
    prompt: "依頼",
    promptPlaceholder: "例: このモデルを安全に安定化して実行する次の手順を提案してください。",
    requestPlan: "プラン生成",
    requesting: "計画中...",
    summary: "概要",
    rationale: "理由",
    suggestedActions: "提案アクション",
    noSuggestedActions: "実行可能なアクションは返されませんでした。",
    starterPrompts: "スタータープロンプト",
    usePrompt: "使う",
    runAction: "実行",
    localEngine: "ルールエンジン",
    llmEngine: "リモートモデル",
    notConfigured: "LLM プランを要求する前に、エンドポイントとモデル名を入力してください。",
    approveExecution: "このプランを確認し、実行を許可します。",
    executePlan: "プランを実行",
    transactions: "トランザクション",
    noTransactions: "まだアシスタントのトランザクションはありません。",
    rollback: "ロールバック",
    confirmationRequired: "確認が必要",
    highRiskHint: "このアクションは実行前に追加のオペレーター確認が入ります。",
  },
} as const;

export function WorkbenchAssistantPanel({
  variant = "sidebar",
  language,
  mode,
  onModeChange,
  currentStudyLabel,
  currentRuntimeLabel,
  currentJobLabel,
  currentResultLabel,
  localCards,
  promptPresets,
  llmBaseUrl,
  llmApiKey,
  llmModel,
  transactions,
  onLlmBaseUrlChange,
  onLlmApiKeyChange,
  onLlmModelChange,
  onRequestPlan,
  onExecuteLlmAction,
  onExecuteLlmPlan,
  onRollbackTransaction,
}: WorkbenchAssistantPanelProps) {
  const t = copy[language];
  const floating = variant === "floating";
  const [prompt, setPrompt] = useState("");
  const [plan, setPlan] = useState<AssistantPlan | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [approvedExecution, setApprovedExecution] = useState(false);

  const requestPlan = async () => {
    if (!llmBaseUrl.trim() || !llmModel.trim()) {
      setError(t.notConfigured);
      return;
    }

    setPending(true);
    setError(null);

    try {
      const nextPlan = await onRequestPlan(prompt.trim() || t.promptPlaceholder);
      setPlan(nextPlan);
      setApprovedExecution(false);
    } catch (requestError) {
      setError(requestError instanceof Error ? requestError.message : String(requestError));
    } finally {
      setPending(false);
    }
  };

  const handlePromptKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    if (pending) return;
    void requestPlan();
  };

  return (
    <>
      {floating ? (
        <section className="assistant-panel__modebar">
          <div className="panel-tabs">
            <button className={`panel-tab${mode === "local" ? " panel-tab--active" : ""}`} onClick={() => onModeChange("local")} type="button">
              {t.localMode}
            </button>
            <button className={`panel-tab${mode === "llm" ? " panel-tab--active" : ""}`} onClick={() => onModeChange("llm")} type="button">
              {t.llmMode}
            </button>
          </div>
        </section>
      ) : (
        <section className="sidebar-card sidebar-card--compact">
          <div className="card-head">
            <h2>{language === "zh" ? "助手" : language === "ja" ? "アシスタント" : "Assistant"}</h2>
            <span>{mode === "local" ? t.localEngine : t.llmEngine}</span>
          </div>
          <div className="panel-tabs">
            <button className={`panel-tab${mode === "local" ? " panel-tab--active" : ""}`} onClick={() => onModeChange("local")} type="button">
              {t.localMode}
            </button>
            <button className={`panel-tab${mode === "llm" ? " panel-tab--active" : ""}`} onClick={() => onModeChange("llm")} type="button">
              {t.llmMode}
            </button>
          </div>
        </section>
      )}

      <section className="sidebar-card sidebar-card--compact">
        <div className="card-head">
          <h2>{t.context}</h2>
          <span>{currentStudyLabel}</span>
        </div>
        <div className="sidebar-list">
          <div>
            <span>{t.currentStudy}</span>
            <strong>{currentStudyLabel}</strong>
          </div>
          <div>
            <span>{t.currentRuntime}</span>
            <strong>{currentRuntimeLabel}</strong>
          </div>
          <div>
            <span>{t.currentJob}</span>
            <strong>{currentJobLabel}</strong>
          </div>
          <div>
            <span>{t.currentResult}</span>
            <strong>{currentResultLabel}</strong>
          </div>
        </div>
      </section>

      {floating ? (
        <section className="sidebar-card sidebar-card--compact assistant-panel__welcome">
          <div className="card-head">
            <h2>{t.floatingWelcomeTitle}</h2>
            <span>{currentStudyLabel}</span>
          </div>
          <p className="card-copy">{t.floatingWelcomeBody}</p>
        </section>
      ) : null}

      {mode === "local" ? (
        localCards.length === 0 ? (
          <section className="sidebar-card sidebar-card--compact">
            <div className="card-head">
              <h2>{t.localMode}</h2>
              <span>{t.localEngine}</span>
            </div>
            <p className="card-copy">{t.localEmpty}</p>
          </section>
        ) : (
          localCards.slice(0, 5).map((card) => (
            <section className="sidebar-card sidebar-card--compact" key={card.id}>
              <div className="card-head">
                <h2>{card.title}</h2>
                <span className={`status-chip status-chip--${card.tone}`}>{card.tone}</span>
              </div>
              <p className="card-copy">{card.summary}</p>
              <div className="button-row">
                <button className="ghost-button" onClick={card.onAction} type="button">
                  {card.actionLabel}
                </button>
              </div>
            </section>
          ))
        )
      ) : (
        <>
          <section className="sidebar-card sidebar-card--compact">
            <div className="card-head">
              <h2>{t.llmTitle}</h2>
              <span>{t.llmEngine}</span>
            </div>
            <p className="card-copy">{t.llmHint}</p>
            {promptPresets.length ? (
              <div className="script-panel__payload">
                <span>{t.starterPrompts}</span>
                <div className="button-row">
                  {promptPresets.map((preset) => (
                    <button
                      className="ghost-button ghost-button--compact"
                      key={preset.id}
                      onClick={() => setPrompt(preset.prompt)}
                      type="button"
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>
              </div>
            ) : null}
            <div className="form-grid compact">
              <label className="field-span-2">
                <span>{t.baseUrl}</span>
                <input type="text" value={llmBaseUrl} onChange={(event) => onLlmBaseUrlChange(event.target.value)} />
              </label>
              <label className="field-span-2">
                <span>{t.apiKey}</span>
                <input type="password" value={llmApiKey} onChange={(event) => onLlmApiKeyChange(event.target.value)} />
              </label>
              <label className="field-span-2">
                <span>{t.model}</span>
                <input type="text" value={llmModel} onChange={(event) => onLlmModelChange(event.target.value)} />
              </label>
              <label className="field-span-2">
                <span>{t.prompt}</span>
                <textarea
                  rows={4}
                  value={prompt}
                  placeholder={t.promptPlaceholder}
                  onChange={(event) => setPrompt(event.target.value)}
                  onKeyDown={handlePromptKeyDown}
                />
              </label>
            </div>
            {error ? <p className="card-copy">{error}</p> : null}
            <div className="button-row">
              <button className="ghost-button" disabled={pending} onClick={() => void requestPlan()} type="button">
                {pending ? t.requesting : t.requestPlan}
              </button>
            </div>
          </section>

          {plan ? (
            <>
              <section className="sidebar-card sidebar-card--compact">
                <div className="card-head">
                  <h2>{t.summary}</h2>
                  <span>{plan.suggested_actions.length}</span>
                </div>
                <p className="card-copy">{plan.summary || "--"}</p>
                <p className="card-copy">{plan.rationale || "--"}</p>
              </section>
              <section className="sidebar-card sidebar-card--compact">
                <div className="card-head">
                  <h2>{t.suggestedActions}</h2>
                  <span>{plan.suggested_actions.length}</span>
                </div>
                {plan.suggested_actions.length === 0 ? (
                  <p className="card-copy">{t.noSuggestedActions}</p>
                ) : (
                  plan.suggested_actions.map((entry, index) => {
                    const actionDefinition = getWorkbenchScriptActionDefinition(entry.action);
                    const highRisk = isWorkbenchScriptActionHighRisk(entry.action);
                    return (
                      <article className="script-panel__action" key={`${entry.action}-${index}`}>
                        <div className="script-panel__action-head">
                          <strong>{entry.action}</strong>
                          <span>{highRisk ? t.confirmationRequired : t.llmMode}</span>
                        </div>
                        {actionDefinition ? (
                          <p className="card-copy">{language === "zh" ? actionDefinition.summary.zh : actionDefinition.summary.en}</p>
                        ) : null}
                        <p className="card-copy">{entry.reason}</p>
                        {highRisk ? <p className="card-copy">{t.highRiskHint}</p> : null}
                        <div className="script-panel__payload">
                          <span>Payload</span>
                          <code>{JSON.stringify(entry.payload ?? {}, null, 2)}</code>
                        </div>
                        <div className="button-row">
                          <button
                            className="ghost-button ghost-button--compact"
                            disabled={!approvedExecution}
                            onClick={() => void onExecuteLlmAction(entry.action, entry.payload, entry.reason)}
                            type="button"
                          >
                            {t.runAction}
                          </button>
                        </div>
                      </article>
                    );
                  })
                )}
                {plan.suggested_actions.length > 0 ? (
                  <>
                    <label className="toggle-row">
                      <div>
                        <span>{t.approveExecution}</span>
                      </div>
                      <input
                        checked={approvedExecution}
                        onChange={(event) => setApprovedExecution(event.target.checked)}
                        type="checkbox"
                      />
                    </label>
                    <div className="button-row">
                      <button
                        className="ghost-button"
                        disabled={!approvedExecution || pending}
                        onClick={() => void onExecuteLlmPlan(plan.suggested_actions, plan.summary || t.llmTitle)}
                        type="button"
                      >
                        {t.executePlan}
                      </button>
                    </div>
                  </>
                ) : null}
              </section>
            </>
          ) : null}

          <section className="sidebar-card sidebar-card--compact">
            <div className="card-head">
              <h2>{t.transactions}</h2>
              <span>{transactions.length}</span>
            </div>
            {transactions.length === 0 ? (
              <p className="card-copy">{t.noTransactions}</p>
            ) : (
              transactions.map((entry) => (
                <article className="script-panel__action" key={entry.id}>
                  <div className="script-panel__action-head">
                    <strong>{entry.summary}</strong>
                    <span>{entry.executedActions.length}</span>
                  </div>
                  <p className="card-copy">{entry.createdAt}</p>
                  <div className="script-panel__payload">
                    <span>Actions</span>
                    <code>{entry.executedActions.join(", ")}</code>
                  </div>
                  <div className="button-row">
                    <button className="ghost-button ghost-button--compact" onClick={() => onRollbackTransaction(entry.id)} type="button">
                      {t.rollback}
                    </button>
                  </div>
                </article>
              ))
            )}
          </section>
        </>
      )}
    </>
  );
}
