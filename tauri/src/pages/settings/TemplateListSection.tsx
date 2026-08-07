import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { LoadingPlaceholder } from '@/components/ui/LoadingPlaceholder';
import { FilterChipGroup } from '@/components/ui/FilterChipGroup';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { SensitivityBadges } from '@/components/template/SensitivityBadges';
import { PluginBadge } from '@/components/template/PluginBadge';
import { LayoutTemplate, Pencil, Search } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import { resolveCustomIcon } from '@/lib/pageIcons';
import type { UserTemplate, ContractRoleBinding } from '@/types/template';

/** 列表行模板投影（P224-③ 拆分，规范出处：TemplateManagerPage 原内联接口） */
export interface ListTemplate {
  id: string;
  name: string;
  category: string;
  contractTypeId?: string;
  properties: Array<{
    id: string;
    name: string;
    type: string;
    sensitivityLevel?: string;
    deprecatedAt?: string;
    contractField?: boolean;
    contractBindings?: ContractRoleBinding[];
  }>;
}

/** 页面归属解析结果（name + 是否已删除） */
export interface PageLabel {
  name: string;
  deleted: boolean;
}

interface TemplateListSectionProps {
  isLoading: boolean;
  error: string | null;
  allTemplates: ListTemplate[];
  filteredTemplates: ListTemplate[];
  templates: UserTemplate[];
  pageFilter: string;
  onPageFilterChange: (v: string | null) => void;
  pageOptions: { id: string; label: string }[];
  searchQuery: string;
  onSearchQueryChange: (v: string) => void;
  resolvePageLabel: (category: string) => PageLabel;
  onOpenEdit: (tpl: UserTemplate) => void;
  onOpenDetail: (tpl: ListTemplate) => void;
  onDelete: (id: string, name: string) => void;
}

/**
 * 模板列表面板（纯展示，P224-③ 拆分）：
 * 搜索 + 页面筛选 + 模板卡片列表 + 三态空位（加载/无模板/无筛选命中）。
 * 数据与回调均由 TemplateManagerPage 透传。
 */
export function TemplateListSection({
  isLoading,
  error,
  allTemplates,
  filteredTemplates,
  templates,
  pageFilter,
  onPageFilterChange,
  pageOptions,
  searchQuery,
  onSearchQueryChange,
  resolvePageLabel,
  onOpenEdit,
  onOpenDetail,
  onDelete,
}: TemplateListSectionProps) {
  const { t } = useTranslation(['common', 'settings']);

  return (
    <>
      {isLoading && templates.length === 0 && <LoadingPlaceholder variant="base" minHeight={120} />}
      {error && <div style={{ color: 'var(--error)' }}>{error}</div>}

      {!isLoading && !error && allTemplates.length > 0 && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 10 }}>
          <Input
            placeholder={t('settings:search_templates', { defaultValue: '搜索模板...' })}
            value={searchQuery}
            onChange={(e) => onSearchQueryChange(e.target.value)}
            onClear={() => onSearchQueryChange('')}
            prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
          />
          <FilterChipGroup
            options={pageOptions.map((opt) => ({ id: opt.id, label: opt.label }))}
            value={pageFilter}
            onChange={onPageFilterChange}
          />
        </div>
      )}

      {!isLoading && allTemplates.length === 0 && (
        <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
          <LayoutTemplate size={ICON_SIZE['5xl']} style={{ marginBottom: 12, opacity: 0.4 }} />
          <div>{t('settings:no_templates', { defaultValue: '暂无模板' })}</div>
          <div style={{ fontSize: 'var(--text-caption)', marginTop: 4 }}>
            {t('settings:no_templates_hint', { defaultValue: '点击右上角"新建模板"创建' })}
          </div>
        </div>
      )}

      {!isLoading && allTemplates.length > 0 && filteredTemplates.length === 0 && (
        <div style={{ textAlign: 'center', color: 'var(--text-secondary)', padding: 40 }}>
          <div>{t('settings:no_templates_filtered', { defaultValue: '没有符合筛选条件的模板' })}</div>
        </div>
      )}

      <div style={{ display: 'flex', flexDirection: 'column', gap: 'var(--card-gap-sm)' }}>
        {filteredTemplates.map((tpl) => {
          const ut = templates.find((u) => u.id === tpl.id);
          const TemplateIcon = ut?.iconId ? resolveCustomIcon(ut.iconId) : LayoutTemplate;
          return (
            <Card key={tpl.id} interactive onClick={() => onOpenDetail(tpl)}>
              <div
                style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between' }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
                  <TemplateIcon size={ICON_SIZE.xl} style={{ flexShrink: 0 }} />
                  <div>
                    <div
                      style={{
                        fontSize: 'var(--text-sm)',
                        fontWeight: 500,
                        display: 'flex',
                        alignItems: 'center',
                        gap: 6,
                      }}
                    >
                      {tpl.name}
                      <PluginBadge
                        contractTypeId={templates.find((u) => u.id === tpl.id)?.contractTypeId}
                        size="sm"
                      />
                    </div>
                    <div
                      style={{
                        fontSize: 'var(--text-badge)',
                        color: 'var(--text-tertiary)',
                        display: 'flex',
                        alignItems: 'center',
                        gap: 8,
                        flexWrap: 'wrap',
                      }}
                    >
                      {(() => {
                        const page = resolvePageLabel(tpl.category);
                        return (
                          <span
                            style={
                              page.deleted
                                ? { textDecoration: 'line-through', opacity: 0.6 }
                                : undefined
                            }
                          >
                            {page.name}
                          </span>
                        );
                      })()}
                      <span>·</span>
                      <span>
                        {tpl.properties.length} {t('settings:template_fields', { defaultValue: '个字段' })}
                      </span>
                      <SensitivityBadges properties={tpl.properties} />
                    </div>
                  </div>
                </div>
                <div style={{ display: 'flex', gap: 8 }} onClick={(e) => e.stopPropagation()}>
                  <Button
                    variant="tertiary"
                    size="sm"
                    onClick={() => {
                      const ut = templates.find((u) => u.id === tpl.id);
                      if (ut) onOpenEdit(ut);
                    }}
                  >
                    <Pencil size={ICON_SIZE.md} style={{ color: 'var(--text-primary)' }} />
                  </Button>
                  <DeleteButton
                    onClick={() => onDelete(tpl.id, tpl.name)}
                    title={t('common:delete')}
                    iconOnly
                  />
                </div>
              </div>
            </Card>
          );
        })}
      </div>
    </>
  );
}
