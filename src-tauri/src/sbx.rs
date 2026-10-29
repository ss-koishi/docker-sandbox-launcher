//! `sbx` CLI の薄いラッパー。
//!
//! GUI から新しい操作を足すときにここを触らなくて済むよう、入り口は
//! 「JSON を取る」「ライフサイクル動詞を投げる」「出力を流す」の 3 つだけにしてある。

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;

use serde::de::DeserializeOwned;

/// Finder から起動された .app は PATH が `/usr/bin:/bin:/usr/sbin:/sbin` しか
/// 持たないので、sbx 本体も子プロセスの PATH も自前で補う。
const EXTRA_PATHS: &[&str] = &[
    "/opt/homebrew/bin",
    "/usr/local/bin",
    "/opt/homebrew/sbin",
    "/usr/local/sbin",
];

pub struct Sbx {
    program: PathBuf,
}

impl Sbx {
    /// 設定で明示されたパスを最優先し、なければ既知の場所を順に探す。
    pub fn locate(configured: &str) -> Result<Self, String> {
        let configured = configured.trim();
        if !configured.is_empty() {
            let path = PathBuf::from(configured);
            if !is_executable(&path) {
                return Err(format!(
                    "設定された sbx のパスが実行できません: {configured}"
                ));
            }
            return Ok(Self { program: path });
        }

        for dir in search_dirs() {
            let candidate = dir.join("sbx");
            if is_executable(&candidate) {
                return Ok(Self {
                    program: candidate,
                });
            }
        }

        Err(format!(
            "sbx が見つかりませんでした。設定画面でパスを指定してください（探索先: {}）",
            search_dirs()
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }

    pub fn program(&self) -> &Path {
        &self.program
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.env("PATH", augmented_path(self.program.parent()));
        // サブコマンドが対話的な確認を出さないようにしておく。
        cmd.env("NO_COLOR", "1");
        cmd.stdin(Stdio::null());
        cmd
    }

    /// 標準出力を文字列で受け取る。終了コードが 0 以外なら stderr をエラーにする。
    pub fn output(&self, args: &[&str]) -> Result<String, String> {
        let out = self
            .command()
            .args(args)
            .output()
            .map_err(|e| format!("sbx {} の起動に失敗しました: {e}", args.join(" ")))?;

        if !out.status.success() {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let detail = if stderr.is_empty() { stdout } else { stderr };
            return Err(format!("sbx {} が失敗しました: {detail}", args.join(" ")));
        }

        Ok(String::from_utf8_lossy(&out.stdout).to_string())
    }

    /// `--format json` を付けて呼び、結果を型に落とす。
    pub fn json<T: DeserializeOwned>(&self, args: &[&str]) -> Result<T, String> {
        let mut full = args.to_vec();
        full.extend_from_slice(&["--format", "json"]);
        let stdout = self.output(&full)?;
        // 対象が 0 件のとき、サブコマンドによっては何も出力しない。
        let stdout = if stdout.trim().is_empty() {
            "null".to_string()
        } else {
            stdout
        };
        serde_json::from_str(&stdout)
            .map_err(|e| format!("sbx {} の JSON を解釈できませんでした: {e}", args.join(" ")))
    }

    /// `rm` / `stop` / `prune` のような、対象を取って終わるだけの操作。
    ///
    /// 新しい動詞を GUI に足すときは [`crate::commands::LIFECYCLE_VERBS`] に
    /// 名前を加えるだけでよい。
    pub fn lifecycle(&self, verb: &str, targets: &[String], flags: &[&str]) -> Result<String, String> {
        let mut args: Vec<&str> = vec![verb];
        args.extend_from_slice(flags);
        args.extend(targets.iter().map(|s| s.as_str()));
        self.output(&args)
    }

    /// 出力を 1 行ずつコールバックに流しながら実行し、終了コードを返す。
    /// create のようにイメージ取得で分単位かかる操作に使う。
    pub fn stream<F>(&self, args: &[String], mut on_line: F) -> Result<i32, String>
    where
        F: FnMut(LogLine),
    {
        let mut child = self
            .command()
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("sbx {} の起動に失敗しました: {e}", args.join(" ")))?;

        let (tx, rx) = mpsc::channel::<LogLine>();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let readers: Vec<_> = [(stdout.map(Pipe::Out), ()), (stderr.map(Pipe::Err), ())]
            .into_iter()
            .filter_map(|(pipe, _)| pipe)
            .map(|pipe| {
                let tx = tx.clone();
                std::thread::spawn(move || pipe.pump(tx))
            })
            .collect();
        drop(tx);

        for line in rx {
            on_line(line);
        }
        for reader in readers {
            let _ = reader.join();
        }

        let status = child
            .wait()
            .map_err(|e| format!("sbx の終了を待てませんでした: {e}"))?;
        Ok(status.code().unwrap_or(-1))
    }
}

/// ストリーミング中の 1 行。
#[derive(Debug, Clone, serde::Serialize)]
pub struct LogLine {
    pub stream: &'static str,
    pub text: String,
}

enum Pipe {
    Out(std::process::ChildStdout),
    Err(std::process::ChildStderr),
}

impl Pipe {
    fn pump(self, tx: mpsc::Sender<LogLine>) {
        let (stream, reader): (&'static str, Box<dyn BufRead>) = match self {
            Pipe::Out(s) => ("stdout", Box::new(BufReader::new(s))),
            Pipe::Err(s) => ("stderr", Box::new(BufReader::new(s))),
        };
        for line in reader.lines() {
            let Ok(text) = line else { break };
            if tx.send(LogLine { stream, text }).is_err() {
                break;
            }
        }
    }
}

fn search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(path) = std::env::var("PATH") {
        dirs.extend(path.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    }
    dirs.extend(EXTRA_PATHS.iter().map(PathBuf::from));
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(&home).join(".local/bin"));
        dirs.push(PathBuf::from(&home).join("bin"));
    }
    dirs.dedup();
    dirs
}

/// 子プロセス用の PATH。sbx 自身が docker などを呼ぶので Homebrew も通しておく。
fn augmented_path(program_dir: Option<&Path>) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(dir) = program_dir {
        parts.push(dir.display().to_string());
    }
    parts.extend(EXTRA_PATHS.iter().map(|s| s.to_string()));
    if let Ok(path) = std::env::var("PATH") {
        parts.extend(path.split(':').filter(|s| !s.is_empty()).map(String::from));
    }
    parts.extend(
        ["/usr/bin", "/bin", "/usr/sbin", "/sbin"]
            .iter()
            .map(|s| s.to_string()),
    );

    let mut seen = std::collections::HashSet::new();
    parts.retain(|p| seen.insert(p.clone()));
    parts.join(":")
}

fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}
