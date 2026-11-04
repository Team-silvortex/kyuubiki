import { AssistantConnectionError, type AssistantConnectionCode } from "@/lib/assistant/model-connection";

const en = {
  connectionSection:"Connection", modelSection:"Model", presetLocal:"Local API", sessionOnly:"Key stays in memory",
  editConnection:"Edit connection", hideConnection:"Collapse connection",
  catalogMode:"Browse models", manualMode:"Enter ID", search:"Filter models", searchHint:"Search by model ID",
  count:"Showing {shown} of {matched}", noMatches:"No matching models. Change the search or enter an ID.",
  current:"Current model", notSelected:"Not selected", idle:"Not checked", connected:"List loaded", failed:"Check failed",
  title:"Model API connection", protocol:"Chat Completions compatible", base:"API base URL", key:"Model API key", model:"Model ID", models:"Available models",
  test:"Test connection / load models", loading:"Connecting...", cancel:"Cancel", clear:"Clear key", show:"Show key", hide:"Hide key", normalize:"Use normalized URL",
  trust:"I trust this service to receive my model API key. Generating a plan also sends the workbench context.",
  hint:"This is your model provider key, not an Agent or Orchestra token. Keys stay in page memory; re-enter them after reload. Changing the API address clears the key.",
  modelHint:"Choose a returned model or enter its exact ID. An empty list or unsupported /models endpoint does not prevent manual configuration.",
  scope:"Connection testing only reads the model list: no study data, model inference or task execution. It does not prove that the selected model can generate a plan.",
  ready:"Connection accepted. Select a model or keep your manually entered ID.", empty:"Connection accepted, but the model list is empty. Enter a model ID manually.",
  native:"Transport: local Rust gateway (no browser CORS dependency)", browser:"Transport: browser direct (provider must allow this browser origin)",
  help:"Privacy and compatibility", scopeShort:"Test reads the model list only; no inference or study data.",
  select:"Select a model", useModel:"Use this model / write request", changed:"Address changed; the previous API key was cleared.",
};
const zh: typeof en = {
  connectionSection:"连接服务", modelSection:"选择模型", presetLocal:"本地接口", sessionOnly:"密钥仅留在内存",
  editConnection:"编辑连接", hideConnection:"收起连接设置",
  catalogMode:"浏览模型", manualMode:"手动填写", search:"筛选模型", searchHint:"按模型 ID 搜索",
  count:"显示 {shown} / {matched} 项", noMatches:"没有匹配的模型，请修改搜索或手动填写 ID。",
  current:"当前模型", notSelected:"尚未选择", idle:"尚未检查", connected:"列表已读取", failed:"检查失败",
  title:"模型 API 连接", protocol:"Chat Completions 兼容接口", base:"API 基础地址", key:"模型 API 密钥", model:"模型 ID", models:"可用模型",
  test:"测试连接 / 加载模型", loading:"连接中…", cancel:"取消", clear:"清除密钥", show:"显示密钥", hide:"隐藏密钥", normalize:"使用规范化地址",
  trust:"我信任此服务接收模型 API 密钥。生成计划时还会向它发送工作台上下文。",
  hint:"这里填写模型服务商的密钥，不是 Agent 或 Orchestra token。密钥只保留在页面内存，刷新后需重新填写；切换 API 地址会清除旧密钥。",
  modelHint:"可以选择返回的模型，也可以手填准确 ID。列表为空或服务不支持 /models，不妨碍手动配置。",
  scope:"连接测试只读取模型列表，不发送研究数据、不调用模型推理、不执行任务；不代表所选模型一定能生成计划。",
  ready:"连接已被服务接受。请选择模型，或保留手动填写的 ID。", empty:"连接已被服务接受，但模型列表为空。请手动填写模型 ID。",
  native:"传输：本机 Rust 网关（不依赖浏览器跨域许可）", browser:"传输：浏览器直连（服务商需允许当前页面跨域访问）",
  help:"隐私与兼容性说明", scopeShort:"测试只读取模型列表，不推理、不发送研究数据。",
  select:"选择模型", useModel:"使用此模型 / 填写请求", changed:"地址已改变，旧 API 密钥已清除。",
};
const ja: typeof en = {
  connectionSection:"サービス接続", modelSection:"モデル選択", presetLocal:"ローカル API", sessionOnly:"キーはメモリのみ",
  editConnection:"接続を編集", hideConnection:"接続設定を閉じる",
  catalogMode:"一覧から選ぶ", manualMode:"ID を入力", search:"モデルを絞り込む", searchHint:"モデル ID で検索",
  count:"{matched} 件中 {shown} 件を表示", noMatches:"一致するモデルがありません。検索を変更するか ID を入力してください。",
  current:"現在のモデル", notSelected:"未選択", idle:"未確認", connected:"一覧取得済み", failed:"確認失敗",
  title:"モデル API 接続", protocol:"Chat Completions 互換", base:"API ベース URL", key:"モデル API キー", model:"モデル ID", models:"利用可能なモデル",
  test:"接続テスト / モデル取得", loading:"接続中…", cancel:"キャンセル", clear:"キーを消去", show:"キーを表示", hide:"キーを隠す", normalize:"正規化 URL を使用",
  trust:"このサービスへのモデル API キー送信を許可します。プラン生成時には作業状態も送信します。",
  hint:"Agent や Orchestra のトークンではなく、モデル提供者のキーです。ページのメモリにのみ保持され、再読み込み後は再入力が必要です。API アドレス変更時はキーを消去します。",
  modelHint:"一覧から選ぶか、正確なモデル ID を入力してください。空の一覧や /models 非対応でも手動設定できます。",
  scope:"接続テストは一覧の取得のみです。研究データ送信、推論、タスク実行は行わず、プラン生成の成功を保証しません。",
  ready:"接続が受け入れられました。モデルを選ぶか、手動の ID を保持してください。", empty:"接続成功ですが一覧は空です。モデル ID を手動入力してください。",
  native:"通信：ローカル Rust ゲートウェイ（CORS 不要）", browser:"通信：ブラウザー直接接続（提供者の CORS 許可が必要）",
  help:"プライバシーと互換性", scopeShort:"テストは一覧取得のみ。推論や研究データ送信はありません。",
  select:"モデルを選択", useModel:"このモデルで依頼を書く", changed:"アドレス変更により以前の API キーを消去しました。",
};
export function modelConnectionCopy(language: string) { return language.startsWith("zh") ? zh : language === "ja" ? ja : en; }

