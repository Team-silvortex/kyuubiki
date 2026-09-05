"use client";

import { defaultWorkbenchRuntimeBackedBackendServices } from "@/lib/workbench/backend-service-composer";
import {
  createWorkbenchStoreBackendService,
  type WorkbenchStoreBackendService,
  type WorkbenchStoreBackendTransport,
  type WorkbenchStoreCatalogQuery,
} from "@/lib/workbench/store-backend-service-core";

export {
  createWorkbenchStoreBackendService,
  type WorkbenchStoreBackendService,
  type WorkbenchStoreBackendTransport,
  type WorkbenchStoreCatalogQuery,
};

export const workbenchStoreBackendService =
  defaultWorkbenchRuntimeBackedBackendServices.store;
