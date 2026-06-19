
import React, { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, Trash2, ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { DeprecatedBadge } from '@/components/ui/DeprecatedBadge';
import { SensitivityBadge as UiSensitivityBadge } from '@/components/ui/SensitivityBadge';
import {
  CUSTOM_ICON_MAP,
  resolveCustomIcon,
  type CustomIconId,
} from '@/lib/pageIcons';
import type {
  UserTemplate,
  TemplateProperty,
  PropertyType,
  SensitivityLevel,
} from '@/types/template';
import { TemplateTypeSelect } from './TemplateTypeSelect';
import { TemplatePageSelect } from './TemplatePageSelect';
import { IconPicker } from './IconPicker';
import { OptionsEditor }  from './OptionsEditor';

interface FieldUsage {
  active: number;
  softDeleted: number;
}

interface TemplateEditorProps {
  editingTemplate: UserTemplate | null;

  editName: string;
  editCategory: string;
  editIconId: string;
  editProperties: TemplateProperty[];
  newFieldType: PropertyType;
  showDeprecated: boolean;
  fieldUsageMap: Record<string, FieldUsage>;
  onEditNameChange: (v: string) => void;
  onEditCategoryChange: (v: string) => void;
  onEditIconIdChange: (v: string) => void;
  onNewFieldTypeChange: (v: PropertyType) => void;
  onAddProperty: () => void;
  onUpdatePropertyName: (index: number, name: string) => void;
  onUpdatePropertyType: (index: number, type: PropertyType) => void;
  onUpdatePropertySensitivity: (index: number, level: SensitivityLevel) => void;
  onUpdatePropertyOptions: (index: number, options: string[]) => void;
  onRemoveProperty: (index: number) => void;
  onRestoreProperty: (index: number) => void;
  onPermanentlyRemoveProperty: (index: number) => void;
  onToggleShowDeprecated: () => void;
  onSave: () => void;
  onClose: () => void;
}

const SENSITIVITY_LEVELS: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

export function TemplateEditor({

  editName,
  editCategory,
  editIconId,
  editProperties,
  newFieldType,
  showDeprecated,
  fieldUsageMap,
  onEditNameChange,
  onEditCategoryChange,
  onEditIconIdChange,
  onNewFieldTypeChange,
  onAddProperty,
  onUpdatePropertyName,
  onUpdatePropertyType,
  onUpdatePropertySensitivity,
  onUpdatePropertyOptions,
  onRemoveProperty,
  onRestoreProperty,
  onPermanentlyRemoveProperty,
  onToggleShowDeprecated,
  onSave,
  onClose,
}: TemplateEditorProps) {
  const [showIconPicker, setShowIconPicker] = useState(false);

  const { t } = useTranslation(['settings', 'common', 'editor']);

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 16, width: '100%' }}>
      <Input
        label={t('common:name') || '名称'}
        value={editName}
        onChange={(e) => onEditNameChange(e.target.value)}
      />
      <TemplatePageSelect
        value={editCategory}
        onChange={onEditCategoryChange}
        label={t('settings:template_category') || '所属页面'}
      />

      {/* Icon picker — collapsible, default collapsed */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
        <button
          type="button"
          onClick={() => setShowIconPicker((v) => !v)}
          style={{
            display: 'flex',
            alignItems: 'center',
            gap: 6,
            padding: '6px 10px',
            borderRadius: 8,
            border: '1px solid var(--border-subtle)',
            background: 'var(--bg-toolbar)',
            cursor: 'pointer',
            fontSize: 13,
            fontWeight: 500,
            color: 'var(--text-secondary)',
            fontFamily: 'inherit',
            textAlign: 'left',
            width: '100%',
          }}
        >
          <span
            style={{
              transform: showIconPicker ? 'rotate(90deg)' : 'rotate(0deg)',
              transition: 'transform 0.15s ease',
              display: 'inline-flex',
              flexShrink: 0,
            }}
          >
            <ChevronRight size={14} />
          </span>
          {React.createElement(
            editIconId && editIconId in CUSTOM_ICON_MAP
              ? CUSTOM_ICON_MAP[editIconId as CustomIconId]
              : resolveCustomIcon(editIconId),
            { size: 18, style: { color: 'var(--accent-primary)', flexShrink: 0 } },
          )}
          <span style={{ flex: 1 }}>
            {t('settings:template_icon') || '模板图标'}
          </span>
          <span style={{ fontSize: 12, color: 'var(--text-tertiary)' }}>
            {showIconPicker
              ? (t('common:collapse') || '收起')
              : (t('settings:click_to_change_icon') || '点击选择图标')}
          </span>
        </button>
        {showIconPicker && <IconPicker value={editIconId} onChange={onEditIconIdChange} />}
      </div>

      <div>
        <div
          style={{
            fontSize: 13,
            fontWeight: 500,
            marginBottom: 8,
            color: 'var(--text-secondary)',
          }}
        >
          {t('settings:fields_section_title') || '字段列表'}
        </div>

        <div
          style={{
            background: 'var(--bg-toolbar)',
            borderRadius: 8,
            padding: '8px 4px',
            border: '1px solid var(--border-subtle)',
          }}
        >
          {editProperties.filter((p) => !p.deprecatedAt).length === 0 &&
            editProperties.filter((p) => p.deprecatedAt).length === 0 && (
              <div style={{ fontSize: 12, color: 'var(--text-tertiary)', padding: '12px 0' }}>
                {t('settings:empty_template_hint') || '此模板暂无字段，点击下方添加'}
              </div>
            )}

          <div
            style={{
              display: 'flex',
              flexDirection: 'column',
              gap: 8,
              maxHeight: '35vh',
              overflowY: 'auto',
              overflowX: 'hidden',
              paddingRight: 4,
            }}
          >
            {editProperties
              .map((prop, idx) => ({ prop, idx }))
              .filter(({ prop }) => !prop.deprecatedAt)
              .map(({ prop, idx }) => (
                <div
                  key={prop.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    padding: '8px 10px',
                    borderRadius: 6,
                    background: 'var(--bg-subtle)',
                  }}
                >
                  <div style={{ flex: 1, minWidth: 80 }}>
                    <input
                      value={prop.name}
                      onChange={(e) => onUpdatePropertyName(idx, e.target.value)}
                      placeholder={t('settings:field_name') || '字段名称'}
                      style={{
                        width: '100%',
                        height: 36,
                        padding: '0 10px',
                        borderRadius: 6,
                        border: '1px solid var(--border-subtle)',
                        background: 'var(--bg-elevated)',
                        color: 'var(--text-primary)',
                        fontSize: 14,
                        fontFamily: 'inherit',
                        outline: 'none',
                        boxSizing: 'border-box',
                      }}
                    />
                  </div>
                  <select
                    value={prop.type}
                    onChange={(e) => onUpdatePropertyType(idx, e.target.value as PropertyType)}
                    style={{
                      height: 36,
                      padding: '0 10px',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: 'var(--bg-elevated)',
                      color: 'var(--text-primary)',
                      fontSize: 13,
                      cursor: 'pointer',
                      boxSizing: 'border-box',
                      minWidth: 90,
                    }}
                  >
                    {([
                      'text',
                      'multiline',
                      'number',
                      'date',
                      'datetime',
                      'boolean',
                      'select',
                      'multiselect',
                      'url',
                      'email',
                      'phone',
                      'file',
                    ] as PropertyType[]).map((pt) => (
                      <option key={pt} value={pt}>
                        {t(`editor:field_types.${pt}`, pt)}
                      </option>
                    ))}
                  </select>
                  {(prop.type === 'select' || prop.type === 'multiselect') && (
                    <OptionsEditor
                      options={prop.options || []}
                      onChange={(opts) => onUpdatePropertyOptions(idx, opts)}
                      fieldName={prop.name}
                      fieldType={prop.type === 'multiselect' ? 'multiselect' : 'select'}
                    />
                  )}
                  <select
                    value={prop.sensitivityLevel || 'internal'}
                    onChange={(e) =>
                      onUpdatePropertySensitivity(idx, e.target.value as SensitivityLevel)
                    }
                    style={{
                      height: 36,
                      padding: '0 10px',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: 'var(--bg-elevated)',
                      color: 'var(--text-primary)',
                      fontSize: 13,
                      cursor: 'pointer',
                      boxSizing: 'border-box',
                    }}
                  >
                    {SENSITIVITY_LEVELS.map((sl) => (
                      <option key={sl} value={sl}>
                        {t(`editor:sensitivity_levels.${sl}`, sl)}
                      </option>
                    ))}
                  </select>
                  <button
                    type="button"
                    onClick={() => onRemoveProperty(idx)}
                    title={t('settings:remove_field') || '删除'}
                    style={{
                      height: 36,
                      width: 36,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      borderRadius: 6,
                      border: '1px solid var(--border-subtle)',
                      background: 'transparent',
                      color: '#e74c3c',
                      cursor: 'pointer',
                      flexShrink: 0,
                    }}
                  >
                    <Trash2 size={14} />
                  </button>
                </div>
              ))}
          </div>

          {/* Deprecated fields */}
          {editProperties.filter((p) => p.deprecatedAt).length > 0 && (
            <DeprecatedFieldsSection
              editProperties={editProperties}
              showDeprecated={showDeprecated}
              fieldUsageMap={fieldUsageMap}
              onToggleShowDeprecated={onToggleShowDeprecated}
              onRestoreProperty={onRestoreProperty}
              onPermanentlyRemoveProperty={onPermanentlyRemoveProperty}
            />
          )}
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 12 }}>
          <TemplateTypeSelect value={newFieldType} onChange={onNewFieldTypeChange} />
          <button
            type="button"
            onClick={onAddProperty}
            style={{
              height: 36,
              padding: '0 14px',
              borderRadius: 6,
              border: 'none',
              background: 'var(--accent-primary)',
              color: 'white',
              fontSize: 13,
              cursor: 'pointer',
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              whiteSpace: 'nowrap',
            }}
          >
            <Plus size={14} />
            {t('settings:add_field') || '添加字段'}
          </button>
        </div>
      </div>

      <div style={{ display: 'flex', justifyContent: 'flex-end', gap: 8, paddingTop: 4 }}>
        <Button variant="secondary" onClick={onClose}>
          {t('common:cancel') || '取消'}
        </Button>
        <Button onClick={onSave}>
          {t('common:save') || '保存'}
        </Button>
      </div>
    </div>
  );
}

