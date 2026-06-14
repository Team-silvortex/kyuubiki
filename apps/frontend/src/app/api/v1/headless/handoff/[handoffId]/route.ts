import { NextResponse } from "next/server";

import { getHeadlessHandoffStatus } from "@/lib/scripting/workbench-headless-handoff-registry";

export const runtime = "nodejs";

export async function GET(_request: Request, context: { params: Promise<{ handoffId: string }> }) {
  const { handoffId } = await context.params;
  const payload = getHeadlessHandoffStatus(handoffId);
  if (!payload) {
    return NextResponse.json(
      { error: "handoff_not_found", message: `handoff '${handoffId}' is not registered in the current runtime` },
      { status: 404 },
    );
  }
  return NextResponse.json(payload);
}
