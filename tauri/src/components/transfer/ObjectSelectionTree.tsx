import { useTranslation } from 'react-i18next';
import type { ReactNode } from 'react';
import { Paperclip } from 'lucide-react';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { formatBytes } from '@/lib/utils';
import { ICON_SIZE } from '@/lib/constants';
import type { SensitivityLevel } from '@/components/ui/SensitivityBadge';

export interface SelectionTreeObject {
  id: string;
  name: string;
  sensitivityLevel: string;
  sectionType: string;
}

export interface SelectionTreeAttachment {
  id: string;
  fileName: string;
  sizeBytes: number;
}

export interface SelectionTreePageGroup {
  sectionType: string;
  pageName?: string;
  /** 可选：页面对象计数（导出侧显式提供；导入侧未提供时回退 objects.length） */
  objectCount?: number;
  objects: SelectionTreeObject[];
}

interface ObjectSelectionTreeProps {
  pageGroups: SelectionTreePageGroup[];
  selectedPageIds: Set<string>;
  expandedPages: Set<string>;
  expandedObjects: Set<string>;
  selectedAttachmentIds: Set<string>;
  objectAttachments: Map<string, SelectionTreeAttachment[]>;
  totalSelected: number;
  /** 对象行是否显示附件展开箭头（导出：includeAttachments；导入：该对象有附件） */
  showAttachmentExpand: (obj: SelectionTreeObject) => boolean;
  isObjectSelected: (objId: string) => boolean;
  onTogglePage: (sectionType: string, objectIds: string[]) => void;
  onToggleObject: (objId: string, sectionType: string, objectIds: string[]) => void;
  onToggleObjectExpanded: (objId: string) => void;
  onToggleAttachment: (
    attId: string,
    objId: string,
    sectionType: string,
    objectIds: string[],
  ) => void;
  onToggleExpandedPage: (sectionType: string) => void;
  onSelectAll: (selectAll: boolean) => void;
  /** 可选：对象行上的冲突徽标（导入场景） */
  renderConflictBadge?: (obj: SelectionTreeObject) => ReactNode;
  /** 可选：限制树高度并滚动（导入场景） */
  scrollable?: boolean;
  /** 可选：是否渲染 select-all 行（默认渲染） */
  showSelectAll?: boolean;
}

/**
 * 导出/导入共用的 页面 → 对象 → 附件 选择树。
 * 选择状态通过回调归一化（导出用 Set、导入用 Map，均适配为 isObjectSelected）。
 */
