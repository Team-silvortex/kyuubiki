export function renderAssistantShellCopy(params) {
  const { elements, copy, isBusy, setText } = params;

  setText(elements.assistantIntroLabel, copy.assistant.introLabel);
  setText(elements.assistantIntroTitle, copy.assistant.introTitle);
  setText(elements.assistantIntroCopy, copy.assistant.introCopy);
  setText(elements.assistantClose, copy.assistant.close);
  setText(elements.assistantEngineLabel, copy.assistant.engine);
  setText(elements.assistantContextSectionLabel, copy.assistant.section);
  setText(elements.assistantContextRuntimeLabel, copy.assistant.runtime);
  setText(elements.assistantContextBundleLabel, copy.assistant.bundle);
  setText(elements.assistantLocalActionsLabel, copy.assistant.quickActions);
  setText(elements.assistantLocalActionStart, copy.assistant.quickStart);
  setText(elements.assistantLocalActionLibrary, copy.assistant.quickLibrary);
  setText(elements.assistantLocalActionBundles, copy.assistant.quickBundles);
  setText(elements.assistantLocalActionGuides, copy.assistant.quickGuides);
  setText(elements.assistantLocalAskLabel, copy.assistant.ask);
  setText(elements.assistantLocalPromptLabel, copy.assistant.askLabel);
  setText(elements.assistantLocalAsk, copy.assistant.askButton);
  if (elements.assistantLocalOutput && !isBusy) {
    elements.assistantLocalOutput.textContent = copy.assistant.askEmpty;
  }
  setText(elements.assistantDocsLabel, copy.assistant.docs);
  setText(elements.assistantDocsIndexTitle, copy.assistant.docsIndexTitle);
  setText(elements.assistantDocsIndexCopy, copy.assistant.docsIndexCopy);
  setText(elements.assistantDocsCurrentTitle, copy.assistant.docsCurrentTitle);
  setText(elements.assistantDocsCurrentCopy, copy.assistant.docsCurrentCopy);
  setText(elements.assistantDocsOperationsTitle, copy.assistant.docsOperationsTitle);
  setText(elements.assistantDocsOperationsCopy, copy.assistant.docsOperationsCopy);
  setText(elements.assistantDocsTroubleshootingTitle, copy.assistant.docsTroubleshootingTitle);
  setText(elements.assistantDocsTroubleshootingCopy, copy.assistant.docsTroubleshootingCopy);
  setText(elements.assistantSuggestedLabel, copy.assistant.suggested);
  setText(elements.assistantLlmIntroCopy, copy.assistant.llmIntro);
  setText(elements.assistantBaseUrlLabel, copy.assistant.baseUrl);
  setText(elements.assistantApiKeyLabel, copy.assistant.apiKey);
  setText(elements.assistantPresetLabel, copy.assistant.preset);
  setText(elements.assistantModelLabel, copy.assistant.model);
  setText(elements.assistantRequestLabel, copy.assistant.request);
  setText(elements.assistantRequestPlan, copy.assistant.generate);
  setText(elements.assistantApproveLabel, copy.assistant.approve);
  setText(elements.assistantExecutePlan, copy.assistant.execute);
  setText(elements.assistantEndpointPolicy, copy.dynamic.endpointPolicyDefault);
  if (elements.assistantOutput && !isBusy) {
    elements.assistantOutput.textContent = copy.assistant.ready;
  }
  setText(elements.assistantAuditLabel, copy.assistant.audit);
}
