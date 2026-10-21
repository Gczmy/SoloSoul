import type { ObjectData, SnapshotEntry } from '@/lib/generated/ipcContracts';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { publicObjects } from './stores';

export const PUBLIC_ACCOUNT_ID = 'public-layout-fixture';
const firstTime = Date.UTC(2026, 9, 12, 8);
export function publicObjectData(id: string): ObjectData | null {
  const object = publicObjects.find((item) => item.id === id);
  if (!object) return null;
  const fields = Object.fromEntries(
    Array.from({ length: 14 }, (_, index) => [
      `field-${index}`,
      {
        name: `公开字段 ${index + 1}`,
        type: index === 0 ? 'multiline' : 'text',
        sensitivityLevel: 'public',
      },
    ]),
  );
  return {
    id: object.id,
    accountId: PUBLIC_ACCOUNT_ID,
    name: object.name,
    typeId: object.sectionId,
    sensitivityLevel: 'public',
    templateId: null,
    templateType: null,
    templateHash: null,
    ignoredTemplateHash: null,
    contractTypeId: null,
    deletedAt: null,
    tags: [],
    createdAt: new Date(firstTime).toISOString(),
    updatedAt: new Date(firstTime + 9 * 86400000).toISOString(),
    properties: {
      __fields: fields,
      ...Object.fromEntries(
        Object.keys(fields).map((key, index) => [
          key,
          index === 0
            ? `这是 ${object.name} 的公开内容。\n右侧历史与附件会跟随左侧选中的对象。\n可以分别滚动编辑区与右侧列表。`
            : `${object.id} · 示例字段值 ${index + 1}`,
        ]),
      ),
    },
    propertyLabels: Object.fromEntries(Object.keys(fields).map((key) => [key, 'public'])),
  };
}
export function publicHistory(objectId: string): SnapshotEntry[] {
  if (!publicObjectData(objectId)) return [];
  const count =
    objectId === 'public-object-0'
      ? 48
      : objectId === 'public-object-1'
        ? 17
        : objectId === 'public-object-2'
          ? 0
          : 6;
  // 故意按旧到新返回，验收正式组件自己的倒序排列。
  return Array.from({ length: count }, (_, index) => ({
    id: `${objectId}:snapshot:${index}`,
    timestamp: firstTime + index * 4 * 3600000,
    triggeredBy: 'user_edit',
    diffSummary: index === 0 ? 'diff_created' : 'diff_updated',
  }));
}
export function publicSnapshot(snapshotId: string) {
  const match = /^(public-object-\d+):snapshot:(\d+)$/.exec(snapshotId);
  const object = match && publicObjectData(match[1]);
  if (!object || !publicHistory(object.id).some((item) => item.id === snapshotId)) return null;
  return {
    name: object.name,
    tags: ['公开演示'],
    properties: {
      __fields: {
        summary: { name: '变更说明', type: 'text', sensitivityLevel: 'public' },
        reference: { name: '对象归属', type: 'text', sensitivityLevel: 'public' },
        hidden: { name: '敏感字段演示', type: 'text', sensitivityLevel: 'sensitive' },
      },
      summary: `公开版本 ${Number(match[2]) + 1} 的内容`,
      reference: object.id,
      hidden: '仅供演示的遮挡值，不包含实际凭据',
    },
    propertyLabels: { summary: 'public', reference: 'public', hidden: 'sensitive' },
  };
}
export function publicAttachments(objectId: string): AttachmentItem[] {
  if (!publicObjectData(objectId)) return [];
  const count =
    objectId === 'public-object-0'
      ? 44
      : objectId === 'public-object-1'
        ? 13
        : objectId === 'public-object-2'
          ? 0
          : 5;
  return Array.from({ length: count }, (_, index) => ({
    id: `${objectId}:attachment:${index}`,
    objectId,
    fileName: `${objectId} · 公开附件 ${String(index + 1).padStart(2, '0')} · 用于验收长文件名称与独立滚动.txt`,
    mimeType: 'text/plain',
    sizeBytes: 1024 * (index + 1),
    createdAt: new Date(firstTime + index * 3 * 3600000).toISOString(),
    vaultPath: `public-fixture://${objectId}/${index}.txt`,
  }));
}
export function publicAttachmentText(path: string): string | null {
  const match = /^public-fixture:\/\/(public-object-\d+)\/(\d+)\.txt$/.exec(path);
  if (!match || !publicAttachments(match[1]).some((item) => item.vaultPath === path)) return null;
  return `公开附件内容\n对象：${match[1]}\n文件序号：${Number(match[2]) + 1}\n\n这段内容由隔离夹具在内存中生成，不读取磁盘文件。`;
}
