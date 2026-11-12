//! sandbox をホストのターミナルで開く。
//!
//! 連携先は 4 つ。
//!
//! - **herdr**: ソケット API で workspace を作り、その root pane でコマンドを走らせる。
//!   herdr サーバーが持つので、ターミナルを閉じてもセッションは生き残る。
//! - **Herdr（GUI）**: デスクトップアプリ（`herdr-app`）に space を 1 つ作り、
//!   その最初のターミナルでコマンドを走らせる。中身は herdr-core が持つので、
//!   こちらもウィンドウを閉じてもセッションは残る。
//! - **Ghostty**: AppleScript で既存インスタンスに新しいウィンドウを開く。
//!   macOS では `ghostty -e` が使えず、`open -na` だと起動のたびに別インスタンスが
//!   増えてしまうため、AppleScript を使っている。
//! - **カスタム**: `{command}` を含むテンプレートを `sh -c` に渡す。

use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Serialize;

use crate::model::AppSettings;
use crate::sbx::{command_for, locate_program, output_of};

const GHOSTTY_APP: &str = "/Applications/Ghostty.app";
/// Herdr デスクトップアプリ。インストール先が固定でないので bundle id で前面に出す。
const HERDR_APP_BUNDLE_ID: &str = "dev.herdr.app";

/// 設定画面に出す、連携先の検出結果。
#[derive(Debug, Clone, Serialize)]
pub struct TerminalInfo {
    /// 実際に使われる連携先（auto を解決した結果）。
    pub effective: String,
    pub herdr_running: bool,
    pub herdr_path: Option<String>,
    /// Herdr デスクトップアプリのウィンドウが開いているか。
    pub herdr_app_running: bool,
    pub herdr_app_path: Option<String>,
    pub ghostty_installed: bool,
}

pub fn info(settings: &AppSettings) -> TerminalInfo {
    let herdr = locate_program("herdr", &settings.herdr_path).ok();
    let herdr_running = herdr.as_deref().is_some_and(server_running);
    let herdr_app = locate_program("herdr-app", &settings.herdr_app_path).ok();
    let herdr_app_running = herdr_app.as_deref().is_some_and(window_running);
    let ghostty_installed = Path::new(GHOSTTY_APP).is_dir();

    let effective = match settings.terminal.as_str() {
        "herdr" | "herdr-gui" | "ghostty" | "custom" => settings.terminal.clone(),
        // auto: 開いている Herdr のウィンドウ → herdr サーバー → Ghostty の順。
        _ if herdr_app_running => "herdr-gui".into(),
        _ if herdr_running => "herdr".into(),
        _ => "ghostty".into(),
    };

    TerminalInfo {
        effective,
        herdr_running,
        herdr_path: herdr.map(|p| p.display().to_string()),
        herdr_app_running,
        herdr_app_path: herdr_app.map(|p| p.display().to_string()),
        ghostty_installed,
    }
}

/// `argv` を `label` という名前でターミナルに開く。使った連携先の名前を返す。
///
/// `env` は `KEY=VALUE` の並び。ターミナルの中で走る sbx もホスト扱いなので、
/// アプリが sbx を直接呼ぶときと同じものを渡す。
pub fn open(
    settings: &AppSettings,
    label: &str,
    cwd: Option<&str>,
    argv: &[String],
    env: &[String],
) -> Result<String, String> {
    if argv.is_empty() {
        return Err("実行するコマンドが空です".into());
    }

    match info(settings).effective.as_str() {
        "herdr" => open_herdr(settings, label, cwd, argv, env),
        "herdr-gui" => open_herdr_gui(settings, label, cwd, argv, env),
        "custom" => open_custom(settings, argv, env),
        _ => open_ghostty(cwd, argv, env),
    }
}

/// herdr に workspace を 1 つ作り、その root pane でコマンドを走らせる。
fn open_herdr(
    settings: &AppSettings,
    label: &str,
    cwd: Option<&str>,
    argv: &[String],
    env: &[String],
) -> Result<String, String> {
    let herdr = locate_program("herdr", &settings.herdr_path)?;
    if !server_running(&herdr) {
        return Err("herdr サーバーが動いていません。herdr を起動してください".into());
    }

    let mut create: Vec<&str> = vec!["workspace", "create", "--label", label, "--focus"];
    if let Some(cwd) = cwd.filter(|c| !c.trim().is_empty()) {
        create.extend_from_slice(&["--cwd", cwd]);
    }
    // pane のシェルは herdr サーバーの環境を継ぐので、不足分はここで渡す。
    for pair in env {
        create.extend_from_slice(&["--env", pair]);
    }

    let stdout = output_of(&herdr, &create)?;
    let pane = parse_pane_id(&stdout)?;

    let mut run: Vec<&str> = vec!["pane", "run", &pane];
    run.extend(argv.iter().map(String::as_str));
    output_of(&herdr, &run)?;

    // herdr の TUI は別のターミナル上で動いているので、見える場所まで持ち上げる。
    if settings.focus_terminal {
        activate_ghostty();
    }

    Ok("herdr".into())
}

