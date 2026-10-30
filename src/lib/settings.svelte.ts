import { getSettings, saveSettings } from './api';
import type { AppSettings } from './types';

/** load() が終わるまでの仮の値。実際の既定値は Rust 側が持つ。 */
const EMPTY: AppSettings = {
  kits_dir: '',
  sbx_path: '',
  workspace_root: '',
  terminal: 'auto',
  herdr_path: '',
  terminal_command: '',
  focus_terminal: true
};

/** アプリ設定の単一のソース。各画面はここを読み、設定画面だけが書き換える。 */
class SettingsStore {
  current = $state<AppSettings>({ ...EMPTY });
  loaded = $state(false);

  async load() {
    this.current = await getSettings();
    this.loaded = true;
  }

  async save(next: AppSettings) {
    this.current = await saveSettings(next);
  }
}

export const settings = new SettingsStore();
