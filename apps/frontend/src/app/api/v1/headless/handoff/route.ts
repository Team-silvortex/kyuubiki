import { NextResponse } from "next/server";

import {
  listHeadlessHandoffs,
  registerHeadlessHandoff,
} from "@/lib/scripting/workbench-headless-handoff-registry";
import { asHeadlessOrchestraHandoffEnvelope } from "@/lib/scripting/workbench-headless-orchestra-handoff";

export const runtime = "nodejs";

export async function GET() {
  return NextResponse.json({ handoffs: listHeadlessHandoffs() });
}

export async function POST(request: Request) {
  try {
    const payload = asHeadlessOrchestraHandoffEnvelope(await request.json().catch(() => null));
    if (!payload) {
      return NextResponse.json(
        { error: "invalid_handoff", message: "payload is not a valid kyuubiki.headless-orchestra-handoff/v1 envelope" },
        { status: 400 },
      );
    }

    return NextResponse.json(registerHeadlessHandoff(payload));
  } catch (error) {
    return NextResponse.json(
      { error: error instanceof Error ? error.message : "failed to receive headless handoff" },
      { status: 500 },
    );
  }
}
