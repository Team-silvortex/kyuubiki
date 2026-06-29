export function buildWorkbenchSystemBackendTargetCopy(language: string) {
  const zh = language === "zh";
  const ja = language === "ja";

  return {
    label: zh ? "后端服务目标" : ja ? "バックエンド接続先" : "Backend service target",
    help: zh
      ? "留空使用同源 /api；填写 http(s) 地址后，Workbench 会作为独立 GUI client 调用远端 orch、mesh gateway 或兼容服务。"
      : ja
        ? "空欄なら同一オリジン /api を使います。http(s) を指定すると独立 GUI client として接続します。"
        : "Leave empty for same-origin /api. Set an http(s) URL to point this GUI client at a remote orch, mesh gateway, or compatible service.",
    placeholder: "https://orch.example.local:4000",
    effective: zh ? "当前生效" : ja ? "有効な接続先" : "Effective",
    source: zh ? "来源" : ja ? "ソース" : "Source",
    saved: zh
      ? "后端服务目标已保存，后续请求会使用新目标。"
      : ja
        ? "接続先を保存しました。以後のリクエストで使用します。"
        : "Backend service target saved; future requests will use it.",
    cleared: zh
      ? "后端服务目标已清空，回到同源 /api。"
      : ja
        ? "接続先をクリアし、同一オリジン /api に戻しました。"
        : "Backend service target cleared; returning to same-origin /api.",
    sources: {
      query: zh ? "URL 参数" : ja ? "URL パラメータ" : "URL query",
      local_storage: zh ? "本地配置" : ja ? "ローカル設定" : "Local setting",
      settings: zh ? "工作台设置" : ja ? "Workbench 設定" : "Workbench settings",
      environment: zh ? "环境变量" : ja ? "環境変数" : "Environment",
      default: zh ? "同源默认" : ja ? "同一オリジン既定" : "Same-origin default",
    },
  };
}
