/** `sbx ls --format json` の 1 要素。 */
export type Sandbox = {
  name: string;
  id: string;
  agent: string;
  status: string;
  /** 先頭が primary workspace。読み取り専用マウントは `/path:ro` の形。 */
  workspaces: string[];
  created_at: string | null;
  last_used_at: string | null;
  workspace_missing: boolean;
};

/** kits ディレクトリ配下で見つかったローカル kit。 */
export type Kit = {
  id: string;
  path: string;
  name: string | null;
  display_name: string | null;
  kind: string | null;
  requires_agent: string | null;
  /** kit ディレクトリの host-env が定義している環境変数のキー。 */
  host_env_keys: string[];
  /** spec の `credentials[].service`。作成前に `sbx secret set` が要るもの。 */
  required_secrets: string[];
  /** secret を用意するコマンド（kit ディレクトリの secret-setup）。 */
  secret_setup: string | null;
  spec_file: string | null;
  error: string | null;
};

/** `required_secrets` のうち、まだ登録されていないもの。 */
export function missingSecrets(kit: Kit, stored: string[]): string[] {
  return kit.required_secrets.filter((name) => !stored.includes(name));
}

export type WorkspaceMount = {
  path: string;
  readonly: boolean;
};

export type CreateRequest = {
  agent: string;
  name: string;
  /** 先頭が primary workspace。 */
  workspaces: WorkspaceMount[];
  /** `--kit` に渡す mixin 参照。 */
  kits: string[];
};

/** ターミナル連携先。`auto` は herdr が動いていれば herdr、無ければ Ghostty。 */
export type TerminalTarget = 'auto' | 'herdr' | 'ghostty' | 'custom';

export type AppSettings = {
  kits_dir: string;
  sbx_path: string;
  workspace_root: string;
  terminal: TerminalTarget;
  herdr_path: string;
  /** `terminal` が `custom` のときのテンプレート。`{command}` を置換する。 */
  terminal_command: string;
  focus_terminal: boolean;
};

export type TerminalInfo = {
  /** auto を解決した結果、実際に使われる連携先。 */
  effective: Exclude<TerminalTarget, 'auto'>;
  herdr_running: boolean;
  herdr_path: string | null;
  ghostty_installed: boolean;
};

export const TERMINAL_LABELS: Record<Exclude<TerminalTarget, 'auto'>, string> = {
  herdr: 'herdr',
  ghostty: 'Ghostty',
  custom: 'カスタムコマンド'
};

export type SbxInfo = {
  path: string;
  version: string;
};

export type LogLine = {
  stream: 'stdout' | 'stderr';
  text: string;
};

/** Rust 側の `LIFECYCLE_VERBS` に載っている動詞。 */
export type LifecycleVerb = 'rm' | 'stop' | 'prune';

/** kit の表示名。displayName → name → ディレクトリ名の順。 */
export function kitLabel(kit: Kit): string {
  return kit.display_name ?? kit.name ?? kit.id;
}

/** `--kit` に渡せるのは mixin だけ。kind 未記載のものは判断できないので許可する。 */
export function isMixin(kit: Kit): boolean {
  return kit.kind !== 'workload';
}

/**
 * host-env の本文から、実際に効くキーを取り出す。
 * Rust 側の `parse_env` と同じ規則（空行と `#` 始まりを無視、最初の `=` で分割）。
 */
export function parseEnvKeys(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line !== '' && !line.startsWith('#'))
    .map((line) => {
      // `=` を含まない行は変数ではないので捨てる。
      const at = line.indexOf('=');
      return at < 0 ? '' : line.slice(0, at).trim();
    })
    .filter((key) => key !== '');
}
