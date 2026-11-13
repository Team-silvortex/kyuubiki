export function projectOperatorCopy(language: string) {
  if (language.startsWith("zh")) return {
    title: "项目算子", intro: "安装、项目启用、Agent 执行分别确认。启用记录保存在本机，固定版本和摘要；不是商店收藏，也不会启动或重配 Agent。",
    refresh: "核验本机安装", choose: "选择算子包", enable: "在此项目启用", disable: "在此项目停用", enabled: "已启用的包", empty: "此项目尚未启用算子包。",
    confirm: "确认更改当前项目的算子启用记录？不会卸载包、加载动态库或停止正在执行的任务。", project: "请先选择项目。", desktop: "需要桌面版及已更新的配套 Installer。",
    ready: "已核验，可准备任务", changed: "包缺失或内容已变化，禁止准备任务", operator: "执行算子", input: "输入 JSON", config: "配置 JSON", endpoint: "目标 Agent（host:port）",
    prepare: "核验并准备任务", run: "提交到 Agent", export: "导出 TaskIR", check: "查询原任务", ack: "确认结果并解除占位", pending: "待确认任务（当前控制通道）",
    runConfirm: "这将执行已启用包中的原生算子代码。确认已信任该包，并在指定 Agent 执行一次？断线不会自动重跑。",
    route: "此入口目前只支持直连 / Mesh 控制模式，且指定 Agent 必须已配置算子库和持久任务存储。不会借用商店账号权限或自动切换模式。",
    root: "供本机 Agent 配置的算子库根目录（不会自动修改）", recovery: "中断或结果不明时，只查询原任务。刷新页面后也可在系统的任务恢复面板找回；不得直接重跑。",
    stale: "页面、项目或控制配置已变化，未继续派发。", prepared: "任务已准备，尚未执行。", error: "操作未确认，请刷新核验或查询原任务。不要把超时当成未执行。",
  };
  if (language.startsWith("ja")) return {
    title: "プロジェクト演算子", intro: "インストール、有効化、Agent 実行は別々に確認します。固定バージョンとハッシュを本機に保存します。Agent の起動や設定変更は行いません。",
    refresh: "インストールを検証", choose: "パッケージを選択", enable: "このプロジェクトで有効化", disable: "このプロジェクトで無効化", enabled: "有効なパッケージ", empty: "有効なパッケージはありません。",
    confirm: "プロジェクトの有効化記録を変更しますか？アンインストール、ライブラリ読込、実行中タスクの停止は行いません。", project: "プロジェクトを選択してください。", desktop: "更新済み Installer を備えたデスクトップ版が必要です。",
    ready: "検証済み・タスク準備可能", changed: "パッケージが不在または変更されています", operator: "演算子", input: "入力 JSON", config: "設定 JSON", endpoint: "対象 Agent（host:port）",
    prepare: "検証してタスクを準備", run: "Agent に送信", export: "TaskIR を出力", check: "元のタスクを照会", ack: "結果を確認して予約解除", pending: "未確認タスク（現在の制御経路）",
    runConfirm: "信頼したパッケージのネイティブコードを対象 Agent で一度実行しますか？切断後の自動再実行はありません。",
    route: "直結 / Mesh モード専用です。Agent に演算子ライブラリと永続タスクストアの設定が必要です。ストアの認証や自動モード切替は使いません。",
    root: "本機 Agent 用ライブラリのルート（自動変更なし）", recovery: "不明な結果は元のタスクを照会してください。再読込後はシステムの復旧パネルから確認できます。再送しないでください。",
    stale: "ページ、プロジェクト、制御設定が変わったため送信しませんでした。", prepared: "準備済み・未実行。", error: "未確認です。再検証または元のタスクを照会してください。タイムアウトは未実行を意味しません。",
  };
  return {
    title: "Project operators", intro: "Confirm installation, project enablement and Agent execution separately. Exact versions and hashes are saved on this device. Enabling never starts or reconfigures an Agent.",
    refresh: "Verify local installations", choose: "Choose a package", enable: "Enable in this project", disable: "Disable in this project", enabled: "Enabled packages", empty: "No operator package enabled in this project.",
    confirm: "Change this project's enablement record? This will not uninstall, load libraries or stop running tasks.", project: "Select a project first.", desktop: "Requires the desktop app and an updated companion Installer.",
    ready: "Verified, ready to prepare", changed: "Missing or changed package; preparation blocked", operator: "Operator", input: "Input JSON", config: "Config JSON", endpoint: "Target Agent (host:port)",
    prepare: "Verify and prepare task", run: "Submit to Agent", export: "Export TaskIR", check: "Inspect original task", ack: "Acknowledge result and release reservation", pending: "Pending tasks (current control channel)",
    runConfirm: "Run native code from this trusted package once on the specified Agent? Connection loss will not trigger automatic replay.",
    route: "Direct / Mesh control mode only. The target Agent must already have its package library and durable execution store configured. No store credentials or automatic mode switching.",
    root: "Package root for local Agent configuration (not changed automatically)", recovery: "Inspect the original task when interrupted or uncertain. After reload, the System recovery panel can recover it. Do not resubmit.",
    stale: "Page, project or control settings changed; dispatch was not continued.", prepared: "Task prepared, not executed.", error: "Operation unconfirmed. Refresh verification or inspect the original task. A timeout does not mean nothing executed.",
  };
}

