export function operatorVariantCopy(language: string) {
  if (language.startsWith("zh")) return {
    platform: "执行平台", local: "本机平台算子库", refresh: "核验当前平台的包",
    hint: "Linux 变体只在本机暂存，不在控制端加载。项目启用记录按平台隔离；部署前还会核对目标 Agent 的实际平台。",
    stage: "暂存 Linux 部署包", confirm: "将此确切 Linux 变体加入独立部署缓存？不会在本机加载、部署到 Agent 或执行，不再消耗下载额度。已有不同内容不会覆盖。",
    variant_staging: "暂存结果待核对", variant_staged: "Linux 部署包已暂存，未部署", variant_stage_failed: "暂存未确认，请核验或重试",
    receipt: "暂存回执不是 Agent 就绪证明。下一步：项目启用 → 指定 Agent → 部署 → 受管重启后核验。移除下载缓存不会删除独立的部署包。",
    root: "此平台的本机部署缓存（不是远程 Agent 的路径）",
    mismatch: "目标 Agent 平台与所选变体不匹配，请重新选择；没有继续传输或执行。",
    unsupported: "本机安装只接收本机 Rust 算子包；Linux x86_64 / arm64 包可独立暂存后部署。没有匹配变体时不会改用其他平台。",
  };
  if (language.startsWith("ja")) return {
    platform: "実行プラットフォーム", local: "本機用ライブラリ", refresh: "選択したプラットフォームを検証",
    hint: "Linux 版は配備用に保存するだけで制御端末では読み込みません。有効化記録はプラットフォーム別です。転送前に Agent の実際のプラットフォームを検証します。",
    stage: "Linux 配備パッケージを保存", confirm: "この Linux 版を独立した配備キャッシュに保存しますか？本機での読込、Agent 配備、実行は行わず、枠も再消費しません。異なる内容は上書きしません。",
    variant_staging: "保存結果の確認が必要", variant_staged: "Linux 配備パッケージ保存済み・未配備", variant_stage_failed: "保存未確認・確認または再試行",
    receipt: "保存記録は Agent の準備完了を意味しません。有効化 → Agent 指定 → 配備 → 管理された再起動後に検証。ダウンロード削除では配備キャッシュは削除されません。",
    root: "本機上の配備キャッシュ（リモート Agent のパスではありません）",
    mismatch: "Agent のプラットフォームが選択と一致しません。転送・実行を中止しました。選び直してください。",
    unsupported: "本機向け Rust パッケージのみ本機にインストールできます。Linux x86_64 / arm64 は別途保存して配備できます。別プラットフォームへの自動切替はありません。",
  };
  return {
    platform: "Execution platform", local: "This device's operator library", refresh: "Verify selected platform packages",
    hint: "Linux variants are staged, never loaded on the control device. Enablement is isolated by platform; deployment also checks the Agent's actual platform.",
    stage: "Stage Linux deployment package", confirm: "Stage this exact Linux variant in an isolated deployment cache? No local loading, Agent deployment or execution, no extra download credit, and no replacement of different contents.",
    variant_staging: "Staging outcome needs checking", variant_staged: "Linux package staged, not deployed", variant_stage_failed: "Staging unconfirmed; verify or retry",
    receipt: "A staging receipt is not Agent readiness. Next: enable in project → choose Agent → deploy → managed restart and verify. Removing the download does not remove the separate deployment cache.",
    root: "Platform deployment cache on this device (not a remote Agent path)",
    mismatch: "Agent platform does not match the selected variant. Select the correct platform; transfer or execution was not continued.",
    unsupported: "Local installation accepts this device's Rust packages only. Linux x86_64 / arm64 packages can be staged separately for deployment. No fallback to a different platform.",
  };
}
