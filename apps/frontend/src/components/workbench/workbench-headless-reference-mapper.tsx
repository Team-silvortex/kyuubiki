"use client";

import { useMemo, useState } from "react";
import type { HeadlessReferenceToken } from "@/components/workbench/workbench-headless-workflow-contract";

type WorkbenchHeadlessReferenceMapperProps = {
  applyLabel: string;
  clearLabel: string;
  emptyLabel: string;
  field: string;
  helperText: string;
  references: HeadlessReferenceToken[];
  title: string;
  value: string;
  onApply: (template: string) => void;
  onClear: () => void;
};

export function WorkbenchHeadlessReferenceMapper({
  applyLabel,
  clearLabel,
  emptyLabel,
  field,
  helperText,
  references,
  title,
  value,
  onApply,
  onClear,
}: WorkbenchHeadlessReferenceMapperProps) {
  const options = useMemo(() => references.filter((reference) => reference.outputKey === field), [field, references]);
  const [selectedTemplate, setSelectedTemplate] = useState("");

  if (options.length === 0 && !value.includes("{{")) {
    return <p className="card-copy">{emptyLabel}</p>;
  }

  return (
    <>
      <p className="card-copy">{title}</p>
      {value.includes("{{") ? <p className="card-copy">{`${helperText} ${value}`}</p> : null}
      {options.length > 0 ? (
        <label className="field-label">
          <span>{field}</span>
          <select className="text-input" onChange={(event) => setSelectedTemplate(event.target.value)} value={selectedTemplate}>
            <option value="">{applyLabel}</option>
            {options.map((reference) => (
              <option key={`${field}-${reference.label}`} value={reference.template}>
                {`${reference.label} -> ${reference.outputKey}`}
              </option>
            ))}
          </select>
        </label>
      ) : null}
      <div className="button-row">
        <button className="ghost-button ghost-button--compact" disabled={!selectedTemplate} onClick={() => selectedTemplate && onApply(selectedTemplate)} type="button">
          {applyLabel}
        </button>
        <button className="ghost-button ghost-button--compact" disabled={!value.includes("{{")} onClick={onClear} type="button">
          {clearLabel}
        </button>
      </div>
    </>
  );
}