export function ObjectSelectionTree({
  pageGroups,
  selectedPageIds,
  expandedPages,
  expandedObjects,
  selectedAttachmentIds,
  objectAttachments,
  totalSelected,
  showAttachmentExpand,
  isObjectSelected,
  onTogglePage,
  onToggleObject,
  onToggleObjectExpanded,
  onToggleAttachment,
  onToggleExpandedPage,
  onSelectAll,
  renderConflictBadge,
  scrollable = false,
  showSelectAll = true,
}: ObjectSelectionTreeProps) {
  const { t } = useTranslation(['settings', 'common', 'navigation']);
  const totalObjects = pageGroups.reduce((s, g) => s + g.objects.length, 0);
  const selectAll = totalSelected < totalObjects;

  if (pageGroups.length === 0) {
    return (
      <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-tertiary)' }}>
        {t('common:no_data')}
      </p>
    );
  }

  return (
    <>
      {showSelectAll && (
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '4px 0',
            cursor: 'pointer',
            userSelect: 'none',
            borderBottom: '1px solid var(--border-subtle)',
            marginBottom: 4,
          }}
          onClick={() => onSelectAll(selectAll)}
        >
          <SelectCheckbox
            checked={totalSelected > 0 && totalSelected === totalObjects}
            indeterminate={totalSelected > 0 && totalSelected < totalObjects}
            onChange={() => onSelectAll(selectAll)}
          />
          <span style={{ fontSize: 'var(--text-body-sm)', fontWeight: 500, flex: 1 }}>
            {totalSelected === totalObjects ? t('common:deselect_all') : t('common:select_all')}
          </span>
          <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {t('common:object_count', { n: totalSelected })}
          </span>
        </div>
      )}
      <div
        style={
          scrollable
            ? { maxHeight: 320, overflowY: 'auto', fontSize: 'var(--text-body-sm)' }
            : { display: 'flex', flexDirection: 'column', gap: 4 }
        }
      >
        {pageGroups.map((group) => {
          const allIds = group.objects.map((o) => o.id);
          const pageChecked = selectedPageIds.has(group.sectionType);
          const someChecked = !pageChecked && allIds.some((id) => isObjectSelected(id));
          const expanded = expandedPages.has(group.sectionType);
          return (
            <div key={group.sectionType}>
              {/* Page row */}
              <div
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 8,
                  padding: '8px 0',
                  cursor: 'pointer',
                  userSelect: 'none',
                }}
              >
                <SelectCheckbox
                  checked={pageChecked}
                  indeterminate={someChecked && !pageChecked}
                  onChange={() => onTogglePage(group.sectionType, allIds)}
                />
                <span
                  onClick={() => onToggleExpandedPage(group.sectionType)}
                  style={{
                    fontSize: 'var(--text-body)',
                    fontWeight: 600,
                    flex: 1,
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                  }}
                >
                  <span
                    style={{
                      transform: expanded ? 'rotate(90deg)' : 'none',
                      transition: 'transform 0.15s',
                      fontSize: 'var(--text-badge)',
                    }}
                  >
                    ▶
                  </span>
                  <span
                    style={{
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {t(`navigation:${group.sectionType}`, group.pageName ?? group.sectionType)}
                  </span>
                </span>
                <span style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
                  {t('common:object_count', { n: group.objectCount ?? group.objects.length })}
                </span>
              </div>

              {/* Object rows (collapsible) */}
              {expanded &&
                group.objects.map((obj) => (
                  <div key={obj.id}>
                    <label
                      style={{
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        padding: '4px 0 4px 28px',
                        cursor: 'pointer',
                      }}
                    >
                      <SelectCheckbox
                        checked={isObjectSelected(obj.id)}
                        onChange={() => onToggleObject(obj.id, group.sectionType, allIds)}
                      />
                      <span
                        style={{
                          fontSize: 'var(--text-body-sm)',
                          flex: 1,
                          overflow: 'hidden',
                          textOverflow: 'ellipsis',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {obj.name}
                      </span>
                      <SensitivityBadge level={obj.sensitivityLevel as SensitivityLevel} />
                      {renderConflictBadge?.(obj)}
                      {showAttachmentExpand(obj) && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.preventDefault();
                            e.stopPropagation();
                            onToggleObjectExpanded(obj.id);
                          }}
                          style={{
                            fontSize: 'var(--text-badge)',
                            background: 'none',
                            border: 'none',
                            cursor: 'pointer',
                            padding: '0 4px',
                            transform: expandedObjects.has(obj.id) ? 'rotate(90deg)' : 'none',
                            transition: 'transform 0.15s',
                            color: 'var(--text-tertiary)',
                          }}
                        >
                          ▶
                        </button>
                      )}
                    </label>
                    {expandedObjects.has(obj.id) && (
                      <div style={{ paddingLeft: 52, paddingBottom: 4 }}>
                        {(objectAttachments.get(obj.id) || []).length === 0 ? (
                          <span
                            style={{
                              fontSize: 'var(--text-caption)',
                              color: 'var(--text-tertiary)',
                            }}
                          >
                            {t('settings:no_attachments', 'No attachments')}
                          </span>
                        ) : (
                          <>
                            <div
                              style={{
                                display: 'flex',
                                alignItems: 'center',
                                gap: 4,
                                padding: '2px 0',
                                fontSize: 'var(--text-badge)',
                                color: 'var(--text-tertiary)',
                                borderBottom: '1px solid var(--border-subtle)',
                                marginBottom: 2,
                              }}
                            >
                              <Paperclip size={ICON_SIZE['2xs']} />
                              <span>
                                {t('settings:attachments_label', 'Attachments')} (
                                {(objectAttachments.get(obj.id) || []).length})
                              </span>
                            </div>
                            {(objectAttachments.get(obj.id) || []).map((att) => (
                              <label
                                key={att.id}
                                style={{
                                  display: 'flex',
                                  alignItems: 'center',
                                  gap: 6,
                                  padding: '2px 0 2px 16px',
                                  cursor: 'pointer',
                                }}
                              >
                                <SelectCheckbox
                                  checked={selectedAttachmentIds.has(att.id)}
                                  onChange={() =>
                                    onToggleAttachment(att.id, obj.id, group.sectionType, allIds)
                                  }
                                />
                                <Paperclip
                                  size={ICON_SIZE['2xs']}
                                  style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
                                />
                                <span style={{ fontSize: 'var(--text-caption)', flex: 1 }}>
                                  {att.fileName}
                                </span>
                                <span
                                  style={{
                                    fontSize: 'var(--text-badge)',
                                    color: 'var(--text-tertiary)',
                                  }}
                                >
                                  {formatBytes(att.sizeBytes)}
                                </span>
                              </label>
                            ))}
                          </>
                        )}
                      </div>
                    )}
                  </div>
                ))}
            </div>
          );
        })}
      </div>
    </>
  );
}