interface DeprecatedFieldsSectionProps {
  editProperties: TemplateProperty[];
  showDeprecated: boolean;
  fieldUsageMap: Record<string, FieldUsage>;
  onToggleShowDeprecated: () => void;
  onRestoreProperty: (index: number) => void;
  onPermanentlyRemoveProperty: (index: number) => void;
}

function DeprecatedFieldsSection({
  editProperties,
  showDeprecated,
  fieldUsageMap,
  onToggleShowDeprecated,
  onRestoreProperty,
  onPermanentlyRemoveProperty,
}: DeprecatedFieldsSectionProps) {
  const { t } = useTranslation(['settings', 'common']);

  const fieldTypeIcons: Record<string, React.ReactNode> = {
    text: <span style={{ fontSize: 10 }}>T</span>,
    multiline: <span style={{ fontSize: 10 }}>¶</span>,
    number: <span style={{ fontSize: 10 }}>#</span>,
    date: <span style={{ fontSize: 10 }}>📅</span>,
    datetime: <span style={{ fontSize: 10 }}>🕐</span>,
    boolean: <span style={{ fontSize: 10 }}>☑</span>,
    select: <span style={{ fontSize: 10 }}>▼</span>,
    multiselect: <span style={{ fontSize: 10 }}>☑☑</span>,
    url: <span style={{ fontSize: 10 }}>🔗</span>,
    email: <span style={{ fontSize: 10 }}>✉</span>,
    phone: <span style={{ fontSize: 10 }}>📞</span>,
    file: <span style={{ fontSize: 10 }}>📄</span>,
  };

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
          fontSize: 12,
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
                  <span style={{ color: 'var(--text-tertiary)' }}>
                    {fieldTypeIcons[prop.type] || fieldTypeIcons.text}
                  </span>
                  <span
                    style={{
                      fontSize: 14,
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
                    {t('common:restore') || '恢复'}
                  </Button>
                  {cleanable && (
                    <Button
                      variant="tertiary"
                      size="sm"
                      onClick={() => onPermanentlyRemoveProperty(idx)}
                      style={{ color: '#e74c3c' }}
                    >
                      {t('common:clean_up') || '清理'}
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
                    <span style={{ fontSize: 11, color: 'var(--text-tertiary)' }}>
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
                          fontSize: 11,
                          color: 'var(--accent-primary)',
                          cursor: 'pointer',
                          textDecoration: 'underline',
                          whiteSpace: 'nowrap',
                        }}
                      >
                        {t('common:go_to_trash') || '前往回收站'}
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
