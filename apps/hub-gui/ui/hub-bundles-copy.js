export function renderHubBundlesCopy(params) {
    const { elements, copy, isBusy, setText } = params;
    setText(elements.bundlesIntroLabel, copy.bundles.introLabel);
    setText(elements.bundlesIntroTitle, copy.bundles.introTitle);
    setText(elements.bundlesIntroCopy, copy.bundles.introCopy);
    setText(elements.bundlesBundlePathLabel, copy.bundles.bundlePath);
    setText(elements.bundlesComparePathLabel, copy.bundles.comparePath);
    setText(elements.bundlesOutputPathLabel, copy.bundles.outputPath);
    setText(elements.bundlesActionInspect, copy.bundles.inspect);
    setText(elements.bundlesActionValidate, copy.bundles.validate);
    setText(elements.bundlesActionNormalize, copy.bundles.normalize);
    setText(elements.bundlesActionUnpack, copy.bundles.unpack);
    setText(elements.bundlesActionPack, copy.bundles.pack);
    setText(elements.bundlesActionDiff, copy.bundles.diff);
    setText(elements.bundlesActionOpenWorkbench, copy.bundles.openWorkbench);
    setText(elements.bundlesActionDesktopTools, copy.bundles.desktopTools);
    setText(elements.bundlesRecentBundlesLabel, copy.bundles.recentBundles);
    setText(elements.bundlesRecentCompareLabel, copy.bundles.recentCompare);
    setText(elements.bundlesRecentOutputsLabel, copy.bundles.recentOutputs);
    setText(elements.bundlesRecentActionsLabel, copy.bundles.recentActions);
    setText(elements.bundlesHistoryAll, copy.bundles.all);
    setText(elements.bundlesHistoryFailed, copy.bundles.failed);
    setText(elements.bundlesHistoryInspect, copy.bundles.inspect);
    setText(elements.bundlesHistoryNormalize, copy.bundles.normalize);
    setText(elements.bundlesHistoryDiff, copy.bundles.diff);
    setText(elements.bundlesHistoryKeepFailed, copy.bundles.keepFailed);
    setText(elements.bundlesHistoryImport, copy.bundles.import);
    setText(elements.bundlesHistoryExport, copy.bundles.export);
    setText(elements.bundlesHistoryClear, copy.bundles.clear);
    setText(elements.bundlesFavoritesLabel, copy.bundles.favorites);
    setText(elements.bundlesRecentLabel, copy.bundles.recent);
    if (elements.projectBundleOutput && !isBusy) {
        elements.projectBundleOutput.textContent = copy.bundles.ready;
    }
}
