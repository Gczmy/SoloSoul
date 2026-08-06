import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/Button';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SensitivityBadge as UiSensitivityBadge } from '@/components/ui/SensitivityBadge';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { ICON_SIZE } from '@/lib/constants';
import type { SensitivityLevel, TemplateProperty } from '@/types/template';

export interface FieldUsage {
  active: number;
  softDeleted: number;
}

interface DeprecatedFieldsSectionProps {
  editProperties: TemplateProperty[];
  showDeprecated: boolean;
  fieldUsageMap: Record<string, FieldUsage>;
  onToggleShowDeprecated: () => void;
  onRestoreProperty: (index: number) => void;
  onPermanentlyRemoveProperty: (index: number) => void;
}

/**
 * 模板已归档字段区块：折叠开关 + 字段使用情况 + 恢复/清理操作。
 * 从 TemplateEditor 抽出。
 */
export function DeprecatedFieldsSection({
  editProperties,
  showDeprecated,
  fieldUsageMap,
  onToggleShowDeprecated,
  onRestoreProperty,
  onPermanentlyRemoveProperty,
}: DeprecatedFieldsSectionProps) {
  const { t } = useTranslation(['settings', 'common']);

  const deprecatedFields = editProperties
    .map((prop, idx) => ({ prop, idx }))
    .filter(({ prop }) => prop.deprecatedAt);

  return (
    <div style={{ marginTop: 16 }}>
      <button
        onClick={onToggleShowDeprecated}
        style={{
          display: 'flex',
          alignItems: 'center',
          gap: 6,
          fontSize: 'var(--text-caption)',
          color: 'var(--text-secondary)',
          background: 'transparent',
          border: 'none',
          cursor: 'pointer',
          padding: 0,
          fontWeight: 500,
        }}
      >
        <span
          style={{
            transform: showDeprecated ? 'rotate(90deg)' : 'rotate(0deg)',
            transition: 'transform 0.15s ease',
            display: 'inline-block',
          }}
        >
          ▶
        </span>
        {t('settings:deprecated_fields_count', {
          count: deprecatedFields.length,
        })}
      </button>
      {showDeprecated && (
        <div
          style={{
            display: 'flex',
            flexDirection: 'column',
            gap: 8,
            marginTop: 8,
            maxHeight: '20vh',
            overflowY: 'auto',
            overflowX: 'hidden',
            paddingRight: 4,
          }}
        >
          {deprecatedFields.map(({ prop, idx }) => {
            const usage = fieldUsageMap[prop.id];
            const cleanable = usage ? usage.active === 0 && usage.softDeleted === 0 : false;
            return (
              <div
                key={prop.id}
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  gap: 6,
                  padding: '8px 10px',
                  borderRadius: 6,
                  background: 'var(--bg-toolbar)',
                  border: '1px solid var(--border-subtle)',
                  opacity: 0.75,
                }}
              >
                <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
                  <FieldTypeIcon type={prop.type} size={ICON_SIZE.sm} />
                  <span
                    style={{
                      fontSize: 'var(--text-body)',
                      fontWeight: 500,
                      color: 'var(--text-secondary)',
                      flex: 1,
                      minWidth: 1,
                      textDecoration: 'line-through',
                    }}
                  >
                    {prop.name}
                  </span>
                  <UiSensitivityBadge
                    level={(prop.sensitivityLevel || 'internal') as SensitivityLevel}
                  />
                  <DeprecatedBadge />
                  <Button variant="tertiary" size="sm" onClick={() => onRestoreProperty(idx)}>
                    {t('common:restore', { defaultValue: '恢复' })}
                  </Button>
                  {cleanable && (
                    <Button
                      variant="tertiary"
                      size="sm"
                      onClick={() => onPermanentlyRemoveProperty(idx)}
                      style={{ color: '#e74c3c' }}
                    >
                      {t('common:clean_up', { defaultValue: '清理' })}
                    </Button>
                  )}
                </div>
                {usage && !cleanable && (
                  <div
                    style={{
                      display: 'flex',
                      alignItems: 'center',
                      gap: 8,
                      justifyContent: 'space-between',
                    }}
                  >
                    <span style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {usage.active > 0 && usage.softDeleted > 0
                        ? t('settings:field_in_use_both', {
                            activeCount: usage.active,
                            softDeletedCount: usage.softDeleted,
                          })
                        : usage.active > 0
                          ? t('settings:field_in_use_active', {
                              activeCount: usage.active,
                            })
                          : t('settings:field_in_use_trash', {
                              softDeletedCount: usage.softDeleted,
                            })}
                    </span>
                    {usage.softDeleted > 0 && (
                      <span
                        onClick={() => {}}
                        style={{
                          fontSize: 'var(--text-badge)',
                          color: 'var(--accent-primary)',
                          cursor: 'pointer',
                          textDecoration: 'underline',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {t('common:go_to_trash', { defaultValue: '前往回收站' })}
                      </span>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
}
