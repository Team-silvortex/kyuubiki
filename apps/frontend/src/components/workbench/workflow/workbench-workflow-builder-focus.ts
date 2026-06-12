"use client";

import { useEffect, useState, type RefObject } from "react";

export type WorkflowBuilderFocusTarget = "validation" | "package-policy" | "snapshots";

type WorkflowBuilderFocusRequest = {
  target: WorkflowBuilderFocusTarget;
  token: number;
};

let pendingRequest: WorkflowBuilderFocusRequest | null = null;
const listeners = new Set<(request: WorkflowBuilderFocusRequest) => void>();

const focusTargetSelectors: Record<WorkflowBuilderFocusTarget, string> = {
  validation: '[data-workflow-validation-card="card"]',
  "package-policy": '[data-workflow-package-policy-card="card"]',
  snapshots: '[data-workflow-snapshot-card="card"]',
};

const WORKFLOW_BUILDER_FOCUS_CLASS = "workflow-builder-focus-target";

export function requestWorkflowBuilderFocus(target: WorkflowBuilderFocusTarget) {
  const request = { target, token: Date.now() };
  pendingRequest = request;
  listeners.forEach((listener) => listener(request));
}

function subscribeWorkflowBuilderFocus(listener: (request: WorkflowBuilderFocusRequest) => void) {
  listeners.add(listener);
  if (pendingRequest) {
    listener(pendingRequest);
  }
  return () => {
    listeners.delete(listener);
  };
}

export function useWorkflowBuilderFocus(builderRootRef: RefObject<HTMLElement | null>) {
  const [request, setRequest] = useState<WorkflowBuilderFocusRequest | null>(null);
  const [activeTarget, setActiveTarget] = useState<WorkflowBuilderFocusTarget | null>(null);

  useEffect(() => subscribeWorkflowBuilderFocus(setRequest), []);

  useEffect(() => {
    if (!request) return;
    queueMicrotask(() => {
      const target = builderRootRef.current?.querySelector<HTMLElement>(focusTargetSelectors[request.target]);
      if (target) {
        setActiveTarget(request.target);
        target.classList.remove(WORKFLOW_BUILDER_FOCUS_CLASS);
        void target.offsetWidth;
        target.classList.add(WORKFLOW_BUILDER_FOCUS_CLASS);
        target.scrollIntoView({ block: "nearest", behavior: "smooth" });
        window.setTimeout(() => {
          target.classList.remove(WORKFLOW_BUILDER_FOCUS_CLASS);
          setActiveTarget((current) => (current === request.target ? null : current));
        }, 1800);
      }
      if (pendingRequest?.token === request.token) {
        pendingRequest = null;
      }
    });
  }, [builderRootRef, request]);

  return activeTarget;
}
