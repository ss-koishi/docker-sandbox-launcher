//! sandbox をホストのターミナルで開く。
//!
//! 連携先は 3 つ。
//!
//! - **herdr**: ソケット API で workspace を作り、その root pane でコマンドを走らせる。
//!   herdr サーバーが持つので、ターミナルを閉じてもセッションは生き残る。
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

/// 設定画面に出す、連携先の検出結果。
#[derive(Debug, Clone, Serialize)]
pub struct TerminalInfo {
    /// 実際に使われる連携先（auto を解決した結果）。
    pub effective: String,
    pub herdr_running: bool,
    pub herdr_path: Option<String>,
    pub ghostty_installed: bool,
}

pub fn info(settings: &AppSettings) -> TerminalInfo {
    let herdr = locate_program("herdr", &settings.herdr_path).ok();
    let herdr_running = herdr.as_deref().is_some_and(server_running);
    let ghostty_installed = Path::new(GHOSTTY_APP).is_dir();

    let effective = match settings.terminal.as_str() {
        "herdr" | "ghostty" | "custom" => settings.terminal.clone(),
        // auto: herdr が動いていれば herdr、そうでなければ Ghostty。
        _ if herdr_running => "herdr".into(),
        _ => "ghostty".into(),
    };

    TerminalInfo {
        effective,
        herdr_running,
        herdr_path: herdr.map(|p| p.display().to_string()),
        ghostty_installed,
    }
}

/// `argv` を `label` という名前でターミナルに開く。使った連携先の名前を返す。
pub fn open(
    settings: &AppSettings,
    label: &str,
    cwd: Option<&str>,
    argv: &[String],
) -> Result<String, String> {
    if argv.is_empty() {
        return Err("実行するコマンドが空です".into());
    }

    // ターミナルの中で走る sbx もホスト扱いなので、同じ環境変数を渡す。
    let env = settings.env_kv();

    match info(settings).effective.as_str() {
        "herdr" => open_herdr(settings, label, cwd, argv, &env),
        "custom" => open_custom(settings, argv, &env),
        _ => open_ghostty(cwd, argv, &env),
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
    }

    #[test]
    fn autoはherdrの稼働状況で決まる() {
        let i = info(&settings("auto"));
        assert_eq!(i.effective, if i.herdr_running { "herdr" } else { "ghostty" });
    }
}
