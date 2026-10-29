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
    /// この kit を使うときにホスト側の sbx へ渡す環境変数のキー。
    /// 値は表示しない（必要なら host-env ファイルを直接見る）。
    pub host_env_keys: Vec<String>,
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
    /// ターミナル連携先。`auto` | `herdr` | `ghostty` | `custom`。
    pub terminal: String,
    /// herdr 実行ファイルのパス。空なら自動探索する。
    pub herdr_path: String,
    /// `terminal` が `custom` のときのコマンドテンプレート。`{command}` を置換する。
    pub terminal_command: String,
    /// herdr で開いたあと、Ghostty を前面に出すか。
    pub focus_terminal: bool,
    /// sbx に渡す環境変数。1 行 1 つの `KEY=VALUE`。
    ///
    /// エージェントによってはホスト側の環境変数を要求する
    /// (claude-vertex の `ANTHROPIC_VERTEX_PROJECT_ID` など)。これは kit の
    /// spec.yml や sbxenv.yaml の `env:` では満たせない。どちらもサンドボックス
    /// 側のスコープで、チェックはホストの sbx プロセスが行うため。
    pub extra_env: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        let home = home_dir();
        Self {
            kits_dir: join_home(&home, ".sbx/kits"),
            sbx_path: String::new(),
            workspace_root: join_home(&home, "sbx-workspaces"),
            terminal: "auto".into(),
            herdr_path: String::new(),
            terminal_command: String::new(),
            focus_terminal: true,
            extra_env: String::new(),
        }
    }
}

/// `KEY=VALUE` を 1 行ずつ並べたテキストを解釈する。
/// 空行と `#` 始まりの行は読み飛ばし、キーが空のものは捨てる。
pub fn parse_env(text: &str) -> Vec<(String, String)> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            let key = key.trim();
            if key.is_empty() {
                return None;
            }
            Some((key.to_string(), value.trim().to_string()))
        })
        .collect()
}

/// 後から来たものが前を上書きするように環境変数を畳み込む。
pub fn merge_env(layers: &[Vec<(String, String)>]) -> Vec<(String, String)> {
    let mut merged: Vec<(String, String)> = Vec::new();
    for layer in layers {
        for (key, value) in layer {
            match merged.iter_mut().find(|(k, _)| k == key) {
                Some(slot) => slot.1 = value.clone(),
                None => merged.push((key.clone(), value.clone())),
            }
        }
    }
    merged
}

impl AppSettings {
    /// `extra_env` の複数行テキストを `(KEY, VALUE)` の並びにする。
    pub fn env_pairs(&self) -> Vec<(String, String)> {
        parse_env(&self.extra_env)
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

    fn with_env(text: &str) -> AppSettings {
        AppSettings {
            extra_env: text.into(),
            ..AppSettings::default()
        }
    }

    #[test]
    fn 環境変数の複数行テキストを解釈する() {
        let s = with_env(
            "
            # コメントは無視する
            ANTHROPIC_VERTEX_PROJECT_ID=my-gcp-project
            CLOUD_ML_REGION = global

            EMPTY=
            =値だけ
            値なし
        ",
        );

        assert_eq!(
            s.env_pairs(),
            vec![
                ("ANTHROPIC_VERTEX_PROJECT_ID".to_string(), "my-gcp-project".to_string()),
                ("CLOUD_ML_REGION".to_string(), "global".to_string()),
                ("EMPTY".to_string(), String::new()),
            ]
        );
    }

    #[test]
    fn 値に等号が含まれていても壊れない() {
        assert_eq!(
            with_env("OTEL_RESOURCE_ATTRIBUTES=team.member=foo@example.com").env_pairs(),
            vec![(
                "OTEL_RESOURCE_ATTRIBUTES".to_string(),
                "team.member=foo@example.com".to_string()
            )]
        );
    }

    #[test]
    fn 環境変数は後のレイヤーが勝つ() {
        let global = parse_env("A=1\nB=2");
        let kit = parse_env("B=99\nC=3");

        assert_eq!(
            merge_env(&[global, kit]),
            vec![
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "99".to_string()),
                ("C".to_string(), "3".to_string()),
            ]
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
