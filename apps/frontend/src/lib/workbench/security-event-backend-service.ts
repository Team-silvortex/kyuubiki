"use client";

import {
  createSecurityEvent,
  fetchSecurityEvents,
} from "@/lib/api/security-results-client";
import {
  createSecurityEventBackendService,
  type WorkbenchSecurityEventBackendService,
  type WorkbenchSecurityEventBackendTransport,
  type WorkbenchSecurityEventFilters,
  type WorkbenchSecurityEventInput,
} from "@/lib/workbench/security-event-backend-service-core";

export {
  createSecurityEventBackendService,
  type WorkbenchSecurityEventBackendService,
  type WorkbenchSecurityEventBackendTransport,
  type WorkbenchSecurityEventFilters,
  type WorkbenchSecurityEventInput,
};

export const workbenchSecurityEventBackendService = createSecurityEventBackendService({
  createEvent: createSecurityEvent,
  fetchEvents: fetchSecurityEvents,
});
