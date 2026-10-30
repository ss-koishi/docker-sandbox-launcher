//! kits ディレクトリの走査。
//!
//! 「kit = spec ファイルを 1 つ持つサブディレクトリ」として扱う。spec の中身は
//! スキーマのバージョンごとに形が違うので、構造体に落とさず必要なキーだけ拾う。

use std::path::Path;

use crate::model::Kit;

const SPEC_FILES: &[&str] = &["spec.yml", "spec.yaml", "kit.yml", "kit.yaml"];

/// kit ディレクトリに置く、ホスト側 sbx へ渡す環境変数のファイル。
///
/// spec.yml には書けない（未知のキーは `sbx kit validate` が弾く）ので、
/// このアプリ独自のサイドカーとして扱う。sbx はこのファイルを見ない。
pub const HOST_ENV_FILE: &str = "host-env";

/// kit を使う前にホストで済ませておく手順を書いておくファイル。
///
/// `sbx secret set` の引数は環境ごとに違い（どのサービスアカウントを
/// impersonate するか等）、MCP サーバーの登録は spec.yml に書く口が無い
/// (`field mcp not found in type spec.specFileV2`)。host-env と同じく
/// このアプリ独自のサイドカーとして扱う。sbx はこのファイルを見ない。
///
/// 先頭にディレクティブを書くと、アプリが用意済みかを確認する。
/// ディレクティブはコメント形式なので、ファイルごとシェルに貼っても動く。
///
/// ```sh
/// #secret: mcp:datadog:pat   → sbx secret ls と突き合わせる
/// #mcp: datadog              → sbx mcp ls と突き合わせ、作成時に --static-mcp で渡す
/// #file: ~/.config/...       → ホストにそのファイルがあるかを見る
/// ```
pub const SECRET_SETUP_FILE: &str = "secret-setup";

const SECRET_DIRECTIVE: &str = "#secret:";
const MCP_DIRECTIVE: &str = "#mcp:";
const FILE_DIRECTIVE: &str = "#file:";

/// サイドカーを読み取った結果。
#[derive(Default)]
struct Setup {
    secrets: Vec<String>,
    mcp: Vec<String>,
    files: Vec<String>,
    /// ディレクティブを除いた、画面に出すコマンド。
    command: String,
}

/// サイドカーを、ディレクティブと「表示するコマンド」に分ける。
fn parse_setup(text: &str) -> Setup {
    let mut setup = Setup::default();
    let mut lines = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(SECRET_DIRECTIVE) {
            push_name(&mut setup.secrets, rest);
        } else if let Some(rest) = trimmed.strip_prefix(MCP_DIRECTIVE) {
            push_name(&mut setup.mcp, rest);
        } else if let Some(rest) = trimmed.strip_prefix(FILE_DIRECTIVE) {
            push_name(&mut setup.files, &expand_home(rest.trim()));
        } else {
            lines.push(line);
        }
    }

    setup.command = lines.join("\n").trim().to_string();
    setup
}

/// 先頭の `~` をホームに展開する。それ以外はそのまま。
fn expand_home(path: &str) -> String {
    match path.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", crate::model::home_dir()),
        None => path.to_string(),
    }
}

fn push_name(into: &mut Vec<String>, raw: &str) {
    let name = raw.trim();
    if !name.is_empty() && !into.iter().any(|n| n == name) {
        into.push(name.to_string());
    }
}

/// 1 つの kit ディレクトリの host-env を読む。無ければ空。
pub fn host_env(kit_dir: &str) -> Vec<(String, String)> {
    std::fs::read_to_string(Path::new(kit_dir).join(HOST_ENV_FILE))
        .map(|text| crate::model::parse_env(&text))
        .unwrap_or_default()
}

