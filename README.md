# Docker Sandbox Launcher

[Docker Sandboxes](https://docs.docker.com/ai/sandboxes/) の `sbx` CLI を GUI から操作する macOS 向けアプリです。
Tauri v2 + SvelteKit で作ってあり、配布物はシステムの WKWebView を使うので数 MB 程度に収まります。

## できること

- **サンドボックスの一覧** — `sbx ls` の内容（状態・エージェント・workspace・作成日時）をカード表示
- **作成** — エージェント、sandbox 名、workspace、kits（mixin）を選んで `sbx create` を実行。進捗は逐次表示される
- **削除** — 確認ダイアログののち `sbx rm --force`
- **ターミナルで開く** — 一覧から `sbx run --name <名前>` を herdr / Ghostty で実行
- **kits ディレクトリの設定** — kit を置いてあるディレクトリを GUI で指定し、配下の kit を一覧・spec を閲覧

## 必要なもの

- macOS
- [`sbx` CLI](https://docs.docker.com/ai/sandboxes/install/)（`brew install sbx` 等）
- 開発するなら Node.js と Rust ツールチェーン

## 使い方

初回は **設定** タブで以下を確認してください。既定値のままで足りることが多いはずです。

| 設定 | 既定値 | 用途 |
| --- | --- | --- |
| kits ディレクトリ | `~/.sbx/kits` | 直下の「`spec.yml` を持つサブディレクトリ」を kit として列挙する |
| workspace の作成先 | `~/sbx-workspaces` | 作成画面の「〜を作成して使う」が掘る親ディレクトリ |
| sbx のパス | （自動検出） | 空なら PATH と Homebrew の標準的な場所から探す |
| ターミナル連携 | 自動 | 端末ボタンの飛び先。下記参照 |

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
npm run tauri build    # .app と .dmg を作る
npm run check          # Svelte / TypeScript の型チェック
```

Homebrew の rustup は keg-only なので、`cargo` を PATH に通しておく必要があります。

```fish
# ~/.config/fish/config.fish
fish_add_path /opt/homebrew/opt/rustup/bin
```

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
