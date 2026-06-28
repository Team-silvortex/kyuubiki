export function renderHubLibraryCopy(params) {
    const { elements, copy, isBusy, workflowCatalogBusy, setText } = params;
    setText(elements.libraryIntroLabel, copy.library.introLabel);
    setText(elements.libraryIntroTitle, copy.library.introTitle);
    setText(elements.libraryIntroCopy, copy.library.introCopy);
    setText(elements.libraryCatalogUrlLabel, copy.library.catalogUrl);
    setText(elements.libraryLabelNoteLabel, copy.library.labelOrNote);
    setText(elements.libraryActionRegister, copy.library.register);
    setText(elements.libraryActionSyncLocal, copy.library.syncLocal);
    setText(elements.libraryActionSyncRemote, copy.library.syncRemote);
    setText(elements.libraryActionExport, copy.library.export);
    setText(elements.libraryActionImport, copy.library.import);
    setText(elements.libraryActionClear, copy.library.clear);
    setText(elements.libraryManagedWorkloadsLabel, copy.library.managedWorkloads);
    setText(elements.librarySearchLabel, copy.library.workloadSearchLabel);
    if (elements.workloadLibrarySearch) {
        elements.workloadLibrarySearch.placeholder = copy.library.workloadSearchPlaceholder;
    }
    setText(elements.librarySearchClear, copy.library.workloadSearchClear);
    setText(elements.libraryFilterAll, copy.library.all);
    setText(elements.libraryFilterMechanical, copy.library.mechanical);
    setText(elements.libraryFilterThermal, copy.library.thermal);
    setText(elements.libraryFilterThermo, copy.library.thermo);
    setText(elements.libraryFamilyAll, copy.library.allFamilies);
    setText(elements.libraryFamilyAxial, copy.library.axial);
    setText(elements.libraryFamilyBeams, copy.library.beams);
    setText(elements.libraryFamilyTrusses, copy.library.trusses);
    setText(elements.libraryFamilyPlanes, copy.library.planes);
    setText(elements.workflowCatalogLabel, copy.library.workflowCatalogLabel);
    setText(elements.workflowCatalogTitle, copy.library.workflowCatalogTitle);
    setText(elements.workflowCatalogCopy, copy.library.workflowCatalogCopy);
    setText(elements.workflowCatalogSearchLabel, copy.library.workflowCatalogSearchLabel);
    if (elements.workflowCatalogSearch) {
        elements.workflowCatalogSearch.placeholder = copy.library.workflowCatalogSearchPlaceholder;
    }
    setText(elements.workflowCatalogSearchClear, copy.library.workflowCatalogSearchClear);
    setText(elements.workflowCatalogRefresh, copy.library.workflowCatalogRefresh);
    if (elements.workflowCatalogOutput && !workflowCatalogBusy) {
        elements.workflowCatalogOutput.textContent = copy.library.workflowCatalogReady;
    }
    if (elements.workloadLibraryOutput && !isBusy) {
        elements.workloadLibraryOutput.textContent = copy.library.ready;
    }
}
