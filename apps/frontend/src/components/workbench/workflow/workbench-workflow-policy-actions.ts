"use client";

import { useEffect } from "react";

export type WorkflowPolicyAction =
  | "preview-validation-fixes"
  | "scan-package-residuals"
  | "preview-package-repairs";
export type WorkflowPolicyActionStatus = "complete" | "ready";

type WorkflowPolicyActionRequest = {
  action: WorkflowPolicyAction;
  token: number;
};

type WorkflowPolicyActionFeedback = {
  action: WorkflowPolicyAction;
  status: WorkflowPolicyActionStatus;
  detail: string;
  token: number;
};

let pendingRequest: WorkflowPolicyActionRequest | null = null;
let pendingFeedback: WorkflowPolicyActionFeedback | null = null;
const listeners = new Set<(request: WorkflowPolicyActionRequest) => void>();
const feedbackListeners = new Set<(feedback: WorkflowPolicyActionFeedback) => void>();

export function requestWorkflowPolicyAction(action: WorkflowPolicyAction) {
  const request = { action, token: Date.now() };
  pendingRequest = request;
  listeners.forEach((listener) => listener(request));
}

export function publishWorkflowPolicyActionFeedback(
  action: WorkflowPolicyAction,
  status: WorkflowPolicyActionStatus,
  detail: string,
) {
  const feedback = { action, status, detail, token: Date.now() };
  pendingFeedback = feedback;
  feedbackListeners.forEach((listener) => listener(feedback));
}

function subscribeWorkflowPolicyAction(listener: (request: WorkflowPolicyActionRequest) => void) {
  listeners.add(listener);
  if (pendingRequest) {
    listener(pendingRequest);
  }
  return () => {
    listeners.delete(listener);
  };
}

function subscribeWorkflowPolicyFeedback(listener: (feedback: WorkflowPolicyActionFeedback) => void) {
  feedbackListeners.add(listener);
  if (pendingFeedback) {
    listener(pendingFeedback);
  }
  return () => {
    feedbackListeners.delete(listener);
  };
}

export function useWorkflowPolicyAction(handler: (action: WorkflowPolicyAction) => void) {
  useEffect(
    () =>
      subscribeWorkflowPolicyAction((request) => {
        handler(request.action);
        if (pendingRequest?.token === request.token) {
          pendingRequest = null;
        }
      }),
    [handler],
  );
}

export function useWorkflowPolicyActionFeedback(
  handler: (feedback: { action: WorkflowPolicyAction; status: WorkflowPolicyActionStatus; detail: string }) => void,
) {
  useEffect(
    () =>
      subscribeWorkflowPolicyFeedback((feedback) => {
        handler(feedback);
        if (pendingFeedback?.token === feedback.token) {
          pendingFeedback = null;
        }
      }),
    [handler],
  );
}
