"use client";

import { buildWorkbenchSnapshot, restoreWorkbenchSnapshot } from "@/lib/workbench/history";
import { getWorkbenchScriptActionDefinition } from "@/lib/scripting/workbench-script-runtime";
import { evaluateWorkbenchUxActionGuardrail } from "@/components/workbench/workbench-ux-action-guardrails";

export function buildWorkbenchScriptSnapshot(props: Record<string, any>) {
  return {
    studyKind: props.studyKind,
    sidebarSection: props.sidebarSection,
    studyTab: props.studyTab,
    modelTab: props.modelTab,
    libraryTab: props.libraryTab,
    systemPanelTab: props.systemPanelTab,
    systemDataTab: props.systemDataTab,
    language: props.language,
    theme: props.theme,
    frontendRuntimeMode: props.frontendRuntimeMode,
    selectedProjectId: props.selectedProjectId,
    selectedModelId: props.selectedModelId,
    selectedVersionId: props.selectedVersionId,
    selectedAdminJobId: props.selectedAdminJobId,
    selectedAdminResultJobId: props.selectedAdminResultJobId,
    adminFilterProjectId: props.adminFilterProjectId,
    adminFilterModelVersionId: props.adminFilterModelVersionId,
    loadedModelName: props.loadedModelName,
    activeMaterial: props.activeMaterial,
    selectedNode: props.selectedNode,
    selectedElement: props.selectedElement,
    selectedTruss3dNodeIndices: props.selectedTruss3dNodes,
    memberDraftNodeIndices: props.memberDraftNodes,
    immersiveViewport: props.immersiveViewport,
    immersiveToolDrawerOpen: props.immersiveToolDrawerOpen,
    immersiveHelpDrawerOpen: props.immersiveHelpDrawerOpen,
    truss3dProjectionMode: props.truss3dProjectionMode,
    truss3dViewPreset: props.truss3dViewPreset,
    truss3dBoxSelectMode: props.truss3dBoxSelectMode,
    truss3dLinkMode: props.truss3dLinkMode,
    hasResult: props.hasAnyResult,
    jobStatus: props.job?.status ?? null,
    projectCount: props.projects?.length ?? 0,
    jobHistoryCount: props.jobHistory?.length ?? 0,
    resultCount: props.resultRecords?.length ?? 0,
    protocolAgentCount: props.protocolAgents?.length ?? 0,
    healthStatus: props.health?.status ?? null,
    message: props.message,
  };
}

export function buildWorkbenchUiSnapshot(props: Record<string, any>) {
  return buildWorkbenchSnapshot({
    studyKind: props.studyKind,
    axialForm: props.axialForm,
    heatBarModel: props.heatBarModel,
    heatPlaneModel: props.heatPlaneModel,
    thermalBarModel: props.thermalBarModel,
    thermalBeamModel: props.thermalBeamModel,
    thermalFrameModel: props.thermalFrameModel,
    thermalTrussModel: props.thermalTrussModel,
    trussModel: props.trussModel,
    thermalTruss3dModel: props.thermalTruss3dModel,
    truss3dModel: props.truss3dModel,
    planeModel: props.planeModel,
    frameModel: props.frameModel,
    beamModel: props.beamModel,
    torsionModel: props.torsionModel,
    springModel: props.springModel,
    spring2dModel: props.spring2dModel,
    spring3dModel: props.spring3dModel,
    parametric: props.parametric,
    panelParametric: props.panelParametric,
    activeMaterial: props.activeMaterial,
    loadedModelName: props.loadedModelName,
    sidebarSection: props.sidebarSection,
    selectedNode: props.selectedNode,
    selectedElement: props.selectedElement,
    memberDraftNodes: props.memberDraftNodes,
  });
}

export function restoreWorkbenchUiSnapshot(snapshot: any, props: Record<string, any>) {
  restoreWorkbenchSnapshot(
    snapshot,
    {
      setStudyKind: props.setStudyKind,
      setAxialForm: props.setAxialForm,
      setHeatBarModel: props.setHeatBarModel,
      setHeatPlaneModel: props.setHeatPlaneModel,
      setThermalBarModel: props.setThermalBarModel,
      setThermalBeamModel: props.setThermalBeamModel,
      setThermalFrameModel: props.setThermalFrameModel,
      setThermalTrussModel: props.setThermalTrussModel,
      setTrussModel: props.setTrussModel,
      setThermalTruss3dModel: props.setThermalTruss3dModel,
      setTruss3dModel: props.setTruss3dModel,
      setPlaneModel: props.setPlaneModel,
      setFrameModel: props.setFrameModel,
      setBeamModel: props.setBeamModel,
      setTorsionModel: props.setTorsionModel,
      setSpringModel: props.setSpringModel,
      setSpring2dModel: props.setSpring2dModel,
      setSpring3dModel: props.setSpring3dModel,
      setParametric: props.setParametric,
      setPanelParametric: props.setPanelParametric,
      setActiveMaterial: props.setActiveMaterial,
      setLoadedModelName: props.setLoadedModelName,
      setSidebarSection: props.setSidebarSection,
      setSelectedNode: props.setSelectedNode,
      setSelectedElement: props.setSelectedElement,
      setMemberDraftNodes: props.setMemberDraftNodes,
    },
    props.resetActiveResult,
  );
}