/// `herdr workspace create` の JSON から、コマンドを流し込む pane を取り出す。
fn parse_pane_id(stdout: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim())
        .map_err(|e| format!("herdr の応答を解釈できませんでした: {e}"))?;

    value["result"]["root_pane"]["pane_id"]
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| "herdr の応答に pane_id がありませんでした".into())
}

/// Herdr デスクトップアプリに space を 1 つ作り、その最初のターミナルで
/// コマンドを走らせる。
fn open_herdr_gui(
    settings: &AppSettings,
    label: &str,
    cwd: Option<&str>,
    argv: &[String],
    env: &[String],
) -> Result<String, String> {
    let herdr_app = locate_program("herdr-app", &settings.herdr_app_path)?;
    if !window_running(&herdr_app) {
        return Err("Herdr のウィンドウが開いていません。Herdr を起動してください".into());
    }

    // space.new には環境変数の引数が無いので、env(1) を頭に挟む形で渡す。
    let argv_json = serde_json::to_string(&with_env_prefix(argv, env))
        .map_err(|e| format!("argv を JSON にできませんでした: {e}"))?;

    let mut create: Vec<&str> = vec!["do", "space.new", "--name", label];
    if let Some(cwd) = cwd.filter(|c| !c.trim().is_empty()) {
        create.extend_from_slice(&["--folder", cwd]);
    }
    create.extend_from_slice(&["--argv", &argv_json]);

    let stdout = output_of(&herdr_app, &create)?;
    // 失敗は終了コードにも出るが、念のため応答の error も見る。
    if let Some(message) = error_message(&stdout) {
        return Err(format!("Herdr が space を作れませんでした: {message}"));
    }

    if settings.focus_terminal {
        activate_bundle(HERDR_APP_BUNDLE_ID);
    }

    Ok("herdr-gui".into())
}

/// `env` が空でなければ `/usr/bin/env KEY=VALUE … ` を頭に足した argv を返す。
fn with_env_prefix(argv: &[String], env: &[String]) -> Vec<String> {
    if env.is_empty() {
        return argv.to_vec();
    }

    let mut out = Vec::with_capacity(argv.len() + env.len() + 1);
    out.push("/usr/bin/env".to_string());
    out.extend(env.iter().cloned());
    out.extend(argv.iter().cloned());
    out
}

/// `herdr-app do` の応答に `error` があれば取り出す。
fn error_message(stdout: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).ok()?;
    value["error"].as_str().map(str::to_string)
}

/// AppleScript で Ghostty に新しいウィンドウを開かせる。
fn open_ghostty(cwd: Option<&str>, argv: &[String], env: &[String]) -> Result<String, String> {
    if !Path::new(GHOSTTY_APP).is_dir() {
        return Err(format!("Ghostty が見つかりません: {GHOSTTY_APP}"));
    }

    let command = argv
        .iter()
        .map(|a| shell_quote(a))
        .collect::<Vec<_>>()
        .join(" ");

    let mut config = format!("command:{}", applescript_string(&command));
    if let Some(cwd) = cwd.filter(|c| !c.trim().is_empty()) {
        config.push_str(&format!(
            ", initial working directory:{}",
            applescript_string(cwd)
        ));
    }
    if !env.is_empty() {
        let list = env
            .iter()
            .map(|pair| applescript_string(pair))
            .collect::<Vec<_>>()
            .join(", ");
        config.push_str(&format!(", environment variables:{{{list}}}"));
    }
    // コマンドが終わってもウィンドウを残し、出力を読めるようにする。
    config.push_str(", wait after command:true");

    let script = format!(
        "tell application \"Ghostty\"\n\
         \tnew window with configuration {{{config}}}\n\
         \tactivate\n\
         end tell"
    );

    run_detached("/usr/bin/osascript", &["-e".into(), script])?;
    Ok("ghostty".into())
}

/// `{command}` を実コマンドで置き換えて `sh -c` に渡す。
fn open_custom(
    settings: &AppSettings,
    argv: &[String],
    env: &[String],
) -> Result<String, String> {
    let template = settings.terminal_command.trim();
    if template.is_empty() {
        return Err("カスタムコマンドが設定されていません".into());
    }
    if !template.contains("{command}") {
        return Err("カスタムコマンドに {command} が含まれていません".into());
    }

    // env(1) を頭に挟む形で環境変数を足す。
    let prefix = if env.is_empty() {
        String::new()
    } else {
        let pairs = env
            .iter()
            .map(|p| shell_quote(p))
            .collect::<Vec<_>>()
            .join(" ");
        format!("/usr/bin/env {pairs} ")
    };

    let command = prefix
        + &argv
            .iter()
            .map(|a| shell_quote(a))
            .collect::<Vec<_>>()
            .join(" ");

    run_detached("/bin/sh", &["-c".into(), template.replace("{command}", &command)])?;
    Ok("custom".into())
}

