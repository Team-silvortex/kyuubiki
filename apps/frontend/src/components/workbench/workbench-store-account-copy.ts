const en = {
  tab: "Account", title: "Team Silvortex account", badge: "Shared account",
  intro: "Register on the shared account website, then return to your research. Local research and self-hosted catalogs do not require a central store account.",
  origin: "Account website", placeholder: "https://your-account-host",
  originHint: "Use the HTTPS origin supplied by your administrator, including a port if needed. This is separate from the catalog source.",
  invalid: "Enter an HTTPS origin only, with no username, password, path, query or fragment.",
  save: "Save website", clear: "Remove website", saved: "Account website saved on this device.",
  storage: "The website preference could not be read or saved. Check local storage and try again.",
  open: "Register / account website", opening: "Opening browser…", destination: "Destination",
  steps: [
    ["Open the account website", "Check the destination, then choose registration or sign-in on the account page."],
    ["Complete account verification", "Enter credentials only on the account website. Follow its email verification and two-factor prompts."],
    ["Return to Workbench", "Your research stays open. Website registration does not automatically connect a Workbench session."],
  ],
  next: "Complete registration or sign-in in your browser, then return here. No Workbench login result has been received.",
  failed: "The system browser could not be opened. Copy the destination below into your browser, or update the desktop app and retry.",
  certificate: "If a test site reports an untrusted certificate, obtain its public CA from the administrator. Do not bypass the warning.",
  back: "Back to catalog",
};

const zh: typeof en = {
  tab: "账号", title: "Team Silvortex 账号", badge: "统一账号",
  intro: "在统一账号网站完成注册，再返回研究工作。使用本地研究功能和自部署目录不需要中心商店账号。",
  origin: "账号网站", placeholder: "https://你的账号站点",
  originHint: "填写管理员提供的 HTTPS 站点地址，需要时包含端口。这与商店目录源是两项独立配置。",
  invalid: "请只填写 HTTPS 站点地址，不包含用户名、密码、路径、查询参数或片段。",
  save: "保存站点", clear: "移除站点", saved: "账号站点已保存到本机。",
  storage: "无法读取或保存站点设置，请检查本地存储后重试。",
  open: "注册 / 账号网站", opening: "正在打开浏览器…", destination: "即将访问",
  steps: [
    ["打开账号网站", "核对目标地址，在账号页面选择注册或登录。"],
    ["完成账号验证", "只在账号网站填写凭证，按提示完成邮箱验证和双重认证。"],
    ["返回工作台", "研究工作保持打开。网站注册成功不会自动建立工作台登录会话。"],
  ],
  next: "请在浏览器中完成注册或登录，然后返回此页。工作台尚未收到登录结果。",
  failed: "无法打开系统浏览器。可以把下方目标地址复制到浏览器，或更新桌面应用后重试。",
  certificate: "测试站点若提示证书不受信任，请向管理员获取公开 CA 证书，不要绕过警告。",
  back: "返回商店目录",
};

const ja: typeof en = {
  tab: "アカウント", title: "Team Silvortex アカウント", badge: "共通アカウント",
  intro: "共通アカウントサイトで登録してから研究に戻ります。ローカル研究とセルフホストのカタログに中央ストアのアカウントは不要です。",
  origin: "アカウントサイト", placeholder: "https://your-account-host",
  originHint: "管理者から提供された HTTPS のオリジンを入力してください。必要ならポートも指定します。カタログソースとは別の設定です。",
  invalid: "HTTPS のオリジンのみを入力してください。認証情報、パス、クエリ、フラグメントは使用できません。",
  save: "サイトを保存", clear: "サイトを削除", saved: "アカウントサイトをこの端末に保存しました。",
  storage: "サイト設定を読み込みまたは保存できません。ローカルストレージを確認して再試行してください。",
  open: "登録 / アカウントサイト", opening: "ブラウザーを開いています…", destination: "アクセス先",
  steps: [
    ["アカウントサイトを開く", "アクセス先を確認し、アカウントページで登録またはログインを選びます。"],
    ["アカウントを確認する", "認証情報はアカウントサイトだけに入力し、メール確認と二要素認証の案内に従います。"],
    ["Workbench に戻る", "研究は開いたままです。サイトで登録しても Workbench のログイン状態にはなりません。"],
  ],
  next: "ブラウザーで登録またはログインを完了してから戻ってください。Workbench はまだログイン結果を受信していません。",
  failed: "システムブラウザーを開けませんでした。下の URL をブラウザーに貼り付けるか、デスクトップアプリを更新して再試行してください。",
  certificate: "テストサイトの証明書が信頼されない場合、管理者から公開 CA 証明書を入手してください。警告を回避しないでください。",
  back: "カタログに戻る",
};

export const storeAccountCopy = (language: string) => language.startsWith("zh") ? zh : language === "ja" ? ja : en;
