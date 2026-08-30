"use client";

import type { ReactNode } from "react";

export type WorkbenchRouteJourneyStep = {
  id: string;
  title: string;
  hint?: ReactNode;
  status?: ReactNode;
  automation?: Record<string, string>;
  onOpen: () => void;
};

type WorkbenchRouteJourneyProps = {
  steps: WorkbenchRouteJourneyStep[];
};

export function WorkbenchRouteJourney({ steps }: WorkbenchRouteJourneyProps) {
  return (
    <ol className="workbench-route-journey" data-workbench-route-journey="true">
      {steps.map((step, index) => (
        <li key={step.id}>
          <button
            {...step.automation}
            className="workbench-route-step"
            onClick={step.onOpen}
            type="button"
          >
            <span aria-hidden="true" className="workbench-route-step__index">
              {index + 1}
            </span>
            <span className="workbench-route-step__copy">
              <strong>{step.title}</strong>
              {step.hint ? <small>{step.hint}</small> : null}
            </span>
            {step.status ? <span className="workbench-route-step__status">{step.status}</span> : null}
            <span aria-hidden="true" className="workbench-route-step__arrow">&gt;</span>
          </button>
        </li>
      ))}
    </ol>
  );
}
