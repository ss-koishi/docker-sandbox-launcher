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
        let path = self.path.trim();
        if self.readonly {
            format!("{path}:ro")
        } else {
            path.to_string()
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

impl CreateRequest {
    /// `sbx` に渡す引数列を組み立てる。
    pub fn to_args(&self) -> Result<Vec<String>, String> {
        let agent = self.agent.trim();
        let name = self.name.trim();
        if name.is_empty() {
            return Err("sandbox 名を入力してください".into());
        }
        if agent.is_empty() {
            return Err("エージェント（または kit 参照）を選んでください".into());
        }

        let mut args = vec![
            "create".to_string(),
            agent.to_string(),
            "--name".to_string(),
            name.to_string(),
        ];
        args.extend(
            self.workspaces
                .iter()
                .filter(|w| !w.path.trim().is_empty())
                .map(WorkspaceMount::to_arg),
        );
        for kit in self.kits.iter().filter(|k| !k.trim().is_empty()) {
            args.push("--kit".to_string());
            args.push(kit.trim().to_string());
        }
        Ok(args)
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn mount(path: &str, readonly: bool) -> WorkspaceMount {
        WorkspaceMount {
            path: path.into(),
            readonly,
        }
    }

    fn request() -> CreateRequest {
        CreateRequest {
            agent: "claude-vertex".into(),
            name: "my-project".into(),
            workspaces: vec![
                mount("/Users/me/sbx-workspaces/my-project", false),
                mount("/Users/me/src/acme", true),
            ],
            kits: vec!["/Users/me/.sbx/kits/aws".into()],
        }
    }

    #[test]
    fn primary_が先頭で_ro_は接尾辞になる() {
        assert_eq!(
            request().to_args().unwrap(),
            vec![
                "create",
                "claude-vertex",
                "--name",
                "my-project",
                "/Users/me/sbx-workspaces/my-project",
                "/Users/me/src/acme:ro",
                "--kit",
                "/Users/me/.sbx/kits/aws",
            ]
        );
    }

    #[test]
    fn 空の_workspace_と_kit_は落とす() {
        let req = CreateRequest {
            workspaces: vec![mount("  ", false)],
            kits: vec!["".into(), "  ".into()],
            ..request()
        };

        assert_eq!(
            req.to_args().unwrap(),
            vec!["create", "claude-vertex", "--name", "my-project"]
        );
    }

    /// `sbx ls --format json`（v0.50.0）の実出力をそのまま読めることを固定しておく。
    #[test]
    fn sbx_ls_の_json_を読める() {
        let json = r#"[
          {
            "name": "aws-ro",
            "id": "59409586-1d7c-4ced-9aed-b27b55409a5e",
            "agent": "claude-vertex",
            "status": "running",
            "last_used_at": "2026-10-29T01:44:45.856038Z",
            "workspaces": [
              "/Users/me/sbx-workspaces/aws-ro",
              "/Users/me/src/acme:ro"
            ],
            "created_at": "2026-10-20T04:59:55Z"
          },
          {
            "name": "gcp-co2",
            "id": "1917c3c3-8dd1-4aeb-9721-3da525a2d0f7",
            "agent": "claude",
            "status": "stopped",
            "workspaces": ["/Users/me/.config/sbx-kits/claude-code"],
            "created_at": "2026-10-19T04:53:17Z",
            "workspace_missing": true
          }
        ]"#;

        let list: Vec<Sandbox> = serde_json::from_str(json).unwrap();

        assert_eq!(list.len(), 2);
        assert_eq!(list[0].workspaces[1], "/Users/me/src/acme:ro");
        assert!(!list[0].workspace_missing);
        // last_used_at が無い要素もある。
        assert_eq!(list[1].last_used_at, None);
        assert!(list[1].workspace_missing);
    }

    #[test]
    fn 名前とエージェントは必須() {
        let no_name = CreateRequest {
            name: "  ".into(),
            ..request()
        };
        let no_agent = CreateRequest {
            agent: "".into(),
            ..request()
        };

        assert!(no_name.to_args().is_err());
        assert!(no_agent.to_args().is_err());
    }
}
