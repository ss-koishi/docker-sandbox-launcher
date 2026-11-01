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
| ターミナル連携 | 自動 | 端末ボタンの飛び先。下記参照 |

### kit ごとの host-env

エージェントによってはホスト側の環境変数を要求します。たとえば `claude-vertex` は
`ANTHROPIC_VERTEX_PROJECT_ID` が無いとサンドボックスを作る前に止まります。

```
error: claude-vertex requires a Google Cloud project to be selected.
Export ANTHROPIC_VERTEX_PROJECT_ID=<project-id> ... in your shell and re-run.
```

こうした変数は、**kit ディレクトリの `host-env` ファイル**に置きます。その kit を使う
ときだけホストの sbx プロセスへ渡されます。

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

**`spec.yml` や `sbxenv.yaml` の `env:` では代用できません。** どちらもサンドボックス側の
スコープで、このチェックはホストの sbx プロセスが行うためです
（[環境ファイルのドキュメント](https://docs.docker.com/ai/sandboxes/configuration/environment-files/)
にも「環境ファイルの `env` 値はホストコマンドには渡されない」と明記されています）。
`sbxenv.yaml` に `env:` を書いたうえでホストの変数を消して `sbx env create` すると、
同じエラーで止まることを確認しています。`spec.yml` に独自キーを足す案も通りません。

```
error: invalid spec.yaml: field hostEnvironment not found in type spec.specFileV2
```

`host-env` は sbx が関知しないただのファイルなので、置いても `sbx kit validate` は通ります。

適用されるタイミングは次のとおりです。複数の kit が同じキーを持つ場合は、選択順の後ろが
勝ちます。

| 操作 | kit の特定方法 |
| --- | --- |
| 作成 | 作成画面で選んだ kit |
| ターミナルで開く | `sbx inspect <名前>` が返す `kits` |

ターミナル経路では、その先の `sbx run` にも届くよう herdr の `--env` や Ghostty の
`environment variables` として引き継ぎます。

Kits 画面と作成画面には、その kit がどのキーを渡すかがチップで表示されます（値は出しません）。

なお kit を 1 つも指定せずエージェントだけを選んだ場合、この仕組みは効きません。その構成で
ホスト側の環境変数が要るときは、シェルの設定で export してください。

### kit ごとの secret-setup

kit によっては、作成前にホスト側で済ませておく手順があります。アプリはこれを
**確認するだけ**で、代わりに実行はしません。手順とコマンドは kit ディレクトリの
**`secret-setup`** ファイルに置きます。

確認できる前提条件は 3 種類です。コメント形式のディレクティブで宣言するので、
ファイルをそのままシェルに貼っても動きます。

| ディレクティブ | 確認方法 | 例 |
| --- | --- | --- |
| `#secret: <名前>` | `sbx secret ls` | `#secret: mcp:datadog:pat` |
| `#mcp: <名前>` | `sbx mcp ls`。作成時に `--static-mcp` で渡す | `#mcp: datadog` |
| `#file: <パス>` | ホストにそのファイルがあるか（`~` 展開あり） | `#file: ~/.config/gcloud/application_default_credentials.json` |

`spec.yml` の `credentials[].service` も自動で `#secret:` と同じ扱いになり、重複は除かれます。

```
~/.sbx/kits/gcp/
├── spec.yml        credentials: [{ service: gcp, ... }]
└── secret-setup    ← 未設定のときにアプリが表示する
```

```sh
# secret-setup
sbx secret set gcp --command "gcloud auth application-default print-access-token \
  --impersonate-service-account=... --lifetime=50m" --refresh 45m
```

未達の前提があると、Kits 画面と作成ダイアログに警告とこのコマンドが出て、コピーボタンで
取れます。`secret-setup` が無い kit には `sbx secret set <名前>` を表示します。

#### Vertex の ADC

`claude-code` キットは `#file:` でホストの ADC を見ています。Vertex の認証はここから
来るためです。コンテナ内にも ADC は置かれますが `refresh_token` は `1//sbx-vertex...`
というダミーで、本物は入りません。コンテナが `oauth2.googleapis.com` にトークンを要求
すると sbx のプロキシが OAuth を横取りし、ホストの ADC で差し替えて
`aiplatform.googleapis.com` への通信だけを通します。

これは `gcp` キットの secret とは別系統です。`SBX_GCP_TOKEN` が注入されるのは
`logging` / `monitoring` / `container` / `sqladmin` の 4 ドメインだけで、`aiplatform` は
意図的に除外されています。

#### MCP ゲートウェイの結線

sbx は sandbox ごとに MCP ゲートウェイを立て、`--static-mcp` や `sbx mcp load` で
入れたサーバーを 1 つの接続にまとめます。ドキュメントは
「supported agent integrations read the gateway URL and register it with the agent」
としていますが、**sbx v0.50.0 + Claude Code 2.1.306 ではこの登録が行われません**。

- claude のバイナリに `MCP_GATEWAY_URL` の参照が 0 件
- sbx が claude に渡す引数は `--dangerously-skip-permissions` のみ
- `~/.claude.json` にも `~/.mcp.json` にもゲートウェイの記載なし

ゲートウェイ自体は正常で、`initialize` を投げれば応答します。そこで
`~/.sbx/kits/claude-code` の `setup.startup` が `~/.claude.json` に
`mcpServers.mcp-gateway` を追記して、最後の 1 ステップだけ補っています。
sbx 側が直ったらそのブロックは削除できます。

エージェントからは `datadog` ではなく **`mcp-gateway`** という 1 つのサーバーとして
見える点に注意してください。

#### 確認の限界

いずれも**有無しか見ません**。

- `sbx secret ls` は有効期限を返しません。ただし `--command` 方式の secret は
  `--refresh` に従って sbx 自身が取り直すので、実用上は有無で足ります
- `#file:` は存在するかだけで、ADC の失効や権限不足は判定できません。Vertex が 401 を
  返すときは `gcloud auth application-default login` をやり直してください

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
#    README などで 1 コミット置いて main を作っておくこと。
#    空のままだと branch が無く、リリース時の checkout が
#      fatal: couldn't find remote ref refs/heads/main
#    で失敗する。Casks/ はワークフローが作る。
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