const errors: Record<"en" | "zh" | "ja", Record<AssistantConnectionCode, string>> = {
  en: {
    endpoint:"Use an HTTPS API base URL (for example, /v1), without a username, password, query or fragment. HTTP is only allowed on loopback.",
    trust_required:"Confirm trust in this exact service before sending a model key or context.", credentials:"Check the model ID and key format. A local service may accept an empty key.",
    unauthorized:"The service rejected authentication (401). Check the key and its target service.", forbidden:"Access was denied (403). Check model/project permissions and service policy.",
    not_found:"Endpoint or model not found (404/405). Check the API path. If /models is unsupported, enter the model ID manually.",
    rate_limited:"Rate or quota limit (429). Check your provider quota; no automatic retry was made.", server:"The provider or native gateway failed. Check service status and the API path; no automatic retry was made.",
    network:"Could not connect. Check network/TLS; browser-direct mode also needs CORS permission. Do not disable certificate checks.",
    timeout:"The request timed out. Its upstream outcome may be unknown; no automatic retry was made.", cancelled:"Stopped waiting. A model request already received by the provider may still run; no simulation task was dispatched.",
    response_too_large:"The model response exceeded the 1 MiB limit.", invalid_response:"The service returned an invalid response or unsupported plan. Check Chat Completions compatibility; no actions were executed.",
  },
  zh: {
    endpoint:"请填写 HTTPS API 基础地址（例如 /v1），不要包含用户名、密码、查询参数或片段。HTTP 仅允许本机回环地址。",
    trust_required:"发送模型密钥或上下文之前，请先确认信任这个服务。", credentials:"请检查模型 ID 和密钥格式。本地模型服务可以不填密钥。",
    unauthorized:"认证被拒绝（401）。请检查密钥是否正确，以及是否属于当前服务。", forbidden:"访问被拒绝（403）。请检查模型、项目权限和服务策略。",
    not_found:"接口或模型不存在（404/405）。请检查 API 路径；若不支持 /models，可手动填写模型 ID。",
    rate_limited:"触发速率或额度限制（429）。请检查服务商额度；没有自动重试。", server:"模型服务或本机网关请求失败。请检查服务状态及接口路径；没有自动重试。",
    network:"连接失败。请检查网络和 TLS；浏览器直连还需要服务商允许跨域。不要关闭证书验证。",
    timeout:"请求已超时，上游是否完成可能未知；没有自动重试。", cancelled:"已停止等待。服务商已收到的模型请求可能仍在运行；没有提交仿真任务。",
    response_too_large:"模型响应超过 1 MiB 限制。", invalid_response:"服务返回了无效响应或不支持的计划。请检查 Chat Completions 兼容性；没有执行动作。",
  },
  ja: {
    endpoint:"HTTPS の API ベース URL を入力してください。認証情報、クエリ、フラグメントは禁止です。HTTP はループバックのみです。",
    trust_required:"キーや作業状態を送信する前に、このサービスを信頼することを確認してください。", credentials:"モデル ID とキーの形式を確認してください。ローカルサービスではキーを省略できます。",
    unauthorized:"認証が拒否されました（401）。キーと接続先を確認してください。", forbidden:"アクセス拒否（403）。モデルやプロジェクトの権限を確認してください。",
    not_found:"エンドポイントまたはモデルがありません（404/405）。/models 非対応なら ID を手動入力してください。",
    rate_limited:"利用量または頻度の制限です（429）。自動再試行はしていません。", server:"モデルサービスまたはローカルゲートウェイが失敗しました。自動再試行はしていません。",
    network:"接続できません。ネットワーク、TLS、直接接続時の CORS を確認してください。証明書検証を無効にしないでください。",
    timeout:"タイムアウトしました。上流の結果は不明な場合があります。自動再試行はありません。", cancelled:"待機を停止しました。提供者が受け取った推論は継続する場合があります。解析タスクは送信していません。",
    response_too_large:"応答が 1 MiB の上限を超えました。", invalid_response:"無効な応答または未対応のプランです。互換性を確認してください。アクションは実行していません。",
  },
};
export function modelConnectionError(error: unknown, language: string) {
  const locale = language.startsWith("zh") ? "zh" : language === "ja" ? "ja" : "en";
  return errors[locale][error instanceof AssistantConnectionError ? error.code : "invalid_response"];
}
