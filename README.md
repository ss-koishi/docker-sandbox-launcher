# Docker Sandbox Launcher

[Docker Sandboxes](https://docs.docker.com/ai/sandboxes/) の `sbx` CLI を GUI から操作する macOS 向けアプリです。
Tauri v2 + SvelteKit で作ってあり、配布物はシステムの WKWebView を使うので数 MB 程度に収まります。

## できること

- **サンドボックスの一覧** — `sbx ls` の内容（状態・エージェント・workspace・作成日時）をカード表示
- **作成** — エージェント、sandbox 名、workspace、kits（mixin）を選んで `sbx create` を実行。進捗は逐次表示される
- **削除** — 確認ダイアログののち `sbx rm --force`
- **ターミナルで開く** — 一覧から `sbx run --name <名前>` を herdr / Ghostty で実行
- **kits ディレクトリの設定** — kit を置いてあるディレクトリを GUI で指定し、配下の kit を一覧・spec を閲覧

## インストール

```sh
brew install --cask ss-koishi/tap/docker-sandbox-launcher
```

Developer ID 署名・公証は行っていないため、cask の `postflight` が quarantine 属性を外します。

## 必要なもの

- macOS 11 以降
- [`sbx` CLI](https://docs.docker.com/ai/sandboxes/install/) — `brew install --cask docker/tap/sbx`
- 開発するなら Node.js と Rust ツールチェーン

## 使い方

初回は **設定** タブで以下を確認してください。既定値のままで足りることが多いはずです。

| 設定 | 既定値 | 用途 |
| --- | --- | --- |
| kits ディレクトリ | `~/.sbx/kits` | 直下の「`spec.yml` を持つサブディレクトリ」を kit として列挙する |
| workspace の作成先 | `~/sbx-workspaces` | 作成画面の「〜を作成して使う」が掘る親ディレクトリ |
| sbx のパス | （自動検出） | 空なら PATH と Homebrew の標準的な場所から探す |
| sbx に渡す環境変数 | （空） | `KEY=VALUE` を 1 行ずつ。下記参照 |
| ターミナル連携 | 自動 | 端末ボタンの飛び先。下記参照 |

### sbx に渡す環境変数

エージェントによってはホスト側の環境変数を要求します。たとえば `claude-vertex` は
`ANTHROPIC_VERTEX_PROJECT_ID` が無いとサンドボックスを作る前に止まります。

```
error: claude-vertex requires a Google Cloud project to be selected.
Export ANTHROPIC_VERTEX_PROJECT_ID=<project-id> ... in your shell and re-run.
```

**これは kit の `spec.yml` や `sbxenv.yaml` の `env:` では代用できません。** どちらも
サンドボックス側のスコープで、このチェックはホストの sbx プロセスが行うためです
（[環境ファイルのドキュメント](https://docs.docker.com/ai/sandboxes/configuration/environment-files/)
にも「環境ファイルの `env` 値はホストコマンドには渡されない」と明記されています）。
`sbxenv.yaml` に `env:` を書いたうえでホストの変数を消して `sbx env create` すると、
同じエラーで止まることを確認しています。

この欄に書いた値は、作成・削除・ターミナル起動を含むすべての sbx 呼び出しに渡ります。
ターミナル経路では、その先の `sbx run` にも届くよう herdr の `--env` や Ghostty の
`environment variables` として引き継ぎます。

### kit ごとの host-env

Vertex の設定のように「特定の kit を使うときだけ必要」な変数は、グローバル欄ではなく
**kit ディレクトリの `host-env` ファイル**に置けます。

```
~/.sbx/kits/claude-code/
├── spec.yml
└── host-env          ← このアプリが読む
```

```sh
# host-env
# claude-vertex がホスト側で要求する
ANTHROPIC_VERTEX_PROJECT_ID=my-gcp-project
CLOUD_ML_REGION=global
```

`spec.yml` に書けないのは、未知のキーを `sbx kit validate` が拒否するためです。

```
error: invalid spec.yaml: field hostEnvironment not found in type spec.specFileV2
```

`host-env` は sbx が関知しないただのファイルなので、置いても `sbx kit validate` は通ります。

適用されるタイミングは次のとおりです。優先順位は **グローバル欄 → kit の host-env** で、
後者が勝ちます。複数の kit が同じキーを持つ場合は、選択順の後ろが勝ちます。

| 操作 | kit の特定方法 |
| --- | --- |
| 作成 | 作成画面で選んだ kit |
| ターミナルで開く | `sbx inspect <名前>` が返す `kits` |

Kits 画面と作成画面には、その kit がどのキーを渡すかがチップで表示されます（値は出しません）。

### ターミナル連携

sandbox 一覧の端末ボタンを押すと `sbx run --name <名前>` が実行されます。飛び先は 3 つから選べ、既定の「自動」は herdr サーバーが動いていれば herdr、そうでなければ Ghostty を使います。

**herdr** — ソケット API で sandbox 名のワークスペースを作り、その pane でコマンドを走らせます。

```sh
herdr workspace create --label <名前> --cwd <workspace> --focus   # → root_pane.pane_id
herdr pane run <pane_id> <sbx のパス> run --name <名前>
```

herdr のサーバーが pane を持つので、ターミナルを閉じてもセッションは残ります。herdr の TUI 自体は別のターミナル上で動くため、既定では開いたあと Ghostty を前面に出します（設定で切れます）。

**Ghostty** — AppleScript で既存インスタンスに新しいウィンドウを開きます。

```applescript
tell application "Ghostty"
  new window with configuration {command:"…", initial working directory:"…", wait after command:true}
  activate
end tell
```

macOS では `ghostty -e` が使えず、公式が案内する `open -na Ghostty.app --args -e …` は呼ぶたびに別インスタンスが増えるため、AppleScript を使っています。初回に「Docker Sandbox Launcher が Ghostty を制御しようとしています」という許可ダイアログが出ます（`src-tauri/Info.plist` の `NSAppleEventsUsageDescription` がその説明文です）。

**カスタムコマンド** — `{command}` を含むテンプレートが `sh -c` に渡されます。

```sh
open -na Ghostty.app --args -e {command}
```

### workspace の指定

作成画面では primary workspace を 1 つと、追加のホストディレクトリを複数指定できます。
追加分は個別に読み取り専用（`:ro`）を切り替えられます。サンドボックス内にはホストと同じ絶対パスでマウントされます。

primary を空にすると bind mount なしのサンドボックスになります。

### kits

`--kit` に渡せるのは mixin だけなので、`kind: workload` の kit は選択肢に出しません。
kit の `requires.agent` が選択中のエージェントと食い違う場合は警告を出します（作成自体は試行できます）。

## 開発

```sh
npm install
npm run tauri dev      # 開発起動
npm run check          # Svelte / TypeScript の型チェック
cargo test --manifest-path src-tauri/Cargo.toml

# 配布と同じ universal バイナリを作る
rustup target add aarch64-apple-darwin x86_64-apple-darwin
CI=true npm run tauri build -- --target universal-apple-darwin
```

`CI=true` を付けているのは、DMG 作成時にウィンドウの体裁を整える Finder の AppleScript を
飛ばすためです。付けないとターミナルに Finder の自動化許可がない環境で
`AppleEventがタイムアウトしました (-1712)` で失敗します。CI 上では `CI` が自動で立つので不要です。

Homebrew の rustup は keg-only なので、`cargo` を PATH に通しておく必要があります。

```fish
# ~/.config/fish/config.fish
fish_add_path /opt/homebrew/opt/rustup/bin
```

## リリースと配布

`v0.1.0` のようなタグを push すると `.github/workflows/release.yml` が動き、

1. タグと `src-tauri/tauri.conf.json` の `version` の一致を確認する
2. 型チェックと `cargo test` を走らせる
3. `--target universal-apple-darwin` でビルドし、GitHub Release に `.dmg` を添付する
4. `packaging/cask.rb.tmpl` に version と sha256 を埋めて、tap の `Casks/` へ push する

という流れになります。tap が未作成でも 3 までは成功します。

### 初回だけ必要な準備

```sh
# 1. アプリ本体のリポジトリ（public）
git remote add origin git@github.com:ss-koishi/docker-sandbox-launcher.git
git push -u origin main

# 2. tap のリポジトリ（public、名前は homebrew- 始まりが必須）
#    中身は空でよい。ワークフローが Casks/ を作る
```

GitHub Actions の `GITHUB_TOKEN` は別リポジトリへ push できないので、tap 更新用に
**`TAP_GITHUB_TOKEN`** を Secrets に登録します。`ss-koishi/homebrew-tap` への `contents: write`
を持つ fine-grained personal access token を作ってください。

### 署名・公証を入れる場合

Apple Developer Program に加入して Developer ID Application 証明書を用意したら、次の Secrets を
足して `tauri-apps/tauri-action` のステップに `env` として渡すだけで有効になります。

| Secret | 内容 |
| --- | --- |
| `APPLE_CERTIFICATE` | `.p12` を base64 にしたもの |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` のパスワード |
| `APPLE_SIGNING_IDENTITY` | `Developer ID Application: ...` |
| `APPLE_ID` / `APPLE_PASSWORD` / `APPLE_TEAM_ID` | 公証 (notarytool) 用 |

そのうえで `packaging/cask.rb.tmpl` の `postflight` ブロックを削除します。

## 構成

```
src-tauri/src/
  sbx.rs        sbx CLI のラッパー（実行ファイル探索・JSON 取得・出力ストリーミング）
  terminal.rs   herdr / Ghostty / カスタムコマンドへの橋渡し
  commands.rs   フロントから呼ぶ Tauri コマンド
  kits.rs       kits ディレクトリの走査と spec の読み取り
  model.rs      フロントと共有するデータ構造
  settings.rs   アプリ設定の永続化
src/lib/
  api.ts        invoke の型付きラッパー
  types.ts      Rust 側と対応する型
  components/   画面
```

`rm` / `stop` / `prune` は `commands.rs` の `LIFECYCLE_VERBS` にまとめてあり、
共通の `run_lifecycle` コマンドを通ります。現在 UI に出しているのは `rm` だけですが、
`stop` と `prune` は Rust 側を触らずフロントにボタンを足すだけで有効にできます。
