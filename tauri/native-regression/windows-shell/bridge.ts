import { invoke } from '@tauri-apps/api/core';
import {
  PUBLIC_ACCOUNT_ID,
  publicObjectData,
  publicHistory,
  publicSnapshot,
  publicAttachments,
  publicAttachmentText,
} from './inspectorData';
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
  if (command === 'object_get' && args?.accountId === PUBLIC_ACCOUNT_ID) {
    return Promise.resolve(publicObjectData(String(args.objectId)) as T);
  }
  if (command === 'snapshot_list')
    return Promise.resolve(publicHistory(String(args?.objectId)) as T);
  if (command === 'snapshot_get_data')
    return Promise.resolve(publicSnapshot(String(args?.snapshotId)) as T);
  if (command === 'attachment_list')
    return Promise.resolve(
      (args?.showDeleted ? [] : publicAttachments(String(args?.objectId))) as T,
    );
  if (command === 'fs_read_file_as_text') {
    const text = publicAttachmentText(String(args?.path));
    return text === null
      ? Promise.reject(new Error('Only generated public fixture text is available'))
      : Promise.resolve(text as T);
  }
  if (command === 'template_list' || command === 'object_field_suggestions')
    return Promise.resolve([] as T);
  if (command === 'biometric_check_availability')
    return Promise.resolve({ available: false, configured: false } as T);
  if (command === 'vault_list_accounts')
    return Promise.resolve([{ id: PUBLIC_ACCOUNT_ID, name: '公开布局夹具' }] as T);
  // No account/Vault/business commands are registered in the isolated native host.
  return Promise.resolve(null as T);
}
