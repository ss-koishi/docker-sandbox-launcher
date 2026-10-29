//! フロントエンドとやり取りするデータ構造。
//!
//! `sbx ls --format json` の出力をそのまま受けられる形にしてあるので、
//! CLI 側にフィールドが増えても既存フィールドの読み出しは壊れない。

use serde::{Deserialize, Serialize};

/// `sbx ls --format json` の 1 要素。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sandbox {
    pub name: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub agent: String,
    #[serde(default)]
    pub status: String,
    /// 先頭が primary workspace。読み取り専用マウントは `/path:ro` の形で入る。
    #[serde(default)]
    pub workspaces: Vec<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub last_used_at: Option<String>,
    #[serde(default)]
    pub workspace_missing: bool,
}

/// kits ディレクトリ配下で見つかったローカル kit 1 件。
#[derive(Debug, Clone, Serialize)]
pub struct Kit {
    /// ディレクトリ名。ローカル kit ではこれが `--kit-arg` のハンドルになる。
    pub id: String,
    /// `--kit` にそのまま渡せる絶対パス。
    pub path: String,
    /// spec 内の `name`。
    pub name: Option<String>,
    /// spec 内の `displayName`。
    pub display_name: Option<String>,
    /// `workload` か `mixin`。`--kit` に渡せるのは mixin だけ。
    pub kind: Option<String>,
    /// `requires.agent`。指定があればそのエージェントとしか組み合わせられない。
    pub requires_agent: Option<String>,
    /// 見つかった spec ファイルの絶対パス。
    pub spec_file: Option<String>,
    /// spec の読み取りに失敗した場合の理由。
    pub error: Option<String>,
}

/// create 時に渡す 1 つのホストディレクトリ。
#[derive(Debug, Clone, Deserialize)]
pub struct WorkspaceMount {
    pub path: String,
    #[serde(default)]
    pub readonly: bool,
}

impl WorkspaceMount {
    /// `sbx create` の位置引数の形 (`/path` または `/path:ro`) にする。
    pub fn to_arg(&self) -> String {
        if self.readonly {
            format!("{}:ro", self.path)
        } else {
            self.path.clone()
        }
    }
}

/// sandbox 作成リクエスト。
#[derive(Debug, Clone, Deserialize)]
pub struct CreateRequest {
    /// エージェント名、または kit 参照（`sbx create` の第 1 位置引数）。
    pub agent: String,
    pub name: String,
    /// 先頭が primary workspace。空なら bind mount なしで作られる。
    #[serde(default)]
    pub workspaces: Vec<WorkspaceMount>,
    /// `--kit` に渡す mixin 参照のリスト。
    #[serde(default)]
    pub kits: Vec<String>,
}

/// アプリ自身の設定。`app_config_dir()/settings.json` に保存する。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    /// ローカル kit を置いてあるディレクトリ。
    pub kits_dir: String,
    /// sbx 実行ファイルのパス。空なら自動探索する。
    pub sbx_path: String,
    /// 「workspace を新規作成」したときの作成先。
    pub workspace_root: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let home = home_dir();
        Self {
            kits_dir: join_home(&home, ".sbx/kits"),
            sbx_path: String::new(),
            workspace_root: join_home(&home, "sbx-workspaces"),
        }
    }
}

/// sbx の検出結果。設定画面で「見つかっているか」を示すのに使う。
#[derive(Debug, Clone, Serialize)]
pub struct SbxInfo {
    pub path: String,
    pub version: String,
}

pub fn home_dir() -> String {
    std::env::var("HOME").unwrap_or_default()
}

fn join_home(home: &str, rel: &str) -> String {
    if home.is_empty() {
        String::new()
    } else {
        format!("{home}/{rel}")
    }
}
