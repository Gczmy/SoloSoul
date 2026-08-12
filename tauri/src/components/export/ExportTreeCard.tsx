import type { TFunction } from 'i18next';
import { Card } from '@/components/ui/Card';
import { ObjectSelectionTree } from '@/components/transfer/ObjectSelectionTree';
import type { AttachmentInfo, PageGroup } from '@/types/exportImport';

// PageGroup 单一来源：types/exportImport.ts（useExportImportPage 亦从该处导入）
export type { PageGroup } from '@/types/exportImport';

/**
 * ExportSection 的「页面与对象选择树」卡片（P046 拆分：展示子组件）。
 */
export function ExportTreeCard({
  pageGroups,
  selectedPageIds,
  selectedObjectIds,
  expandedPages,
  expandedObjects,
  selectedAttachmentIds,
  objectAttachments,
  totalSelected,
  includeAttachments,
  onTogglePage,
  onToggleObject,
  onToggleObjectExpanded,
  onToggleAttachment,
  onToggleExpandedPage,
  onSelectAllExport,
  t,
}: {
  pageGroups: PageGroup[];
  selectedPageIds: Set<string>;
  selectedObjectIds: Set<string>;
  expandedPages: Set<string>;
  expandedObjects: Set<string>;
  selectedAttachmentIds: Set<string>;
  objectAttachments: Map<string, AttachmentInfo[]>;
  totalSelected: number;
  includeAttachments: boolean;
  onTogglePage: (sectionType: string, objectIds: string[]) => void;
  onToggleObject: (id: string, sectionType: string, allIdsInGroup: string[]) => void;
  onToggleObjectExpanded: (objectId: string) => void;
  onToggleAttachment: (
    attId: string,
    objectId: string,
    sectionType: string,
    allIdsInGroup: string[],
  ) => void;
  onToggleExpandedPage: (sectionType: string) => void;
  onSelectAllExport: (selectAll: boolean) => void;
  t: TFunction;
}) {
  return (
    <Card>
      <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600, marginBottom: 8 }}>
        {t('settings:select_objects')}
      </h3>
      <ObjectSelectionTree
        pageGroups={pageGroups}
        selectedPageIds={selectedPageIds}
        expandedPages={expandedPages}
        expandedObjects={expandedObjects}
        selectedAttachmentIds={selectedAttachmentIds}
        objectAttachments={objectAttachments}
        totalSelected={totalSelected}
        // 仅真实包含（未软删）附件的对象显示展开图标；无附件对象不再展示「无附件」空面板
        showAttachmentExpand={(obj) => includeAttachments && obj.hasAttachments === true}
        isObjectSelected={(id) => selectedObjectIds.has(id)}
        onTogglePage={onTogglePage}
        onToggleObject={onToggleObject}
        onToggleObjectExpanded={onToggleObjectExpanded}
        onToggleAttachment={onToggleAttachment}
        onToggleExpandedPage={onToggleExpandedPage}
        onSelectAll={onSelectAllExport}
      />
    </Card>
  );
}
