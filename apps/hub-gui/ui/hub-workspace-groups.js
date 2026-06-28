const GROUPS = [
    { id: "project", targets: ["projects"] },
    { id: "runtime", targets: ["runtimes"] },
    { id: "release", targets: ["deploy"] },
    { id: "diagnostics", targets: ["observe", "tools"] },
];
export function renderHubWorkspaceGroups(copy) {
    const nav = document.querySelector(".hub-nav");
    if (!nav || nav.dataset.grouped === "true") {
        updateHubWorkspaceGroups(copy);
        return;
    }
    const buttons = new Map(Array.from(nav.querySelectorAll("[data-target]")).map((button) => [
        button.dataset.target,
        button,
    ]));
    nav.replaceChildren();
    nav.classList.add("hub-nav--grouped");
    for (const group of GROUPS) {
        const section = document.createElement("section");
        section.className = "hub-nav-group";
        section.dataset.workspaceGroup = group.id;
        const label = document.createElement("div");
        label.className = "hub-nav-group__label";
        label.id = `nav-group-${group.id}-label`;
        const copyNode = document.createElement("div");
        copyNode.className = "hub-nav-group__copy desktop-shell-note";
        copyNode.id = `nav-group-${group.id}-copy`;
        const body = document.createElement("div");
        body.className = "hub-nav-group__items";
        for (const target of group.targets) {
            const button = buttons.get(target);
            if (button) {
                body.append(button);
            }
        }
        section.append(label, copyNode, body);
        nav.append(section);
    }
    nav.dataset.grouped = "true";
    updateHubWorkspaceGroups(copy);
}
function updateHubWorkspaceGroups(copy) {
    const groups = copy.workspaceGroups || {};
    for (const group of GROUPS) {
        const label = document.getElementById(`nav-group-${group.id}-label`);
        const copyNode = document.getElementById(`nav-group-${group.id}-copy`);
        if (label) {
            label.textContent = groups[group.id]?.label || group.id;
        }
        if (copyNode) {
            copyNode.textContent = groups[group.id]?.copy || "";
        }
    }
}
