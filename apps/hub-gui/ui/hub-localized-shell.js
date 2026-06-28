import { renderAssistantShellCopy } from "./hub-assistant-shell.js";
import { renderHubBundlesCopy } from "./hub-bundles-copy.js";
import { renderHubHomeCopy } from "./hub-home-copy.js";
import { renderHubLibraryCopy } from "./hub-library-copy.js";
import { renderHubPanelCopy } from "./hub-panel-copy.js";
import { renderHubWorkspaceGroups } from "./hub-workspace-groups.js";
import { renderDirectMeshRegressionSnapshot, renderGuidesPanelCopy, } from "./hub-guides-panel.js";
import { buildHubLanguageOptions } from "./hub-localization-panel.js";
export function createHubLocalizedShell(context) {
    function localizedHistoryFilterLabel(filter) {
        const copy = context.hubCopy();
        switch (filter) {
            case "all":
                return copy.bundles.all;
            case "failed":
                return copy.bundles.failed;
            case "inspect":
                return copy.bundles.inspect;
            case "normalize":
                return copy.bundles.normalize;
            case "diff":
                return copy.bundles.diff;
            default:
                return filter;
        }
    }
    function localizedWorkloadFilterLabel(filter) {
        const copy = context.hubCopy();
        switch (filter) {
            case "all":
                return copy.library.all;
            case "mechanical":
                return copy.library.mechanical;
            case "thermal":
                return copy.library.thermal;
            case "thermo_mechanical":
                return copy.library.thermo;
            default:
                return filter;
        }
    }
    function localizedWorkloadFamilyFilterLabel(filter) {
        const copy = context.hubCopy();
        switch (filter) {
            case "all":
                return copy.library.allFamilies;
            case "axial_and_springs":
                return copy.library.axial;
            case "beams_and_frames":
                return copy.library.beams;
            case "trusses":
                return copy.library.trusses;
            case "planes":
                return copy.library.planes;
            default:
                return filter;
        }
    }
    function localizedWorkflowCatalogLabel(key) {
        const copy = context.hubCopy();
        return copy.dynamic?.[key] || context.fallbackCopy.dynamic?.[key] || key;
    }
    function renderDesktopLanguagePreference() {
        const copy = context.hubCopy();
        document.documentElement.lang = context.state.language;
        context.setText(context.elements.languageLabel, copy.shell.language);
        if (context.elements.languageSelect) {
            const options = buildHubLanguageOptions();
            context.elements.languageSelect.replaceChildren(...options.map((option) => {
                const element = document.createElement("option");
                element.value = option.value;
                element.textContent = option.label;
                return element;
            }));
            context.elements.languageSelect.value = context.state.language;
        }
        context.setText(context.elements.actionStatusLabel, copy.shell.actionStatus);
        context.setText(context.elements.navProjects, copy.nav.projects);
        context.setText(context.elements.navRuntimes, copy.nav.runtimes);
        context.setText(context.elements.navDeploy, copy.nav.deploy);
        context.setText(context.elements.navObserve, copy.nav.observe);
        context.setText(context.elements.navTools, copy.nav.tools);
        renderHubWorkspaceGroups(copy);
        context.setText("brand-hub-focus", copy.shell.focus);
        context.setText(context.elements.heroOpenWorkbench, copy.shell.openWorkbench);
        context.setText(context.elements.heroStartLocal, copy.shell.startLocal);
        context.setText(context.elements.heroValidateEnv, copy.shell.validateEnv);
        renderSignalCopy(copy);
        if (!context.state.isBusy && context.elements.actionState) {
            context.elements.actionState.textContent = copy.shell.idle;
        }
        renderHubHomeCopy({ elements: context.elements, copy, setText: context.setText });
        renderHubLibraryCopy({
            elements: context.elements,
            copy,
            isBusy: context.state.isBusy,
            workflowCatalogBusy: context.state.workflowCatalogBusy,
            setText: context.setText,
        });
        renderHubBundlesCopy({
            elements: context.elements,
            copy,
            isBusy: context.state.isBusy,
            setText: context.setText,
        });
        renderGuidesPanelCopy({
            elements: context.elements,
            copy,
            activeLanguage: context.state.language,
            setText: context.setText,
        });
        renderDirectMeshRegressionSnapshot({
            elements: context.elements,
            snapshot: context.state.directMeshRegressionSnapshot,
            copy,
            regressionGateReport: context.state.regressionGateReport,
            applyDesktopState: context.applyDesktopState,
        });
        renderAssistantShellCopy({
            elements: context.elements,
            copy,
            isBusy: context.state.isBusy,
            setText: context.setText,
        });
        renderPanelLanguage(copy);
        const activeSectionCopy = copy.sections[context.state.activeSection];
        if (activeSectionCopy && context.elements.title && context.elements.copy) {
            context.elements.title.textContent = activeSectionCopy.title;
            context.elements.copy.textContent = activeSectionCopy.copy;
        }
    }
    function renderSignalCopy(copy) {
        context.setText(context.elements.signalIntakeLabel, copy.signals.intakeLabel);
        context.setText(context.elements.signalIntakeTitle, copy.signals.intakeTitle);
        context.setText(context.elements.signalIntakeCopy, copy.signals.intakeCopy);
        context.setText(context.elements.signalDomainsLabel, copy.signals.domainsLabel);
        context.setText(context.elements.signalDomainsTitle, copy.signals.domainsTitle);
        context.setText(context.elements.signalDomainsCopy, copy.signals.domainsCopy);
        context.setText(context.elements.signalFirstMoveLabel, copy.signals.firstMoveLabel);
        context.setText(context.elements.signalFirstMoveTitle, copy.signals.firstMoveTitle);
        context.setText(context.elements.signalFirstMoveCopy, copy.signals.firstMoveCopy);
    }
    function rerenderLocalizedHubShell() {
        renderDesktopLanguagePreference();
        context.renderHubRecents();
        context.renderWorkflowCatalog();
        context.renderHubAssistantAudit();
        context.renderAssistantContext();
        context.renderHubAssistantLocalCards();
        context.renderAssistantPanel();
    }
    function renderPanelLanguage(copy) {
        renderHubPanelCopy({
            elements: context.elements,
            copy,
            setText: context.setText,
            renderToolsPlatformLabel: context.renderToolsPlatformLabel,
        });
    }
    return {
        localizedHistoryFilterLabel,
        localizedWorkflowCatalogLabel,
        localizedWorkloadFamilyFilterLabel,
        localizedWorkloadFilterLabel,
        renderDesktopLanguagePreference,
        renderPanelLanguage,
        rerenderLocalizedHubShell,
    };
}
