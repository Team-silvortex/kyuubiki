export const HUB_GUIDES_I18N = {
    en: {
        guides: {
            primaryLabel: "Primary docs",
            primaryTitle: "Open the right guide",
            primaryCopy: "Start with the docs index, then branch into only the guide that matches the job you are doing now.",
            docsTitle: "Docs index",
            docsCopy: "The single entry to current-line, operations, testing, accuracy, and archived release notes.",
            currentTitle: "Current line",
            overviewDocsLabel: "Docs hub",
            overviewDocsTitle: "One readable shelf",
            overviewDocsCopy: "Use one place for orientation first, then branch into operations, accuracy, or troubleshooting only when needed.",
            overviewCurrentLabel: "Current line",
            overviewTroubleshootingLabel: "Troubleshooting",
            operationsTitle: "Operations",
            troubleshootingTitle: "Troubleshooting",
            accuracyLabel: "Accuracy and confidence",
        },
        assistant: {
            docsIndexTitle: "Docs index",
            docsIndexCopy: "Open the full documentation entry point.",
            docsCurrentTitle: "Current line",
            docsOperationsTitle: "Operations",
            docsTroubleshootingTitle: "Troubleshooting",
        },
    },
    zh: {
        guides: {
            primaryLabel: "主文档",
            primaryTitle: "打开正确的指南",
            primaryCopy: "先从 docs index 开始，再只进入和当前工作匹配的那份指南。",
            docsTitle: "文档索引",
            docsCopy: "current-line、operations、testing、accuracy 和历史 release notes 的统一入口。",
            currentTitle: "当前版本线",
            overviewDocsLabel: "文档中枢",
            overviewDocsTitle: "一个清晰的文档架",
            overviewDocsCopy: "先在一个地方完成定向，再按需要进入 operations、accuracy 或 troubleshooting。",
            overviewCurrentLabel: "当前版本线",
            overviewTroubleshootingLabel: "故障排查",
            operationsTitle: "Operations",
            troubleshootingTitle: "故障排查",
            accuracyLabel: "精度与可信度",
        },
        assistant: {
            docsIndexTitle: "文档索引",
            docsIndexCopy: "打开完整的文档总入口。",
            docsCurrentTitle: "当前版本线",
            docsOperationsTitle: "Operations",
            docsTroubleshootingTitle: "故障排查",
        },
    },
    ja: {
        guides: {
            primaryLabel: "主要ドキュメント",
            primaryTitle: "正しいガイドを開く",
            primaryCopy: "まず docs index から入り、今の作業に合うガイドだけに進みます。",
            docsTitle: "Docs index",
            docsCopy: "current-line、operations、testing、accuracy、archive をまとめた入口です。",
            currentTitle: "Current line",
            overviewDocsLabel: "Docs hub",
            overviewDocsTitle: "読みやすい一つの棚",
            overviewDocsCopy: "まず一か所で向きを合わせ、その後必要に応じて operations、accuracy、troubleshooting に分岐します。",
            overviewCurrentLabel: "Current line",
            overviewTroubleshootingLabel: "Troubleshooting",
            operationsTitle: "Operations",
            troubleshootingTitle: "Troubleshooting",
            accuracyLabel: "精度と信頼性",
        },
        assistant: {
            docsIndexTitle: "Docs index",
            docsIndexCopy: "完全なドキュメント入口を開きます。",
            docsCurrentTitle: "Current line",
            docsOperationsTitle: "Operations",
            docsTroubleshootingTitle: "Troubleshooting",
        },
    },
    es: {
        guides: {
            primaryLabel: "Biblioteca principal",
            primaryTitle: "Centro de documentación",
            primaryCopy: "Usa una sola estantería clara para la línea actual, operaciones, troubleshooting y exactitud.",
            docsTitle: "Índice de docs",
            docsCopy: "Abre el índice principal de documentación.",
            currentTitle: "Línea actual",
        },
        assistant: {
            docsIndexTitle: "Índice de docs",
            docsIndexCopy: "Abre la entrada completa de documentación.",
            docsCurrentTitle: "Línea actual",
            docsOperationsTitle: "Operaciones",
            docsTroubleshootingTitle: "Troubleshooting",
        },
    },
};
export function applyHubGuidesI18n(target) {
    for (const [language, overrides] of Object.entries(HUB_GUIDES_I18N)) {
        if (!target[language])
            continue;
        target[language] = {
            ...target[language],
            guides: {
                ...target[language].guides,
                ...overrides.guides,
            },
            assistant: {
                ...target[language].assistant,
                ...overrides.assistant,
            },
        };
    }
}
