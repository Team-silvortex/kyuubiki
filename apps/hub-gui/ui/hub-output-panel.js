export function createHubOutputPanel({ elements, formatRuntimeReport }) {
  function setOperationOutput(value) {
    elements.operationOutput.textContent = value;
  }

  function setDesktopStatusOutput(value) {
    if (elements.desktopStatusOutput) {
      elements.desktopStatusOutput.textContent = formatRuntimeReport(value);
    }
  }

  function setRuntimeStatusOutput(value) {
    elements.runtimeStatusOutput.textContent = formatRuntimeReport(value);
    if (elements.observeRuntimeStatusOutput) {
      elements.observeRuntimeStatusOutput.textContent = formatRuntimeReport(value);
    }
  }

  function setHotRuntimeStatusOutput(value) {
    if (elements.hotRuntimeStatusOutput) {
      elements.hotRuntimeStatusOutput.textContent = formatRuntimeReport(value);
    }
    if (elements.observeRuntimeStatusOutput) {
      elements.observeRuntimeStatusOutput.textContent = formatRuntimeReport(value);
    }
  }

  function setHotRuntimeLogOutput(value) {
    if (elements.hotRuntimeLogOutput) {
      elements.hotRuntimeLogOutput.textContent = value;
    }
    if (elements.observeHotLogOutput) {
      elements.observeHotLogOutput.textContent = value;
    }
  }

  function setObserveRuntimeLogOutput(value) {
    if (elements.observeRuntimeLogOutput) {
      elements.observeRuntimeLogOutput.textContent = value;
    }
  }

  return {
    setDesktopStatusOutput,
    setHotRuntimeLogOutput,
    setHotRuntimeStatusOutput,
    setObserveRuntimeLogOutput,
    setOperationOutput,
    setRuntimeStatusOutput,
  };
}