export async function invokeWorkbenchScriptAction(options: Record<string, any>): Promise<Record<string, unknown>> {
  const actionDefinition = getWorkbenchScriptActionDefinition(options.action);
  const guardrail = evaluateWorkbenchUxActionGuardrail({
    action: options.action,
    definition: actionDefinition,
    summary: options.uxGuardrailSummary,
  });
  if (!guardrail.allowed) {
    const summary =
      options.language === "zh"
        ? `操作已被防呆保护拦截：${guardrail.reason}`
        : options.language === "ja"
          ? `操作はガードレールによりブロックされました: ${guardrail.reason}`
          : `Action blocked by UX guardrails: ${guardrail.reason}`;
    options.recordSecurityAuditEvent({
      action: options.action,
      source: options.source,
      risk: actionDefinition?.risk === "destructive" ? "destructive" : "sensitive",
      status: "failed",
      note: summary,
    });
    options.appendScriptActionLog({
      action: options.action,
      source: options.source,
      status: "failed",
      summary,
      payload: options.payload,
      note: guardrail.blockingItemId ?? undefined,
    });
    throw new Error(summary);
  }
  if (actionDefinition?.requiresConfirmation) {
    const auditRisk = actionDefinition.risk;
    options.recordSecurityAuditEvent({
      action: options.action,
      source: options.source,
      risk: auditRisk,
      status: "prompted",
      note:
        options.note ??
        (options.language === "zh"
          ? "等待操作员确认。"
          : options.language === "ja"
            ? "オペレーター確認待ちです。"
            : "Waiting for operator confirmation."),
    });
    const confirmationMessage =
      options.language === "zh"
        ? `动作 ${options.action} 属于高风险操作，可能修改、删除或导出敏感数据。\n\n请确认是否继续执行。`
        : options.language === "ja"
          ? `操作 ${options.action} は高リスクで、機微データの変更・削除・出力を行う可能性があります。\n\n実行を続けますか。`
          : `The action ${options.action} is high risk and may modify, delete, or export sensitive data.\n\nConfirm execution?`;
    if (typeof window !== "undefined" && !window.confirm(confirmationMessage)) {
      const summary =
        options.language === "zh"
          ? "已被操作员取消确认。"
          : options.language === "ja"
            ? "オペレーター確認で取り消されました。"
            : "Cancelled by operator confirmation.";
      options.recordSecurityAuditEvent({
        action: options.action,
        source: options.source,
        risk: auditRisk,
        status: "cancelled",
        note: summary,
      });
      options.appendScriptActionLog({
        action: options.action,
        source: options.source,
        status: "failed",
        summary,
        payload: options.payload,
        note: summary,
      });
      throw new Error(summary);
    }
  }

  options.appendScriptActionLog({
    action: options.action,
    source: options.source,
    status: "started",
    summary: JSON.stringify(options.payload),
    payload: options.payload,
    note: options.note,
  });

  try {
    let resultPayload: Record<string, unknown>;
    const navResult = await options.handleWorkbenchScriptNavAction(options.navArgs);
    if (navResult) {
      resultPayload = navResult;
    } else {
      const projectModelResult = await options.handleWorkbenchScriptProjectModelAction(options.projectModelArgs);
      if (projectModelResult) {
        resultPayload = projectModelResult;
      } else {
        const stateResult = await options.handleWorkbenchScriptStateAction(options.stateArgs);
        if (stateResult) {
          resultPayload = stateResult;
        } else {
          const macroDataResult = await options.handleWorkbenchScriptMacroDataAction({
            ...options.macroDataArgs,
            invokeScriptAction: (
              action: string,
              payload: Record<string, unknown> = {},
              source = "script",
              note?: string,
            ) =>
              invokeWorkbenchScriptAction({
                ...options,
                action,
                payload,
                source,
                note,
                navArgs: { ...options.navArgs, action, payload },
                projectModelArgs: { ...options.projectModelArgs, action, payload },
                stateArgs: { ...options.stateArgs, action, payload },
                macroDataArgs: { ...options.macroDataArgs, action, payload, source, note },
              }),
          });
          if (macroDataResult) {
            resultPayload = macroDataResult;
          } else {
            throw new Error(`Unknown script action: ${options.action}`);
          }
        }
      }
    }

    options.appendScriptActionLog({
      action: options.action,
      source: options.source,
      status: "completed",
      summary: JSON.stringify(resultPayload),
      payload: options.payload,
      result: resultPayload,
      note: options.note,
    });
    if (actionDefinition?.requiresConfirmation) {
      options.recordSecurityAuditEvent({
        action: options.action,
        source: options.source,
        risk: actionDefinition.risk,
        status: "completed",
        note:
          options.note ??
          (options.language === "zh"
            ? "高风险动作已执行完成。"
            : options.language === "ja"
              ? "高リスク操作の実行が完了しました。"
              : "High-risk action completed."),
      });
    }
    return resultPayload;
  } catch (error) {
    const summary = error instanceof Error ? error.message : String(error);
    if (actionDefinition?.requiresConfirmation) {
      options.recordSecurityAuditEvent({
        action: options.action,
        source: options.source,
        risk: actionDefinition.risk,
        status: "failed",
        note: summary,
      });
    }
    options.appendScriptActionLog({
      action: options.action,
      source: options.source,
      status: "failed",
      summary,
      payload: options.payload,
      note: summary,
    });
    throw error;
  }
}