export function projectDeploymentCopy(language: string) {
  if (language.startsWith("zh")) return {
    inspect: "核验目标 Agent", deploy: "部署 / 继续上传", abort: "清理未完成上传",
    confirm: "将已启用的原生包发送到指定 Agent？不会覆盖冲突版本、重启 Agent 或自动执行。",
    abortConfirm: "清理目标 Agent 上这个包的未完成上传？已安装的包、其他上传和研究任务不会删除。",
    missing: "目标缺包", uploading: "上传中", installed_restart_required: "已安装，待受管重启后重新核验",
    ready: "目标已加载匹配版本，可执行",
    activation_failed: "包已安装，但外部算子宿主加载失败。请查看 Agent 本机日志，通过 Installer 停服修复后重启核验；不要重复上传或执行。",
    version_conflict: "版本或内容冲突：保留目标原包。请通过 Installer 明确停服更换，不会自动覆盖。",
  };
  if (language.startsWith("ja")) return {
    inspect: "Agent を検証", deploy: "配備 / 転送再開", abort: "未完了転送を削除",
    confirm: "対象 Agent に送信しますか？競合の上書き、再起動、自動実行は行いません。",
    abortConfirm: "このパッケージの未完了転送だけを削除しますか？インストール済みのコード、他の転送、研究タスクは保持します。",
    missing: "パッケージなし", uploading: "転送中", installed_restart_required: "インストール済み。管理された再起動後に再検証",
    ready: "一致するバージョンを読込済み",
    activation_failed: "インストール済みですが、外部演算子ホストの読込に失敗しました。Agent のローカルログを確認し、Installer で停止・修復・再起動後に再検証してください。再送・実行しないでください。",
    version_conflict: "バージョンまたは内容が競合しています。既存のパッケージを保持します。Installer で明示的に停止・交換してください。",
  };
  return {
    inspect: "Verify target Agent", deploy: "Deploy / resume upload", abort: "Clean incomplete upload",
    confirm: "Send this enabled native package to the target Agent? No conflict replacement, restart or automatic execution.",
    abortConfirm: "Remove only this package's incomplete upload on the target Agent? Installed packages, other uploads and research tasks will be retained.",
    missing: "Package missing", uploading: "Uploading", installed_restart_required: "Installed; managed restart and verification required",
    ready: "Matching version loaded, ready to execute",
    activation_failed: "Installed, but the external operator host failed to load. Check local Agent logs; stop and repair through Installer, restart, then verify. Do not reupload or execute.",
    version_conflict: "Version or content conflict. Existing package retained. Use Installer for an explicit stopped replacement; never overwrite automatically.",
  };
}
