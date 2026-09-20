export type WorkbenchCodeSelection = { value: string; start: number; end: number };

export function indentWorkbenchCode(
  selection: WorkbenchCodeSelection, outdent = false, width = 4,
): WorkbenchCodeSelection {
  const { value, start, end } = selection;
  const lineStart = value.slice(0, start).lastIndexOf("\n") + 1;
  if (!outdent && start === end) {
    const spaces = " ".repeat(width - (start - lineStart) % width);
    return { value: value.slice(0, start) + spaces + value.slice(end), start: start + spaces.length, end: end + spaces.length };
  }
  // A selection ending at the next line's start does not include that line.
  const last = end > start && value[end - 1] === "\n" ? end - 1 : end;
  const nextBreak = value.indexOf("\n", last);
  const lineEnd = nextBreak < 0 ? value.length : nextBreak;
  const lines = value.slice(lineStart, lineEnd).split("\n");
  let firstDelta = 0, totalDelta = 0;
  const replacement = lines.map((line, index) => {
    const removed = outdent ? (line.startsWith("\t") ? 1 : Math.min(line.match(/^ */u)![0].length, width)) : 0;
    const delta = outdent ? -removed : width;
    if (!index) firstDelta = delta;
    totalDelta += delta;
    return outdent ? line.slice(removed) : " ".repeat(width) + line;
  }).join("\n");
  return {
    value: value.slice(0, lineStart) + replacement + value.slice(lineEnd),
    start: Math.max(lineStart, start + firstDelta),
    end: Math.max(lineStart, end + totalDelta),
  };
}

export function breakWorkbenchCodeLine(selection: WorkbenchCodeSelection, python: boolean): WorkbenchCodeSelection {
  const { value, start, end } = selection;
  const prefix = value.slice(value.slice(0, start).lastIndexOf("\n") + 1, start);
  const indent = prefix.match(/^\s*/u)?.[0] ?? "";
  const inserted = `\n${indent}${python && prefix.trimEnd().endsWith(":") ? "    " : ""}`;
  return { value: value.slice(0, start) + inserted + value.slice(end), start: start + inserted.length, end: start + inserted.length };
}
