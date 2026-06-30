"use client";

import { useEffect, useMemo, useState } from "react";
import type { DirectMeshSelectionMode, FrontendRuntimeMode } from "@/lib/api";
import {
  mergeLanguagePack,
  persistWorkbenchLanguagePacks,
  persistWorkbenchSettings,
  readWorkbenchLanguagePacks,
  safeStorageGet,
  type WorkbenchLanguagePack,
} from "@/lib/workbench/helpers";
import {
  copyByLanguage,
  resolveWorkbenchBaseCopy,
  type WorkbenchCopy,
} from "@/components/workbench/workbench-copy";
import type {
  AssistantMode,
  ImmersiveToolTab,
  Language,
  Theme,
} from "@/components/workbench/workbench-types";
import type { DirectMeshExecutionState } from "@/components/workbench/workbench-defaults";
import { buildWorkbenchLanguagePackCatalogRows } from "@/components/workbench/workbench-language-pack-catalog";

type UseWorkbenchShellStateArgs = {
  setLoadedModelName: (value: string | ((current: string) => string)) => void;
  setMessage: (value: string) => void;
};

export function useWorkbenchShellState({
  setLoadedModelName,
  setMessage,
}: UseWorkbenchShellStateArgs) {
  const [language, setLanguage] = useState<Language>("en");
  const [languagePacks, setLanguagePacks] = useState<WorkbenchLanguagePack[]>([]);
  const [theme, setTheme] = useState<Theme>("graphite");
  const [frontendRuntimeMode, setFrontendRuntimeMode] = useState<FrontendRuntimeMode>("orchestrated_gui");
  const [directMeshEndpointsText, setDirectMeshEndpointsText] = useState("127.0.0.1:5001,127.0.0.1:5002");
  const [directMeshSelectionMode, setDirectMeshSelectionMode] = useState<DirectMeshSelectionMode>("healthiest");
  const [controlPlaneApiToken, setControlPlaneApiToken] = useState("");
  const [clusterApiToken, setClusterApiToken] = useState("");
  const [directMeshApiToken, setDirectMeshApiToken] = useState("");
  const [assistantMode, setAssistantMode] = useState<AssistantMode>("local");
  const [assistantApiBaseUrl, setAssistantApiBaseUrl] = useState("https://api.openai.com/v1");
  const [assistantApiKey, setAssistantApiKey] = useState("");
  const [assistantModel, setAssistantModel] = useState("gpt-4.1-mini");
  const [assistantWindowOpen, setAssistantWindowOpen] = useState(false);
  const [directMeshExecution, setDirectMeshExecution] = useState<DirectMeshExecutionState | null>(null);
  const [showShortcutHints, setShowShortcutHints] = useState(true);
  const [immersiveGuardrails, setImmersiveGuardrails] = useState(true);
  const [immersiveViewport, setImmersiveViewport] = useState(false);
  const [immersiveToolDrawerOpen, setImmersiveToolDrawerOpen] = useState(false);
  const [immersiveHelpDrawerOpen, setImmersiveHelpDrawerOpen] = useState(false);
  const [immersiveToolTab, setImmersiveToolTab] = useState<ImmersiveToolTab>("node");
  const [truss3dProjectionMode, setTruss3dProjectionMode] = useState<"ortho" | "persp">("ortho");
  const [truss3dShowGrid, setTruss3dShowGrid] = useState(true);
  const [truss3dShowLabels, setTruss3dShowLabels] = useState(true);
  const [truss3dShowNodes, setTruss3dShowNodes] = useState(true);
  const [truss3dBoxSelectMode, setTruss3dBoxSelectMode] = useState(false);
  const [truss3dViewPreset, setTruss3dViewPreset] = useState<"iso" | "front" | "right" | "top">("iso");
  const [truss3dFocusRequestVersion, setTruss3dFocusRequestVersion] = useState(0);
  const [truss3dResetRequestVersion, setTruss3dResetRequestVersion] = useState(0);
  const [truss3dNudgeStep, setTruss3dNudgeStep] = useState(0.25);
  const [truss3dBatchLoadX, setTruss3dBatchLoadX] = useState(0);
  const [truss3dBatchLoadY, setTruss3dBatchLoadY] = useState(0);
  const [truss3dBatchLoadZ, setTruss3dBatchLoadZ] = useState(0);

  const applyLanguagePreference = (nextLanguage: Language) => {
    const nextCopy = resolveWorkbenchBaseCopy(nextLanguage);
    setLanguage(nextLanguage);
    setLoadedModelName((current) =>
      Object.values(copyByLanguage).some((copy) => copy.defaultModel === current)
        ? nextCopy.defaultModel
        : current,
    );
  };

  const activeLanguagePack = useMemo(
    () => languagePacks.find((pack) => pack.language === language) ?? null,
    [language, languagePacks],
  );
  const t = useMemo(
    () => mergeLanguagePack<WorkbenchCopy>(resolveWorkbenchBaseCopy(language), activeLanguagePack?.overrides ?? null),
    [activeLanguagePack?.overrides, language],
  );
  const languagePackCatalogRows = useMemo(
    () => buildWorkbenchLanguagePackCatalogRows(language),
    [language],
  );

  useEffect(() => {
    const stored = safeStorageGet();
    const desktopLanguage =
      typeof window !== "undefined"
        ? new URLSearchParams(window.location.search).get("desktopLanguage")
        : null;
    if (stored.theme === "linen" || stored.theme === "marine" || stored.theme === "graphite") {
      setTheme(stored.theme);
    }
    if (typeof stored.showShortcutHints === "boolean") setShowShortcutHints(stored.showShortcutHints);
    if (typeof stored.immersiveGuardrails === "boolean") setImmersiveGuardrails(stored.immersiveGuardrails);
    if (stored.frontendRuntimeMode) setFrontendRuntimeMode(stored.frontendRuntimeMode);
    if (stored.directMeshEndpointsText) setDirectMeshEndpointsText(stored.directMeshEndpointsText);
    if (stored.directMeshSelectionMode === "healthiest" || stored.directMeshSelectionMode === "first_reachable") {
      setDirectMeshSelectionMode(stored.directMeshSelectionMode);
    }
    if (stored.controlPlaneApiToken) setControlPlaneApiToken(stored.controlPlaneApiToken);
    if (stored.clusterApiToken) setClusterApiToken(stored.clusterApiToken);
    if (stored.directMeshApiToken) setDirectMeshApiToken(stored.directMeshApiToken);
    if (stored.assistantMode === "local" || stored.assistantMode === "llm") {
      setAssistantMode(stored.assistantMode);
    }
    if (typeof stored.assistantApiBaseUrl === "string" && stored.assistantApiBaseUrl.trim()) {
      setAssistantApiBaseUrl(stored.assistantApiBaseUrl);
    }
    if (typeof stored.assistantApiKey === "string") setAssistantApiKey(stored.assistantApiKey);
    if (typeof stored.assistantModel === "string" && stored.assistantModel.trim()) {
      setAssistantModel(stored.assistantModel);
    }
    const bootLanguage =
      typeof desktopLanguage === "string" && desktopLanguage.trim()
        ? desktopLanguage.trim()
        : typeof stored.language === "string" && stored.language.trim()
          ? stored.language.trim()
          : null;

    if (bootLanguage) {
      applyLanguagePreference(bootLanguage);
      setMessage(resolveWorkbenchBaseCopy(bootLanguage).initialLoaded);
    }
    setLanguagePacks(readWorkbenchLanguagePacks());
  }, [setLoadedModelName, setMessage]);

  useEffect(() => {
    if (typeof window === "undefined") return;

    const handleDesktopLanguage = (event: MessageEvent) => {
      if (event.data?.type !== "kyuubiki:set-language") return;
      const nextLanguage = event.data.language;
      if (typeof nextLanguage === "string" && nextLanguage.trim()) {
        applyLanguagePreference(nextLanguage.trim());
      }
    };

    window.addEventListener("message", handleDesktopLanguage);
    return () => {
      window.removeEventListener("message", handleDesktopLanguage);
    };
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    if (typeof window !== "undefined") {
      window.parent?.postMessage({ type: "kyuubiki:language-changed", language }, "*");
      persistWorkbenchSettings({
        theme,
        language,
        showShortcutHints,
        immersiveGuardrails,
        frontendRuntimeMode,
        directMeshEndpointsText,
        directMeshSelectionMode,
        controlPlaneApiToken,
        clusterApiToken,
        directMeshApiToken,
        assistantMode,
        assistantApiBaseUrl,
        assistantApiKey,
        assistantModel,
      });
    }
  }, [
    assistantApiBaseUrl,
    assistantApiKey,
    assistantMode,
    assistantModel,
    clusterApiToken,
    controlPlaneApiToken,
    directMeshApiToken,
    directMeshEndpointsText,
    directMeshSelectionMode,
    frontendRuntimeMode,
    immersiveGuardrails,
    language,
    showShortcutHints,
    theme,
  ]);

  useEffect(() => {
    persistWorkbenchLanguagePacks(languagePacks);
  }, [languagePacks]);

  return {
    language,
    setLanguage,
    languagePacks,
    setLanguagePacks,
    theme,
    setTheme,
    frontendRuntimeMode,
    setFrontendRuntimeMode,
    directMeshEndpointsText,
    setDirectMeshEndpointsText,
    directMeshSelectionMode,
    setDirectMeshSelectionMode,
    controlPlaneApiToken,
    setControlPlaneApiToken,
    clusterApiToken,
    setClusterApiToken,
    directMeshApiToken,
    setDirectMeshApiToken,
    assistantMode,
    setAssistantMode,
    assistantApiBaseUrl,
    setAssistantApiBaseUrl,
    assistantApiKey,
    setAssistantApiKey,
    assistantModel,
    setAssistantModel,
    assistantWindowOpen,
    setAssistantWindowOpen,
    directMeshExecution,
    setDirectMeshExecution,
    showShortcutHints,
    setShowShortcutHints,
    immersiveGuardrails,
    setImmersiveGuardrails,
    immersiveViewport,
    setImmersiveViewport,
    immersiveToolDrawerOpen,
    setImmersiveToolDrawerOpen,
    immersiveHelpDrawerOpen,
    setImmersiveHelpDrawerOpen,
    immersiveToolTab,
    setImmersiveToolTab,
    truss3dProjectionMode,
    setTruss3dProjectionMode,
    truss3dShowGrid,
    setTruss3dShowGrid,
    truss3dShowLabels,
    setTruss3dShowLabels,
    truss3dShowNodes,
    setTruss3dShowNodes,
    truss3dBoxSelectMode,
    setTruss3dBoxSelectMode,
    truss3dViewPreset,
    setTruss3dViewPreset,
    truss3dFocusRequestVersion,
    setTruss3dFocusRequestVersion,
    truss3dResetRequestVersion,
    setTruss3dResetRequestVersion,
    truss3dNudgeStep,
    setTruss3dNudgeStep,
    truss3dBatchLoadX,
    setTruss3dBatchLoadX,
    truss3dBatchLoadY,
    setTruss3dBatchLoadY,
    truss3dBatchLoadZ,
    setTruss3dBatchLoadZ,
    applyLanguagePreference,
    activeLanguagePack,
    t,
    languagePackCatalogRows,
  };
}