/// 複数の kit の host-env を、並び順どおりに後勝ちで畳み込む。
pub fn host_env_for(kit_dirs: &[String]) -> Vec<(String, String)> {
    let layers: Vec<_> = kit_dirs.iter().map(|d| host_env(d)).collect();
    crate::model::merge_env(&layers)
}

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

    let dir = path.display().to_string();
    let setup = std::fs::read_to_string(path.join(SECRET_SETUP_FILE))
        .map(|text| parse_setup(&text))
        .unwrap_or_default();

    // ファイルの有無はここで見る。フロントからは stat できないため。
    let missing_files = setup
        .files
        .iter()
        .filter(|p| !Path::new(p).exists())
        .cloned()
        .collect();

    let mut kit = Kit {
        id,
        host_env_keys: host_env(&dir).into_iter().map(|(k, _)| k).collect(),
        path: dir,
        name: None,
        display_name: None,
        kind: None,
        requires_agent: None,
        // spec の credentials は下で足す。ここはサイドカー由来の分。
        required_secrets: setup.secrets,
        required_mcp: setup.mcp,
        required_files: setup.files,
        missing_files,
        secret_setup: Some(setup.command).filter(|c| !c.is_empty()),
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
            for service in sequence_field(&spec, "credentials", "service") {
                push_name(&mut kit.required_secrets, &service);
            }
        }
        Err(e) => kit.error = Some(format!("spec を読めませんでした: {e}")),
    }

    Some(kit)
}

