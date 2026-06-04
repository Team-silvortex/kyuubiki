"use client";

import { useMemo } from "react";
import { requestWorkbenchAssistantPlan, type AssistantPlan } from "@/lib/assistant/openai-compatible";
import { SAMPLE_LIBRARY } from "@/lib/models";
import type { WorkbenchCopy, WorkbenchLanguage } from "@/components/workbench/workbench-copy";
import type { TrussDiagnostics, TrussSuggestion } from "@/components/workbench/workbench-defaults";
import type { WorkbenchScriptSnapshot } from "@/lib/scripting/workbench-script-runtime";
import { classifyStudyKindDomain, classifyStudyKindFamily } from "@/lib/workbench/view-models";

export type WorkbenchAssistantCard = {
  id: string;
  title: string;
  summary: string;
  actionLabel: string;
  tone: "good" | "watch" | "risk";
  onAction: () => void;
};

export type WorkbenchAssistantPromptPreset = {
  id: string;
  label: string;
  prompt: string;
};

type AssistantControllerDeps = {
  t: WorkbenchCopy;
  language: WorkbenchLanguage;
  studyKind: string;
  frontendRuntimeMode: "orchestrated_gui" | "direct_mesh_gui";
  selectedProjectId: string | null;
  directMeshEndpointsCount: number;
  hasHealth: boolean;
  jobIsActive: boolean;
  isTruss: boolean;
  isTruss3d: boolean;
  immersiveViewport: boolean;
  hasAnyResult: boolean;
  trussDiagnostics: TrussDiagnostics | null;
  openProjects: () => void;
  openSystemConfig: () => void;
  refreshHealth: () => void;
  cancelCurrentJob: () => void;
  applyTrussSuggestion: (suggestion: TrussSuggestion) => void;
  openSample: (href: string) => void;
  openWorkspaceStudy: (tab: "summary" | "controls") => void;
  runAnalysis: () => void;
  downloadResultCsv: () => void;
  toggleImmersiveViewport: () => void;
  assistantApiBaseUrl: string;
  assistantApiKey: string;
  assistantModel: string;
  getScriptSnapshot: () => WorkbenchScriptSnapshot;
};

const studyKindLabel = (t: WorkbenchCopy, studyKind: string) =>
  t.kinds[studyKind as keyof typeof t.kinds] ?? studyKind;

