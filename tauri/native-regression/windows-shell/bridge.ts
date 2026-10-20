import { invoke } from '@tauri-apps/api/core';
import { publicObjects } from './stores';
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
  if (command === 'object_get' && args?.accountId === 'public-layout-fixture') {
    const object = publicObjects.find((item) => item.id === args.objectId);
    return Promise.resolve(
      (object
        ? {
            id: object.id,
            name: object.name,
            accountId: 'public-layout-fixture',
            typeId: object.sectionId,
            sensitivityLevel: 'public',
            createdAt: '2026-10-20T00:00:00Z',
            updatedAt: '2026-10-20T00:00:00Z',
            properties: { example: '公开示例内容', private_note: 'PUBLIC-FIXTURE-MASK-CHECK' },
            propertyLabels: { example: 'public', private_note: 'sensitive' },
            templateId: null,
            templateType: null,
            deletedAt: null,
          }
        : null) as T,
    );
  }
  // No account/Vault/business commands are registered in the isolated native host.
  return Promise.resolve(null as T);
}
