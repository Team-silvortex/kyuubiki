export function hubMessage(template, replacements = {}) {
  return Object.entries(replacements).reduce(
    (value, [key, replacement]) => value.replaceAll(`{${key}}`, String(replacement)),
    String(template ?? ""),
  );
}
