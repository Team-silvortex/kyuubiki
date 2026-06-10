"use client";

export type PayloadObject = Record<string, unknown>;

export type DraftStep = {
  id: string;
  action: string;
  payloadText: string;
};

export type HeadlessReferenceToken = {
  label: string;
  outputKey: string;
  template: string;
};

export type HeadlessInputPort = {
  key: string;
  bindable?: boolean;
  label: string;
  required?: boolean;
};

export type HeadlessOutputPort = {
  key: string;
  label: string;
};

export type HeadlessActionContract = {
  id: string;
  risk: "normal" | "sensitive" | "destructive";
  summary: Record<string, string>;
  payloadExample: PayloadObject;
  inputSchema: HeadlessInputPort[];
  outputSchema: HeadlessOutputPort[];
};

export type HeadlessWorkflowTemplate = {
  id: string;
  title: Record<string, string>;
  description: Record<string, string>;
  steps: Array<{ action: string; payload: PayloadObject }>;
};

export function formatPayload(payload: PayloadObject) {
  return JSON.stringify(payload, null, 2);
}

export function parsePayloadText(payloadText: string): PayloadObject | null {
  try {
    const parsed = JSON.parse(payloadText) as unknown;
    return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? (parsed as PayloadObject) : null;
  } catch {
    return null;
  }
}

export function updatePayloadField(payload: PayloadObject | null, key: string, value: unknown) {
  const next = { ...(payload ?? {}) };
  if (value === "" || value === undefined) {
    delete next[key];
  } else {
    next[key] = value;
  }
  return next;
}
