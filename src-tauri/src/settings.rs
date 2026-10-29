//! アプリ設定の永続化。`~/Library/Application Support/<identifier>/settings.json`。

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::model::AppSettings;

fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_config_dir()
        .map_err(|e| format!("設定ディレクトリを特定できませんでした: {e}"))?;
    Ok(dir.join("settings.json"))
}

/// 保存済みの設定を読む。未保存・壊れている場合は既定値にフォールバックする。
pub fn load(app: &AppHandle) -> AppSettings {
    settings_path(app)
        .ok()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("{} を作成できませんでした: {e}", parent.display()))?;
    }
    let text = serde_json::to_string_pretty(settings)
        .map_err(|e| format!("設定を JSON にできませんでした: {e}"))?;
    std::fs::write(&path, text)
        .map_err(|e| format!("{} に書き込めませんでした: {e}", path.display()))
}