export function useWorkbenchAssistantController({
  t,
  language,
  studyKind,
  frontendRuntimeMode,
  selectedProjectId,
  directMeshEndpointsCount,
  hasHealth,
  jobIsActive,
  isTruss,
  isTruss3d,
  immersiveViewport,
  hasAnyResult,
  trussDiagnostics,
  openProjects,
  openSystemConfig,
  refreshHealth,
  cancelCurrentJob,
  applyTrussSuggestion,
  openSample,
  openWorkspaceStudy,
  runAnalysis,
  downloadResultCsv,
  toggleImmersiveViewport,
  assistantApiBaseUrl,
  assistantApiKey,
  assistantModel,
  getScriptSnapshot,
}: AssistantControllerDeps) {
  const currentStudyDomain = classifyStudyKindDomain(studyKind);
  const assistantStudyFamily = classifyStudyKindFamily(studyKind);

  const currentStudySample = useMemo(
    () =>
      SAMPLE_LIBRARY.find((sample) => sample.kind === studyKind) ??
      SAMPLE_LIBRARY.find(
        (sample) =>
          classifyStudyKindDomain(sample.kind) === currentStudyDomain &&
          classifyStudyKindFamily(sample.kind) === assistantStudyFamily,
      ) ??
      null,
    [assistantStudyFamily, currentStudyDomain, studyKind],
  );

  const assistantCards = useMemo<WorkbenchAssistantCard[]>(() => {
    const cards: WorkbenchAssistantCard[] = [];

    if (!selectedProjectId) {
      cards.push({
        id: "project",
        title: t.assistantNeedsProject,
        summary: t.assistantNeedsProjectHint,
        actionLabel: t.assistantOpenProjects,
        tone: "watch",
        onAction: openProjects,
      });
    }

    if (frontendRuntimeMode === "direct_mesh_gui" && directMeshEndpointsCount === 0) {
      cards.push({
        id: "direct-mesh",
        title: t.assistantConfigureDirectMesh,
        summary: t.assistantConfigureDirectMeshHint,
        actionLabel: t.settings,
        tone: "risk",
        onAction: openSystemConfig,
      });
    }

    if (!hasHealth) {
      cards.push({
        id: "runtime",
        title: t.assistantRefreshRuntime,
        summary: t.assistantRefreshRuntimeHint,
        actionLabel: t.refresh,
        tone: "watch",
        onAction: refreshHealth,
      });
    }

    if (jobIsActive) {
      cards.push({
        id: "job",
        title: t.assistantCancelRun,
        summary: t.assistantCancelRunHint,
        actionLabel: t.cancelJob,
        tone: "watch",
        onAction: cancelCurrentJob,
      });
    } else if (isTruss && trussDiagnostics?.blockingMessages.length && trussDiagnostics.suggestions[0]) {
      cards.push({
        id: "fix",
        title: t.assistantApplyFix,
        summary: `${t.assistantApplyFixHint} ${trussDiagnostics.blockingMessages[0]}`,
        actionLabel: t.assistantApplyFix,
        tone: "risk",
        onAction: () => applyTrussSuggestion(trussDiagnostics.suggestions[0]),
      });
    } else if (!hasAnyResult) {
      if (currentStudySample) {
        cards.push({
          id: "sample",
          title: t.assistantOpenSample,
          summary: `${t.assistantOpenSampleHint} ${currentStudySample.name}.`,
          actionLabel: currentStudySample.name,
          tone: "good",
          onAction: () => openSample(currentStudySample.href),
        });
      }
      cards.push({
        id: "controls",
        title: t.assistantReviewControls,
        summary: t.assistantReviewControlsHint,
        actionLabel: t.controls,
        tone: "watch",
        onAction: () => openWorkspaceStudy("controls"),
      });
      cards.push({
        id: "run",
        title: t.assistantRunStudy,
        summary: t.assistantRunStudyHint,
        actionLabel: t.run,
        tone: "good",
        onAction: runAnalysis,
      });
    } else {
      cards.push({
        id: "report",
        title: t.assistantReviewReport,
        summary: t.assistantReviewReportHint,
        actionLabel: t.overview,
        tone: "good",
        onAction: () => openWorkspaceStudy("summary"),
      });
      cards.push({
        id: "export",
        title: t.assistantExportResult,
        summary: t.assistantExportResultHint,
        actionLabel: t.exportCsv,
        tone: "watch",
        onAction: downloadResultCsv,
      });
    }

    if (isTruss3d && !immersiveViewport) {
      cards.push({
        id: "immersive",
        title: t.assistantEnterImmersive,
        summary: t.assistantEnterImmersiveHint,
        actionLabel: t.enterImmersive,
        tone: "good",
        onAction: toggleImmersiveViewport,
      });
    }

    return cards;
  }, [
    applyTrussSuggestion,
    assistantStudyFamily,
    cancelCurrentJob,
    currentStudySample,
    directMeshEndpointsCount,
    downloadResultCsv,
    frontendRuntimeMode,
    hasAnyResult,
    hasHealth,
    immersiveViewport,
    isTruss,
    isTruss3d,
    jobIsActive,
    openProjects,
    openSample,
    openSystemConfig,
    openWorkspaceStudy,
    refreshHealth,
    runAnalysis,
    selectedProjectId,
    t,
    toggleImmersiveViewport,
    trussDiagnostics,
  ]);

  const assistantPromptPresets = useMemo<WorkbenchAssistantPromptPreset[]>(
    () => [
      {
        id: "explain",
        label: t.assistantPromptExplain,
        prompt:
          language === "zh"
            ? `我现在在做 ${studyKindLabel(t, studyKind)}。请用小白能懂的话解释这个 study 是算什么的、最重要的输入是什么、我第一次运行前应该先检查哪三件事。`
            : language === "ja"
              ? `今は ${studyKindLabel(t, studyKind)} を扱っています。初心者にも分かる言葉で、この study が何を解くのか、重要な入力は何か、最初の実行前に確認すべき三つのことを教えてください。`
              : `I am working on ${studyKindLabel(t, studyKind)}. Explain this study in beginner-friendly language, name the most important inputs, and tell me the first three things to check before my first run.`,
      },
      {
        id: "materials",
        label: t.assistantPromptMaterial,
        prompt:
          language === "zh"
            ? `我不是材料专业的。针对 ${studyKindLabel(t, studyKind)}，请给我一套保守的起步材料参数建议，并说明哪些参数最值得先保持默认。`
            : language === "ja"
              ? `私は材料の専門家ではありません。${studyKindLabel(t, studyKind)} に対して、保守的な初期材料値の組み合わせを提案し、まずはどのパラメータを既定値に近いままにしておくのが安全か教えてください。`
              : `I am not a materials specialist. For ${studyKindLabel(t, studyKind)}, suggest a conservative starter set of material values and explain which parameters are safest to leave near defaults first.`,
      },
      {
        id: "boundary",
        label: t.assistantPromptBoundary,
        prompt:
          language === "zh"
            ? `请根据当前 ${studyKindLabel(t, studyKind)} 的上下文，帮我检查支撑和载荷应该怎么设才更像一个合理的第一轮仿真，并提醒我常见过约束或漏约束风险。`
            : language === "ja"
              ? `現在の ${studyKindLabel(t, studyKind)} の文脈に基づいて、最初の試行として妥当な支持条件と荷重の置き方を一緒に確認し、拘束不足や過拘束のよくある失敗も教えてください。`
              : `Given the current ${studyKindLabel(t, studyKind)} context, help me choose a sensible first-pass set of supports and loads, and warn me about common under-constrained or over-constrained mistakes.`,
      },
      {
        id: "results",
        label: t.assistantPromptResults,
        prompt:
          language === "zh"
            ? `请按 ${studyKindLabel(t, studyKind)} 的结果语义，告诉我当前最应该先看哪些结果字段，以及看到什么数量级时应该保持警惕。`
            : language === "ja"
              ? `${studyKindLabel(t, studyKind)} の結果の意味に沿って、まずどの結果フィールドを見るべきか、そしてどんな桁や傾向が出たら注意すべきか教えてください。`
              : `For ${studyKindLabel(t, studyKind)}, tell me which result fields I should read first and what kinds of magnitudes or patterns should make me cautious.`,
      },
    ],
    [language, studyKind, t],
  );

  const requestLlmAssistantPlan = async (prompt: string): Promise<AssistantPlan> =>
    requestWorkbenchAssistantPlan({
      baseUrl: assistantApiBaseUrl,
      apiKey: assistantApiKey,
      model: assistantModel,
      prompt,
      snapshot: getScriptSnapshot(),
      localHints: assistantCards.map((card) => ({
        id: card.id,
        title: card.title,
        summary: card.summary,
        actionLabel: card.actionLabel,
      })),
    });

  return {
    assistantCards,
    assistantPromptPresets,
    requestLlmAssistantPlan,
  };
}
