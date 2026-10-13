import { invoke } from '@tauri-apps/api/core';
const native = new Set([
  'set_titlebar_controls',
  'get_window_layout',
  'set_titlebar_color',
  'show_main_window',
  'frame_action',
  'frame_frontend_report',
]);
export function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  if (native.has(command)) return invoke<T>(command, args);
  // No account/Vault/business commands are registered in the isolated native host.
  return Promise.resolve(null as T);
}
