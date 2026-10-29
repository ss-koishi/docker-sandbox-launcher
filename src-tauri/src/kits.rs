//! kits ディレクトリの走査。
//!
//! 「kit = spec ファイルを 1 つ持つサブディレクトリ」として扱う。spec の中身は
//! スキーマのバージョンごとに形が違うので、構造体に落とさず必要なキーだけ拾う。

use std::path::Path;

use crate::model::Kit;

const SPEC_FILES: &[&str] = &["spec.yml", "spec.yaml", "kit.yml", "kit.yaml"];

/// `dir` 直下のサブディレクトリを kit として列挙する。表示名順に並べて返す。
pub fn scan(dir: &str) -> Result<Vec<Kit>, String> {
    let dir = dir.trim();
    if dir.is_empty() {
        return Err("kits ディレクトリが設定されていません".into());
    }
    let root = Path::new(dir);
    if !root.is_dir() {
        return Err(format!("kits ディレクトリが見つかりません: {dir}"));
    }

    let entries =
        std::fs::read_dir(root).map_err(|e| format!("{dir} を読み取れませんでした: {e}"))?;

    let mut kits: Vec<Kit> = entries
        .filter_map(Result::ok)
        .filter(|e| e.path().is_dir())
        .filter(|e| !e.file_name().to_string_lossy().starts_with('.'))
        .filter_map(|e| read_kit(&e.path()))
        .collect();

    kits.sort_by(|a, b| a.label().to_lowercase().cmp(&b.label().to_lowercase()));
    Ok(kits)
}

/// spec ファイルを持たないディレクトリは kit ではないので `None` を返す。
fn read_kit(path: &Path) -> Option<Kit> {
    let id = path.file_name()?.to_string_lossy().to_string();
    let spec_file = SPEC_FILES
        .iter()
        .map(|name| path.join(name))
        .find(|p| p.is_file())?;

    let mut kit = Kit {
        id,
        path: path.display().to_string(),
        name: None,
        display_name: None,
        kind: None,
        requires_agent: None,
        spec_file: Some(spec_file.display().to_string()),
        error: None,
    };

    match std::fs::read_to_string(&spec_file)
        .map_err(|e| e.to_string())
        .and_then(|text| serde_yaml::from_str::<serde_yaml::Value>(&text).map_err(|e| e.to_string()))
    {
        Ok(spec) => {
            kit.name = string_at(&spec, &["name"]);
            kit.display_name = string_at(&spec, &["displayName"]);
            kit.kind = string_at(&spec, &["kind"]);
            // v2 は `requires: {agent: ...}`、v3 は `requires: [...]` のことがある。
            kit.requires_agent = string_at(&spec, &["requires", "agent"]);
        }
        Err(e) => kit.error = Some(format!("spec を読めませんでした: {e}")),
    }

    Some(kit)
}

/// ネストしたマッピングを辿って文字列を取り出す。形が違えば素直に `None`。
fn string_at(value: &serde_yaml::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.as_mapping()?.get(serde_yaml::Value::from(*key))?;
    }
    current.as_str().map(|s| s.to_string())
}

impl Kit {
    /// 一覧に出す名前。displayName → name → ディレクトリ名の順に使う。
    pub fn label(&self) -> &str {
        self.display_name
            .as_deref()
            .or(self.name.as_deref())
            .unwrap_or(&self.id)
    }
}
