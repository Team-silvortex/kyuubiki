export function appendTextElement(parent, tagName, text, className) {
  const element = document.createElement(tagName);
  if (className) {
    element.className = className;
  }
  element.textContent = text;
  parent.appendChild(element);
  return element;
}

export function appendAssistantCardHeader(parent, title, badgeText, badgeClassName) {
  const header = document.createElement("div");
  header.className = "desktop-shell-section-header";
  appendTextElement(header, "strong", title);
  appendTextElement(header, "span", badgeText, badgeClassName);
  parent.appendChild(header);
  return header;
}
