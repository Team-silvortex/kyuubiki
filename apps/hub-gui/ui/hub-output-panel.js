export function createHubOutputPanel({ elements, formatRuntimeReport }) {
  function setEventMessage(message, meta = "event") {
    if (elements.eventMessage) {
      elements.eventMessage.textContent = message;
    }
    if (elements.eventMeta) {
      elements.eventMeta.textContent = meta;
    }
  }

  function setOperationOutput(value) {
    elements.operationOutput.textContent = value;
    setEventMessage(String(value || "operation updated").split("\n")[0], "operation");
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
    setEventMessage,
    setHotRuntimeLogOutput,
    setHotRuntimeStatusOutput,
    setObserveRuntimeLogOutput,
    setOperationOutput,
    setRuntimeStatusOutput,
  };
}
