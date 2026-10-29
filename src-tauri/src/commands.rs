//! フロントエンドから呼ばれる Tauri コマンド。
//!
//! どれも「設定を読む → sbx を呼ぶ」の 2 ステップしかない薄い層にしてある。

use serde::Deserialize;
use tauri::AppHandle;
use tauri::ipc::Channel;

use crate::kits;
use crate::model::{AppSettings, CreateRequest, Kit, Sandbox, SbxInfo};
use crate::sbx::{LogLine, Sbx};
use crate::settings;

/// GUI から投げてよいライフサイクル動詞と、常に付けるフラグ。
///
/// stop / prune を UI に出すときは、ここに動詞が載っているので
/// フロント側にボタンを足すだけでよい（Rust 側の変更は不要）。
pub const LIFECYCLE_VERBS: &[(&str, &[&str])] = &[
    // GUI 側で確認ダイアログを出すので、CLI の確認プロンプトは常に飛ばす。
    ("rm", &["--force"]),
    ("stop", &[]),
    ("prune", &["--force"]),
];

fn sbx(app: &AppHandle) -> Result<Sbx, String> {
    Sbx::locate(&settings::load(app).sbx_path)
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> AppSettings {
    settings::load(&app)
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: AppSettings) -> Result<AppSettings, String> {
    let normalized = AppSettings {
        kits_dir: settings.kits_dir.trim().to_string(),
        sbx_path: settings.sbx_path.trim().to_string(),
        workspace_root: settings.workspace_root.trim().to_string(),
    };
    settings::save(&app, &normalized)?;
    Ok(normalized)
}

#[tauri::command]
pub fn detect_sbx(app: AppHandle) -> Result<SbxInfo, String> {
    let cli = sbx(&app)?;
    let version = cli
        .output(&["version"])
        .unwrap_or_default()
        .lines()
        .next()
        .unwrap_or("")
        .trim()
        .to_string();
    Ok(SbxInfo {
        path: cli.program().display().to_string(),
        version,
    })
}

#[tauri::command]
pub async fn list_sandboxes(app: AppHandle) -> Result<Vec<Sandbox>, String> {
    blocking(move || sbx(&app)?.json::<Vec<Sandbox>>(&["ls"])).await
}

/// `sbx settings` の `kit.shortcuts.<agent>` キーから、選択できるエージェント名を得る。
/// ビルトインの一覧が CLI 側で増減しても追随する。
#[tauri::command]
pub async fn list_agents(app: AppHandle) -> Result<Vec<String>, String> {
    #[derive(Deserialize)]
    struct Setting {
        key: String,
    }

    blocking(move || {
        let entries: Vec<Setting> = sbx(&app)?.json(&["settings", "ls"])?;
        let mut agents: Vec<String> = entries
            .iter()
            .filter_map(|s| s.key.strip_prefix("kit.shortcuts."))
            .map(|s| s.to_string())
            .collect();
        agents.sort();
        agents.dedup();
        Ok(agents)
    })
    .await
}

#[tauri::command]
pub fn list_kits(app: AppHandle) -> Result<Vec<Kit>, String> {
    kits::scan(&settings::load(&app).kits_dir)
}

/// kit の spec を原文のまま返す（閲覧専用）。設定された kits ディレクトリの外は読まない。
#[tauri::command]
pub fn read_kit_spec(app: AppHandle, path: String) -> Result<String, String> {
    let root = std::fs::canonicalize(settings::load(&app).kits_dir)
        .map_err(|e| format!("kits ディレクトリを解決できませんでした: {e}"))?;
    let target = std::fs::canonicalize(&path)
        .map_err(|e| format!("{path} を解決できませんでした: {e}"))?;
    if !target.starts_with(&root) {
        return Err("kits ディレクトリの外のファイルは開けません".into());
    }
    std::fs::read_to_string(&target).map_err(|e| format!("{path} を読めませんでした: {e}"))
}

#[tauri::command]
pub async fn create_sandbox(
    app: AppHandle,
    request: CreateRequest,
    on_log: Channel<LogLine>,
) -> Result<(), String> {
    let args = request.to_args()?;

    blocking(move || {
        let cli = sbx(&app)?;
        let code = cli.stream(&args, |line| {
            let _ = on_log.send(line);
        })?;
        if code == 0 {
            Ok(())
        } else {
            Err(format!("sbx create が終了コード {code} で失敗しました"))
        }
    })
    .await
}

/// rm / stop / prune をまとめて扱う。許可された動詞かどうかは Rust 側で検証する。
#[tauri::command]
pub async fn run_lifecycle(
    app: AppHandle,
    verb: String,
    targets: Vec<String>,
) -> Result<String, String> {
    let Some((verb, flags)) = LIFECYCLE_VERBS.iter().find(|(v, _)| *v == verb) else {
        return Err(format!("未対応の操作です: {verb}"));
    };

    blocking(move || sbx(&app)?.lifecycle(verb, &targets, flags)).await
}

/// workspace 用のディレクトリを作って絶対パスを返す。既にあればそのまま返す。
#[tauri::command]
pub fn ensure_workspace_dir(app: AppHandle, name: String) -> Result<String, String> {
    let root = settings::load(&app).workspace_root;
    let name = name.trim();
    if root.is_empty() {
        return Err("workspace の作成先が設定されていません".into());
    }
    if name.is_empty() || name.contains('/') {
        return Err("sandbox 名にディレクトリ区切りは使えません".into());
    }

    let path = std::path::Path::new(&root).join(name);
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("{} を作成できませんでした: {e}", path.display()))?;
    Ok(path.display().to_string())
}

#[tauri::command]
pub fn path_exists(path: String) -> bool {
    !path.trim().is_empty() && std::path::Path::new(path.trim()).exists()
}

/// sbx の呼び出しは全部ブロッキングなので、UI スレッドを塞がないよう別スレッドへ逃がす。
async fn blocking<T, F>(f: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| format!("処理を実行できませんでした: {e}"))?
}
