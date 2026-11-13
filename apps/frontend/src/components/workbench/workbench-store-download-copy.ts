const en = {
  tab: "Authorized downloads", account: "Connect account", title: "Download a verified release", refresh: "Refresh status", selection: "Select a release", working: "Checking with the native store service…",
  intro: "Choose an exact release from your signed-in account's store. Previewing does not use quota; authorization may use one download credit.",
  desktop: "Open this page in desktop Workbench to use the native account and downloader.", login: "Connect your shared account first. Downloads are bound to that account and issuer.",
  source: "Account and download source", kind: "Asset type", asset: "Asset ID", version: "Exact version", platform: "Target platform", choose: "Choose a platform",
  kinds: ["Operator", "Workflow template", "Frontend DSL template", "Language pack"],
  trust: "Release signing key", key: "Key ID", fingerprint: "SHA-256 of signing public key (SPKI DER)",
  trustHint: "Obtain these independently from the publisher. This is not the TLS CA fingerprint or login key. Never paste a private key or token.",
  prepare: "Preview release and quota", preview: "Check source, version and platform before authorizing.", files: "Files", size: "Size", quota: "Remaining at preview/authorization", unlimited: "Unlimited",
  confirm: "Authorize and download (may use 1 credit)", free: "Authorize and download (no credit charge)", retry: "Retry original authorization", cancel: "Cancel download",
  remove: "Remove local download", removeConfirm: "Remove only this local download and its record? Credits are not refunded; downloading it again may use another credit.",
  history: "Local download queue", empty: "No downloads for this account yet.", cap: "Up to four local records, 256 MiB each. Remove completed downloads when no longer needed. Unresolved authorizations are retained to prevent duplicate charges.",
  noInstall: "Not installed or loaded. Transport signature, size and digest are checked. For a supported operator package, use the native installation button below for separate compatibility checks.",
  charged: "Original authorization used one credit", notCharged: "Original authorization used no credit", unknown: "Authorization outcome not confirmed",
  recovery: "Retries reuse the same authorization; cancellation does not refund quota. Closing the app interrupts transfers. After reconnecting, recovery depends on the service's original-session policy. In-progress network requests may take up to 120 seconds to stop.",
  states: { prepared: "Ready for confirmation", authorizing: "Checking authorization", downloading: "Downloading", downloaded: "Verified download; not installed", failed: "Download failed", cancelled: "Cancelled", cancelling: "Cancelling", interrupted: "Unfinished; check or recover" } as Record<string, string>,
  errors: { login: "The session is unavailable or changed. Reconnect from the account page, then refresh this queue.", quota: "No download credits remain. Check your account entitlements.", trust: "Release verification failed. Check the independently supplied signing key and ask the publisher to verify the release.",
    blocked: "The service has not enabled this download, permission or metering policy. No automatic fallback is used.", recovery: "Authorization is unresolved or its safe recovery window expired. Keep this record and ask the service to check it; do not create a new authorization.",
    busy: "Another download operation is active. Wait or cancel it, then refresh.", unavailable: "This authorization expired or the release was recalled. Ask the service to check before creating another.",
    installer: "Native installation was not confirmed. Check this device's platform, the package manifest and any existing installed version. If Installer is unavailable, update or repair the companion runtime. Do not authorize another download.",
    generic: "The operation did not complete. Refresh status before retrying the original record; no success has been assumed." },
};
type Copy = typeof en;
const zh: Copy = {
  tab: "授权下载", account: "连接账号", title: "下载已核验的发布版本", refresh: "刷新状态", selection: "选择发布版本", working: "正在通过原生商店服务核对，请稍候…",
  intro: "从已登录账号对应的商店选择确切版本。预检不消耗额度；确认授权时可能消耗一次下载额度。",
  desktop: "请在桌面 Workbench 中打开此页面，使用原生账号会话和下载器。", login: "请先连接统一账号。下载记录与该账号和来源绑定。",
  source: "账号及下载来源", kind: "资源类型", asset: "资源 ID", version: "确切版本", platform: "目标平台", choose: "请选择平台",
  kinds: ["算子", "工作流模板", "前端 DSL 模板", "语言包"], trust: "发布签名公钥", key: "密钥 ID", fingerprint: "签名公钥 SHA-256（SPKI DER）",
  trustHint: "请通过独立可信渠道向发布方获取。它不是 HTTPS 的 CA 指纹，也不是登录公钥。不要粘贴私钥或令牌。",
  prepare: "预检版本和额度", preview: "确认来源、版本和平台后再授权。", files: "文件数", size: "大小", quota: "预检/授权时余量", unlimited: "不限额度",
  confirm: "授权并下载（可能消耗 1 次额度）", free: "授权并下载（不扣额度）", retry: "沿用原授权重试", cancel: "取消下载", remove: "移除本地下载",
  removeConfirm: "仅移除此本地下载及记录？这不会退还额度；以后重新下载可能再次计次。",
  history: "本地下载队列", empty: "此账号暂时没有下载记录。", cap: "最多保留 4 条本地记录，每条上限 256 MiB。不再需要的已完成下载可以移除；结果未确认的授权保留记录，避免重复扣次。",
  noInstall: "文件尚未安装或加载。这里只验证传输签名、长度和摘要；支持的算子包可通过下方原生安装按钮继续检查兼容性。",
  charged: "原授权已消耗 1 次额度", notCharged: "原授权未扣额度", unknown: "授权计次结果尚未确认",
  recovery: "重试沿用原授权，取消不退额度。退出应用会中断传输；重新登录后的恢复受服务端原会话策略约束。进行中的网络请求最多需要 120 秒才会停止。",
  states: { prepared: "待确认授权", authorizing: "核对授权中", downloading: "下载中", downloaded: "下载已校验，尚未安装", failed: "下载失败", cancelled: "已取消", cancelling: "正在取消", interrupted: "未完成，需核对或恢复" },
  errors: { login: "账号会话已失效或发生变化。请前往账号页重新连接，再刷新队列。", quota: "下载额度不足，请检查账号权益。", trust: "发布版本校验未通过。请核对独立获取的签名公钥，并让发布方检查文件。",
    blocked: "服务尚未开放此下载、授权或计次策略；不会自动绕过限制。", recovery: "授权结果尚未确认，或安全恢复期限已过。请保留记录，让服务端核对，不要重新创建授权。",
    busy: "已有下载操作进行中，请等待或取消后刷新。", unavailable: "授权已失效，或发布版本已被召回。请先让服务端核对，不要直接重新授权。",
    installer: "原生安装尚未确认。请检查本机平台、包清单及已安装版本；若 Installer 不可用，请更新或修复配套运行时。不要重新创建下载授权。",
    generic: "操作未完成。请先刷新状态，再沿用原记录重试；不会将未确认的结果当作成功。" },
};
const ja: Copy = {
  tab: "認証済みダウンロード", account: "アカウント接続", title: "検証済みリリースのダウンロード", refresh: "状態を更新", selection: "リリースを選択", working: "ネイティブのストアサービスで確認中…",
  intro: "接続中のストアから正確なバージョンを選択します。事前確認は無料ですが、認可時に枠を1回使用する場合があります。",
  desktop: "デスクトップ Workbench で開くとネイティブのアカウントとダウンローダーを使用できます。", login: "先に共通アカウントに接続してください。記録はアカウントと発行元に紐づきます。",
  source: "アカウントと配信元", kind: "種類", asset: "アセット ID", version: "正確なバージョン", platform: "対象プラットフォーム", choose: "プラットフォームを選択",
  kinds: ["オペレーター", "ワークフローテンプレート", "フロント DSL テンプレート", "言語パック"], trust: "リリース署名公開鍵", key: "鍵 ID", fingerprint: "署名公開鍵の SHA-256（SPKI DER）",
  trustHint: "独立した信頼できる経路で公開者から取得してください。TLS CA の指紋やログイン鍵とは異なります。秘密鍵やトークンは入力しないでください。",
  prepare: "リリースと残り枠を確認", preview: "配信元、バージョン、対象を確認してから認可してください。", files: "ファイル数", size: "サイズ", quota: "確認・認可時の残り枠", unlimited: "無制限",
  confirm: "認可してダウンロード（1回分を使う場合あり）", free: "認可してダウンロード（枠の消費なし）", retry: "元の認可で再試行", cancel: "ダウンロードを中止", remove: "ローカルデータを削除",
  removeConfirm: "このローカルダウンロードと記録のみを削除しますか？枠は返却されません。再ダウンロード時に枠を使う場合があります。",
  history: "ローカルダウンロード一覧", empty: "このアカウントのダウンロードはまだありません。", cap: "最大4件、各256 MiBまでです。不要な完了済みデータを削除できます。結果不明の認可は二重計上を防ぐため保持します。",
  noInstall: "未インストール・未ロードです。配信署名、サイズ、ハッシュのみ検証済みです。対応パッケージは下のインストールボタンで互換性を別途確認します。",
  charged: "元の認可で1回分を使用", notCharged: "元の認可は枠の消費なし", unknown: "認可の計上結果が未確認",
  recovery: "再試行は元の認可を使用し、中止しても枠は戻りません。終了で転送は中断します。再ログイン後の復旧はサービス側の元セッション方針に従います。通信の停止には最大120秒かかります。",
  states: { prepared: "認可の確認待ち", authorizing: "認可を確認中", downloading: "ダウンロード中", downloaded: "検証済み・未インストール", failed: "失敗", cancelled: "中止済み", cancelling: "中止処理中", interrupted: "未完了・確認または復旧が必要" },
  errors: { login: "セッションが失効または変更されました。再接続して一覧を更新してください。", quota: "ダウンロード枠が不足しています。権限を確認してください。", trust: "検証に失敗しました。署名鍵を確認し、公開者に確認を依頼してください。",
    blocked: "このダウンロード、権限または計上方針は未有効です。制限は回避しません。", recovery: "認可結果が不明、または復旧期限を過ぎています。記録を残してサービス側に確認し、新しい認可は作らないでください。",
    busy: "別の処理が実行中です。待機または中止後に状態を更新してください。", unavailable: "認可が失効、またはリリースが撤回されています。再認可前にサービス側へ確認してください。",
    installer: "インストールを確認できません。本機の対象、マニフェスト、既存バージョンを確認してください。Installer がない場合はランタイムを更新・修復してください。再ダウンロードの認可は不要です。",
    generic: "処理が完了していません。状態を更新してから元の記録で再試行してください。成功とは扱いません。" },
};
export const storeDownloadCopy = (language: string): Copy => language.startsWith("zh") ? zh : language.startsWith("ja") ? ja : en;
export function downloadError(copy: Copy, code: string) {
  if (["login_required", "session_changed", "download_owner_mismatch"].includes(code)) return copy.errors.login;
  if (code === "quota_exhausted") return copy.errors.quota;
  if (/signing|signature|artifact_mismatch|unsafe_download_url|grant_mismatch/.test(code)) return copy.errors.trust;
  if (/downloads_unavailable|metering|permission_denied|store_identity_mismatch/.test(code)) return copy.errors.blocked;
  if (/authorization_outcome|authorization_recovery/.test(code)) return copy.errors.recovery;
  if (code === "download_busy") return copy.errors.busy;
  if (/grant_expired|grant_revoked|release_recalled|release_withdrawn/.test(code)) return copy.errors.unavailable;
  if (/install|store_platform|store_package|store_manifest|store_binary|operator_download_required/.test(code)) return copy.errors.installer;
  return copy.errors.generic;
}
