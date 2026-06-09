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
