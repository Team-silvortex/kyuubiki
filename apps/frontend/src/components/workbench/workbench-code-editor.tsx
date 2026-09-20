"use client";

import { useEffect, useId, useLayoutEffect, useRef, useState, type KeyboardEvent } from "react";
import { breakWorkbenchCodeLine, indentWorkbenchCode, type WorkbenchCodeSelection } from "./workbench-code-edit";
import { getWorkbenchScriptWorkspaceCopy } from "./workbench-script-workspace-copy";

type Props = {
  value: string;
  onChange: (value: string) => void;
  language: string;
  syntax: "python" | "json";
  label: string;
  onRun: () => void;
  busy?: boolean;
};

export function WorkbenchCodeEditor({ value, onChange, language, syntax, label, onRun, busy = false }: Props) {
  const inputRef = useRef<HTMLTextAreaElement>(null);
  const hintId = useId();
  const pendingSelection = useRef<WorkbenchCodeSelection | null>(null);
  const history = useRef<WorkbenchCodeSelection[]>([{ value, start: 0, end: 0 }]);
  const historyIndex = useRef(0);
  const tabExit = useRef(false);
  const [viewport, setViewport] = useState({ scroll: 0, height: 280 });
  const copy = getWorkbenchScriptWorkspaceCopy(language);
  const lines = value.split("\n").length;
  const firstLine = Math.floor(viewport.scroll / 20);
  const gutterLines = Array.from({ length: Math.max(0, Math.min(lines - firstLine, Math.ceil(viewport.height / 20) + 1)) }, (_, i) => firstLine + i + 1);

  useEffect(() => {
    const input = inputRef.current;
    if (!input) return;
    const observer = new ResizeObserver(() => setViewport({ scroll: input.scrollTop, height: input.clientHeight }));
    observer.observe(input);
    return () => observer.disconnect();
  }, []);
  useLayoutEffect(() => {
    const next = pendingSelection.current;
    if (next && next.value === value) {
      inputRef.current?.setSelectionRange(next.start, next.end);
      pendingSelection.current = null;
    }
    if (history.current[historyIndex.current].value !== value) {
      // Catalog insertion and template replacement start a fresh local edit history.
      history.current = [{ value, start: 0, end: 0 }];
      historyIndex.current = 0;
    }
  }, [value]);

  const remember = (next: WorkbenchCodeSelection, restoreSelection = false) => {
    history.current = history.current.slice(0, historyIndex.current + 1);
    history.current.push(next);
    let size = history.current.reduce((total, item) => total + item.value.length, 0);
    while (history.current.length > 2 && (history.current.length > 100 || size > 1_000_000)) {
      size -= history.current.shift()!.value.length;
    }
    historyIndex.current = history.current.length - 1;
    if (restoreSelection) pendingSelection.current = next;
    onChange(next.value);
  };
  const currentSelection = (): WorkbenchCodeSelection => ({
    value, start: inputRef.current?.selectionStart ?? 0, end: inputRef.current?.selectionEnd ?? 0,
  });
  const keyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    if ((event.ctrlKey || event.metaKey) && event.key === "Enter") {
      event.preventDefault();
      if (!busy) onRun();
      return;
    }
    if ((event.ctrlKey || event.metaKey) && (event.key.toLowerCase() === "z" || event.key.toLowerCase() === "y")) {
      event.preventDefault();
      const direction = event.shiftKey || event.key.toLowerCase() === "y" ? 1 : -1;
      const index = Math.max(0, Math.min(history.current.length - 1, historyIndex.current + direction));
      historyIndex.current = index;
      const next = history.current[index];
      pendingSelection.current = next;
      onChange(next.value);
      inputRef.current?.setSelectionRange(next.start, next.end);
      return;
    }
    if (event.key === "Escape") { tabExit.current = true; return; }
    if (event.key === "Tab" && tabExit.current) { tabExit.current = false; return; }
    tabExit.current = false;
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key !== "Tab" && event.key !== "Enter") return;
    event.preventDefault();
    const selection = currentSelection();
    history.current[historyIndex.current] = selection;
    const next = event.key === "Tab" ? indentWorkbenchCode(selection, event.shiftKey)
      : breakWorkbenchCodeLine(selection, syntax === "python");
    if (next.value !== value) remember(next, true);
  };

  return (
    <div className="pwdt-code-editor" data-workbench-code-editor={syntax}>
      <div className="pwdt-code-editor__surface" dir="ltr">
        <div className="pwdt-code-editor__gutter" aria-hidden="true">
          <pre style={{ transform: `translateY(${12 - viewport.scroll % 20}px)` }}>{gutterLines.join("\n")}</pre>
        </div>
        <textarea ref={inputRef} className="script-panel__editor" aria-label={label} aria-describedby={hintId}
          aria-keyshortcuts="Control+Enter Meta+Enter" autoCapitalize="off" autoCorrect="off" spellCheck={false}
          wrap="off" value={value} onKeyDown={keyDown}
          onBeforeInput={() => { history.current[historyIndex.current] = currentSelection(); }}
          onBlur={() => { tabExit.current = false; }}
          onScroll={(event) => setViewport({ scroll: event.currentTarget.scrollTop, height: event.currentTarget.clientHeight })}
          onChange={(event) => remember({ value: event.target.value, start: event.target.selectionStart, end: event.target.selectionEnd })}
        />
      </div>
      <div className="pwdt-code-editor__footer"><code>{syntax === "python" ? "Python" : "JSON"}</code><span>{lines}</span></div>
      <p className="card-copy pwdt-code-editor__hint" id={hintId}>{copy.editorHint}</p>
    </div>
  );
}