/// `key` のシーケンスを辿り、各要素の `field` を集める。形が違えば空。
///
/// ```yaml
/// credentials:
///   - service: gcp      # ← field = "service" なら ["gcp"]
/// ```
fn sequence_field(value: &serde_yaml::Value, key: &str, field: &str) -> Vec<String> {
    let Some(items) = value
        .as_mapping()
        .and_then(|m| m.get(serde_yaml::Value::from(key)))
        .and_then(serde_yaml::Value::as_sequence)
    else {
        return Vec::new();
    };

    let mut found: Vec<String> = items
        .iter()
        .filter_map(|item| string_at(item, &[field]))
        .collect();
    found.dedup();
    found
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

#[cfg(test)]
mod tests {
    use super::*;

    /// テスト用の使い捨てディレクトリ。drop で消す。
    struct TempDir(std::path::PathBuf);

    impl TempDir {
        fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "dsl-test-{tag}-{}-{:?}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }

        fn kit(&self, name: &str, spec: Option<&str>) {
            let dir = self.0.join(name);
            std::fs::create_dir_all(&dir).unwrap();
            if let Some(spec) = spec {
                std::fs::write(dir.join("spec.yml"), spec).unwrap();
            }
        }

        fn with_host_env(&self, name: &str, text: &str) {
            std::fs::write(self.0.join(name).join(HOST_ENV_FILE), text).unwrap();
        }

        fn dir(&self, name: &str) -> String {
            self.0.join(name).display().to_string()
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn spec_のあるディレクトリだけを_kit_として返す() {
        let tmp = TempDir::new("scan");
        tmp.kit(
            "aws",
            Some("schemaVersion: \"2\"\nkind: mixin\nname: aws-cli\ndisplayName: AWS CLI\n"),
        );
        // spec を持たないので kit ではない。
        tmp.kit("files", None);
        // ドット始まりは無視する。
        tmp.kit(".hidden", Some("kind: mixin\nname: hidden\n"));

        let kits = scan(&tmp.0.display().to_string()).unwrap();

        assert_eq!(kits.len(), 1);
        assert_eq!(kits[0].id, "aws");
        assert_eq!(kits[0].kind.as_deref(), Some("mixin"));
        assert_eq!(kits[0].display_name.as_deref(), Some("AWS CLI"));
        assert_eq!(kits[0].label(), "AWS CLI");
        assert!(kits[0].error.is_none());
    }

    #[test]
    fn requires_が_v3_のリスト形式でも壊れない() {
        let tmp = TempDir::new("requires");
        tmp.kit("v2", Some("kind: mixin\nname: v2\nrequires:\n  agent: claude-vertex\n"));
        tmp.kit("v3", Some("kind: mixin\nname: v3\nrequires: [gojq]\n"));

        let kits = scan(&tmp.0.display().to_string()).unwrap();
        let by_id = |id: &str| kits.iter().find(|k| k.id == id).unwrap().clone();

        assert_eq!(by_id("v2").requires_agent.as_deref(), Some("claude-vertex"));
        assert_eq!(by_id("v3").requires_agent, None);
    }

    #[test]
    fn 壊れた_spec_は_error_を付けて返す() {
        let tmp = TempDir::new("broken");
        tmp.kit("bad", Some("kind: mixin\n  name: [unbalanced\n"));

        let kits = scan(&tmp.0.display().to_string()).unwrap();

        assert_eq!(kits.len(), 1);
        assert!(kits[0].error.is_some());
        // 名前が読めなくてもディレクトリ名で表示できる。
        assert_eq!(kits[0].label(), "bad");
    }

    #[test]
    fn host_env_を読んで一覧にキーを出す() {
        let tmp = TempDir::new("hostenv");
        tmp.kit("vertex", Some("kind: mixin\nname: vertex\n"));
        tmp.with_host_env(
            "vertex",
            "# claude-vertex がホスト側で要求する\nANTHROPIC_VERTEX_PROJECT_ID=p1\nCLOUD_ML_REGION=global\n",
        );
        // host-env を持たない kit は空のまま。
        tmp.kit("plain", Some("kind: mixin\nname: plain\n"));

        let kits = scan(&tmp.0.display().to_string()).unwrap();
        let by_id = |id: &str| kits.iter().find(|k| k.id == id).unwrap().clone();

        assert_eq!(
            by_id("vertex").host_env_keys,
            vec!["ANTHROPIC_VERTEX_PROJECT_ID", "CLOUD_ML_REGION"]
        );
        assert!(by_id("plain").host_env_keys.is_empty());
    }

    #[test]
    fn 複数の_kit_の_host_env_を畳み込む() {
        let tmp = TempDir::new("merge");
        tmp.kit("a", Some("kind: mixin\nname: a\n"));
        tmp.kit("b", Some("kind: mixin\nname: b\n"));
        tmp.with_host_env("a", "SHARED=from-a\nONLY_A=1\n");
        tmp.with_host_env("b", "SHARED=from-b\n");

        // 後ろの kit が勝つ。
        assert_eq!(
            host_env_for(&[tmp.dir("a"), tmp.dir("b")]),
            vec![
                ("SHARED".to_string(), "from-b".to_string()),
                ("ONLY_A".to_string(), "1".to_string()),
            ]
        );
        // 存在しないディレクトリを混ぜても落ちない。
        assert!(host_env_for(&["/no/such/kit".to_string()]).is_empty());
    }

    #[test]
    fn credentials_から必要な_secret_を拾う() {
        let tmp = TempDir::new("secrets");
        tmp.kit(
            "gcp",
            Some(
                "kind: mixin\nname: gcp\ncredentials:\n  \
                 - service: gcp\n    required: true\n    apiKey:\n      name: SBX_GCP_TOKEN\n  \
                 - service: datadog-pat\n    required: true\n",
            ),
        );
        // credentials を持たない kit は空のまま。
        tmp.kit("plain", Some("kind: mixin\nname: plain\n"));
        std::fs::write(
            tmp.0.join("gcp").join(SECRET_SETUP_FILE),
            "sbx secret set gcp --command \"...\" --refresh 45m\n",
        )
        .unwrap();

        let kits = scan(&tmp.0.display().to_string()).unwrap();
        let by_id = |id: &str| kits.iter().find(|k| k.id == id).unwrap().clone();

        assert_eq!(by_id("gcp").required_secrets, vec!["gcp", "datadog-pat"]);
        assert!(by_id("gcp").secret_setup.unwrap().contains("--refresh 45m"));
        assert!(by_id("plain").required_secrets.is_empty());
        assert_eq!(by_id("plain").secret_setup, None);
    }

    #[test]
    fn サイドカーのディレクティブを読み分ける() {
        let tmp = TempDir::new("directives");
        tmp.kit("datadog", Some("kind: mixin\nname: datadog\n"));
        std::fs::write(
            tmp.0.join("datadog").join(SECRET_SETUP_FILE),
            "# 説明のコメントは残す\n\
             #secret: mcp:datadog:pat\n\
             #mcp: datadog\n\
             sbx mcp add datadog --url https://example.com/mcp\n\
             sbx secret set mcp:datadog:pat\n",
        )
        .unwrap();

        let kits = scan(&tmp.0.display().to_string()).unwrap();
        let kit = &kits[0];

        assert_eq!(kit.required_secrets, vec!["mcp:datadog:pat"]);
        assert_eq!(kit.required_mcp, vec!["datadog"]);

        // ディレクティブは表示用のコマンドから取り除き、普通のコメントは残す。
        let command = kit.secret_setup.clone().unwrap();
        assert!(command.contains("# 説明のコメントは残す"));
        assert!(command.contains("sbx mcp add datadog"));
        assert!(!command.contains("#secret:"));
        assert!(!command.contains("#mcp:"));
    }

    #[test]
    fn file_ディレクティブで存在を確認する() {
        let tmp = TempDir::new("files");
        tmp.kit("vertex", Some("kind: mixin\nname: vertex\n"));
        // 自分自身の spec.yml は必ずあるので、あるケースの題材に使う。
        let present = tmp.0.join("vertex").join("spec.yml").display().to_string();
        std::fs::write(
            tmp.0.join("vertex").join(SECRET_SETUP_FILE),
            format!("#file: {present}\n#file: /no/such/adc.json\ngcloud auth application-default login\n"),
        )
        .unwrap();

        let kits = scan(&tmp.0.display().to_string()).unwrap();
        let kit = &kits[0];

        assert_eq!(kit.required_files, vec![present, "/no/such/adc.json".into()]);
        assert_eq!(kit.missing_files, vec!["/no/such/adc.json"]);
    }

    #[test]
    fn file_ディレクティブの_チルダ_を展開する() {
        let home = crate::model::home_dir();
        let setup = parse_setup("#file: ~/.config/gcloud/adc.json\n");

        assert_eq!(setup.files, vec![format!("{home}/.config/gcloud/adc.json")]);
        // 絶対パスはそのまま。
        assert_eq!(parse_setup("#file: /etc/hosts\n").files, vec!["/etc/hosts"]);
    }

    #[test]
    fn spec_とサイドカーの_secret_は合併して重複しない() {
        let tmp = TempDir::new("union");
        tmp.kit(
            "both",
            Some("kind: mixin\nname: both\ncredentials:\n  - service: gcp\n"),
        );
        std::fs::write(
            tmp.0.join("both").join(SECRET_SETUP_FILE),
            "#secret: gcp\n#secret: extra\nsbx secret set gcp\n",
        )
        .unwrap();

        let kits = scan(&tmp.0.display().to_string()).unwrap();
        assert_eq!(kits[0].required_secrets, vec!["gcp", "extra"]);
    }

    #[test]
    fn credentials_の形が違っても落ちない() {
        let tmp = TempDir::new("badcreds");
        // シーケンスではなくマッピング。
        tmp.kit("a", Some("kind: mixin\nname: a\ncredentials:\n  service: gcp\n"));
        // service キーが無い要素。
        tmp.kit("b", Some("kind: mixin\nname: b\ncredentials:\n  - required: true\n"));

        let kits = scan(&tmp.0.display().to_string()).unwrap();
        assert!(kits.iter().all(|k| k.required_secrets.is_empty()));
    }

    #[test]
    fn ディレクトリが無ければエラーになる() {
        assert!(scan("/no/such/kits/dir").is_err());
        assert!(scan("  ").is_err());
    }
}
