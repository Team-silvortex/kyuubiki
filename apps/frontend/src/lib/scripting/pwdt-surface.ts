export type PwdtHostSurface = "workbench" | "hub" | "installer";

export type PwdtSurfaceStatus = "implemented" | "launcher" | "planned-restricted";

export type PwdtSurfaceDescriptor = {
  surface: PwdtHostSurface;
  status: PwdtSurfaceStatus;
  canRunPyodide: boolean;
  canEditDsl: boolean;
  canRecordMacros: boolean;
  canInvokeFrontendActions: boolean;
  role: string;
};

export const PWDT_PRODUCT_NAME = "Pwdt";
export const PWDT_LONG_NAME = "Python WASM DSL Tooling";

export const PWDT_SURFACE_DESCRIPTORS: Record<PwdtHostSurface, PwdtSurfaceDescriptor> = {
  workbench: {
    surface: "workbench",
    status: "implemented",
    canRunPyodide: true,
    canEditDsl: true,
    canRecordMacros: true,
    canInvokeFrontendActions: true,
    role: "Full frontend automation console for the fixed Workbench UI contract.",
  },
  hub: {
    surface: "hub",
    status: "launcher",
    canRunPyodide: false,
    canEditDsl: false,
    canRecordMacros: false,
    canInvokeFrontendActions: false,
    role: "Launcher and project-history stub surface; opens Workbench for full Pwdt runs.",
  },
  installer: {
    surface: "installer",
    status: "planned-restricted",
    canRunPyodide: false,
    canEditDsl: false,
    canRecordMacros: false,
    canInvokeFrontendActions: false,
    role: "Future restricted diagnostics console for installer-safe actions only.",
  },
};

export function getPwdtSurfaceDescriptor(surface: PwdtHostSurface): PwdtSurfaceDescriptor {
  return PWDT_SURFACE_DESCRIPTORS[surface];
}

export function isPwdtFullConsole(surface: PwdtHostSurface): boolean {
  const descriptor = getPwdtSurfaceDescriptor(surface);
  return descriptor.canRunPyodide && descriptor.canEditDsl && descriptor.canInvokeFrontendActions;
}
