function versionMinorLine(version) {
  const parts = String(version).split(".");
  if (parts.length < 2) {
    return `${version}.x`;
  }

  return `${parts[0]}.${parts[1]}.x`;
}

function versionDisplay(codename, version) {
  return `${codename} ${version}`;
}

function textIncludesCheck(file, field, expected, reader) {
  return {
    kind: "text_includes",
    file,
    field,
    expected,
    actual: reader(file).includes(expected) ? expected : null,
  };
}

export function markdownFactChecks(expectedVersion, codename, reader) {
  const expectedMinorLine = versionMinorLine(expectedVersion);
  const expectedDisplayVersion = versionDisplay(codename, expectedVersion);
  const expectedDisplayMinorLine = versionDisplay(codename, expectedMinorLine);
  return [
    textIncludesCheck(
      "docs/version-line.md",
      "current shipping version",
      `current shipping version: \`${expectedDisplayVersion}\``,
      reader,
    ),
    textIncludesCheck(
      "docs/version-line.md",
      "current documentation target",
      `current documentation target: \`${expectedDisplayMinorLine}\` active line`,
      reader,
    ),
    textIncludesCheck(
      "docs/current-line.md",
      "published release snapshot",
      `current published release snapshot in this line is \`${expectedDisplayVersion}\``,
      reader,
    ),
    textIncludesCheck(
      "docs/installer-remote-control.md",
      "preparation line",
      `\`${expectedDisplayMinorLine}\` preparation line`,
      reader,
    ),
    textIncludesCheck(
      "docs/desktop-release-checklist.md",
      "workspace-prep line",
      `current \`${expectedVersion}\` workspace-prep line`,
      reader,
    ),
  ];
}
