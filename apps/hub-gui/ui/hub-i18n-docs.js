export const HUB_DOCS_I18N = {
    en: {
        guides: {
            currentCopy: "Read what moxi 2.x is optimizing for before you make deeper product decisions.",
            overviewCurrentTitle: "moxi 2.x",
            overviewCurrentCopy: "Read the current product posture, version line, and what this generation is trying to harden.",
        },
        assistant: {
            docsCurrentCopy: "Read the current moxi 2.x posture.",
        },
        dynamic: {
            endpointPolicyDefault: "Use https:// for remote providers, or http://localhost / 127.0.0.1 for local gateways. The API key is sent directly to the configured base URL.",
        },
    },
    zh: {
        guides: {
            currentCopy: "先读 moxi 2.x 现在到底在强化什么，再做更深的产品判断。",
            overviewCurrentTitle: "moxi 2.x",
            overviewCurrentCopy: "先读当前产品姿态、版本线和这一代在重点加固什么。",
        },
        assistant: {
            docsCurrentCopy: "查看 moxi 2.x 当前的产品姿态。",
        },
        dynamic: {
            endpointPolicyDefault: "远程提供方请使用 https://，本地网关可使用 http://localhost / 127.0.0.1。API key 会直接发送到配置的 Base URL。",
        },
    },
    ja: {
        guides: {
            currentCopy: "より深い判断の前に、moxi 2.x が何を強化しているかを確認します。",
            overviewCurrentTitle: "moxi 2.x",
            overviewCurrentCopy: "現在のプロダクト姿勢、version line、この世代が何を硬くしているかを確認します。",
        },
        assistant: {
            docsCurrentCopy: "moxi 2.x の現在の姿勢を読みます。",
        },
        dynamic: {
            endpointPolicyDefault: "リモートプロバイダーには https:// を使い、ローカルゲートウェイには http://localhost / 127.0.0.1 を使ってください。API key は設定された Base URL に直接送信されます。",
        },
    },
    es: {
        guides: {
            currentCopy: "Lee la postura actual de moxi 2.x.",
            overviewCurrentTitle: "moxi 2.x",
            overviewCurrentCopy: "Lee la postura actual del producto, la línea de versión y lo que esta generación está intentando endurecer.",
        },
        assistant: {
            docsCurrentCopy: "Lee la postura actual de moxi 2.x.",
        },
        dynamic: {
            endpointPolicyDefault: "Usa https:// para proveedores remotos, o http://localhost / 127.0.0.1 para pasarelas locales. La API key se envía directamente a la URL base configurada.",
        },
    },
};
export function applyHubDocsI18n(target) {
    for (const [language, overrides] of Object.entries(HUB_DOCS_I18N)) {
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
            dynamic: {
                ...target[language].dynamic,
                ...overrides.dynamic,
            },
        };
    }
}
