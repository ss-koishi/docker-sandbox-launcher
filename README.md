# Docker Sandbox Launcher

[Docker Sandboxes](https://docs.docker.com/ai/sandboxes/) の `sbx` CLI を GUI から操作する macOS 向けアプリです。
Tauri v2 + SvelteKit で作ってあり、配布物はシステムの WKWebView を使うので数 MB 程度に収まります。

## できること

- **サンドボックスの一覧** — `sbx ls` の内容（状態・エージェント・workspace・作成日時）をカード表示
- **作成** — エージェント、sandbox 名、workspace、kits（mixin）を選んで `sbx create` を実行。進捗は逐次表示される
- **削除** — 確認ダイアログののち `sbx rm --force`
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
