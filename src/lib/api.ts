import { Channel, invoke } from '@tauri-apps/api/core';
import type {
  AppSettings,
  CreateRequest,
  Kit,
  LifecycleVerb,
  LogLine,
  Sandbox,
  SbxInfo
} from './types';

export const getSettings = () => invoke<AppSettings>('get_settings');

export const saveSettings = (settings: AppSettings) =>
  invoke<AppSettings>('save_settings', { settings });

export const detectSbx = () => invoke<SbxInfo>('detect_sbx');

export const listSandboxes = () => invoke<Sandbox[]>('list_sandboxes');

export const listAgents = () => invoke<string[]>('list_agents');

export const listKits = () => invoke<Kit[]>('list_kits');

export const readKitSpec = (path: string) => invoke<string>('read_kit_spec', { path });

export const ensureWorkspaceDir = (name: string) =>
  invoke<string>('ensure_workspace_dir', { name });

export const pathExists = (path: string) => invoke<boolean>('path_exists', { path });

/** 進捗を 1 行ずつ `onLog` に流しながら sandbox を作る。 */
export function createSandbox(request: CreateRequest, onLog: (line: LogLine) => void) {
  const channel = new Channel<LogLine>();
  channel.onmessage = onLog;
  return invoke<void>('create_sandbox', { request, onLog: channel });
}

/**
 * rm / stop / prune の共通入口。
 * stop や prune を画面に足すときは、ここを呼ぶボタンを置くだけでよい。
 */
export const runLifecycle = (verb: LifecycleVerb, targets: string[]) =>
  invoke<string>('run_lifecycle', { verb, targets });

export const removeSandboxes = (names: string[]) => runLifecycle('rm', names);

/** Tauri が投げてくるエラーは文字列なので、表示用に整える。 */
export function errorMessage(e: unknown): string {
  if (typeof e === 'string') return e;
  if (e instanceof Error) return e.message;
  return String(e);
}