/// 既に起動している Ghostty を前面に出す。未起動なら何もしない。
/// `open -a` は Apple Events を使わないので権限ダイアログが出ない。
fn activate_ghostty() {
    if !Path::new(GHOSTTY_APP).is_dir() {
        return;
    }
    let _ = std::process::Command::new("/usr/bin/open")
        .args(["-a", GHOSTTY_APP])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// bundle id で指定したアプリを前面に出す。`open -b` も Apple Events を使わない。
fn activate_bundle(bundle_id: &str) {
    let _ = std::process::Command::new("/usr/bin/open")
        .args(["-b", bundle_id])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

/// ターミナルの起動は待たずに投げっぱなしにする。終了コードだけ確認する。
fn run_detached(program: &str, args: &[String]) -> Result<(), String> {
    let out = command_for(&PathBuf::from(program))
        .args(args)
        .output()
        .map_err(|e| format!("{program} の起動に失敗しました: {e}"))?;

    if out.status.success() {
        return Ok(());
    }

    let detail = String::from_utf8_lossy(&out.stderr).trim().to_string();
    Err(format!("ターミナルを開けませんでした: {detail}"))
}

/// `herdr status server` が running を返すか。
fn server_running(herdr: &Path) -> bool {
    output_of(herdr, &["status", "server"])
        .map(|out| {
            out.lines()
                .any(|l| l.trim() == "status: running")
        })
        .unwrap_or(false)
}

/// Herdr のウィンドウが開いているか。`herdr-app status` は開いていなければ失敗する。
/// `commands` と違い、起動中のウィンドウに問い合わせるだけで新しく開くことはない。
fn window_running(herdr_app: &Path) -> bool {
    output_of(herdr_app, &["status"]).is_ok()
}

/// AppleScript の文字列リテラルにする。
fn applescript_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '\\' || ch == '"' {
            out.push('\\');
        }
        out.push(ch);
    }
    out.push('"');
    out
}

/// シェルに渡しても 1 語として扱われる形にする。
fn shell_quote(s: &str) -> String {
    let safe = !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@,+".contains(c));

    if safe {
        s.to_string()
    } else {
        format!("'{}'", s.replace('\'', r"'\''"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pane_id_を取り出せる() {
        let json = r#"{"id":"cli:workspace:create","result":{
            "root_pane":{"pane_id":"w52:p1","tab_id":"w52:t1","workspace_id":"w52"},
            "type":"workspace_created",
            "workspace":{"label":"aws-ro","workspace_id":"w52"}}}"#;

        assert_eq!(parse_pane_id(json).unwrap(), "w52:p1");
    }

    #[test]
    fn pane_id_が無ければエラー() {
        assert!(parse_pane_id(r#"{"result":{}}"#).is_err());
        assert!(parse_pane_id("not json").is_err());
    }

    #[test]
    fn 空白や引用符を含む引数を安全に引用する() {
        assert_eq!(shell_quote("/opt/homebrew/bin/sbx"), "/opt/homebrew/bin/sbx");
        assert_eq!(shell_quote("my-project"), "my-project");
        assert_eq!(shell_quote("my project"), "'my project'");
        assert_eq!(shell_quote("it's"), r"'it'\''s'");
        assert_eq!(shell_quote(""), "''");
    }

    #[test]
    fn applescript_の引用符とバックスラッシュをエスケープする() {
        assert_eq!(applescript_string("sbx run"), "\"sbx run\"");
        assert_eq!(applescript_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(applescript_string("a\\b"), "\"a\\\\b\"");
    }

    fn settings(terminal: &str) -> AppSettings {
        AppSettings {
            terminal: terminal.into(),
            ..AppSettings::default()
        }
    }

    #[test]
    fn 明示指定はautoより優先される() {
        // herdr や Ghostty の有無によらず、設定した値がそのまま使われる。
        assert_eq!(info(&settings("ghostty")).effective, "ghostty");
        assert_eq!(info(&settings("custom")).effective, "custom");
        assert_eq!(info(&settings("herdr")).effective, "herdr");
        assert_eq!(info(&settings("herdr-gui")).effective, "herdr-gui");
    }

    #[test]
    fn autoはherdrの稼働状況で決まる() {
        let i = info(&settings("auto"));
        let expected = if i.herdr_app_running {
            "herdr-gui"
        } else if i.herdr_running {
            "herdr"
        } else {
            "ghostty"
        };
        assert_eq!(i.effective, expected);
    }

    #[test]
    fn 環境変数はenvコマンドで前置きする() {
        let argv = vec!["sbx".to_string(), "run".to_string()];

        assert_eq!(with_env_prefix(&argv, &[]), argv);
        assert_eq!(
            with_env_prefix(&argv, &["A=1".to_string(), "B=2".to_string()]),
            vec!["/usr/bin/env", "A=1", "B=2", "sbx", "run"]
        );
    }

    #[test]
    fn herdr_appの応答からerrorを取り出す() {
        assert_eq!(
            error_message(r#"{"error":"no such space"}"#).as_deref(),
            Some("no such space")
        );
        assert_eq!(error_message(r#"{"space":"sp1","terminal":"t1"}"#), None);
        assert_eq!(error_message("not json"), None);
    }
}

