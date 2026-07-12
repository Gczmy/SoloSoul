import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Plus, Trash2, ChevronUp, ChevronDown } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { TemplateFieldInput } from '@/components/TemplateFieldInput';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import type { PropertyType, SensitivityLevel } from '@/types/template';
import styles from './DynamicGroupEditor.module.css';

export interface DynamicGroupItem {
  id: string;
  name: string;
  type: PropertyType;
  sensitivity?: SensitivityLevel;
  value: unknown;
}

interface DynamicGroupEditorProps {
  propertyId: string;
  label: string;
  value: unknown;
  allowedTypes?: PropertyType[];
  maxItems?: number;
  sensitivity?: SensitivityLevel;
  onChange: (value: DynamicGroupItem[]) => void;
  disabled?: boolean;
}

const ALL_PROPERTY_TYPES: PropertyType[] = [
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
];

function normalizeValue(value: unknown): DynamicGroupItem[] {
  if (!Array.isArray(value)) return [];
  return value.filter(
    (item): item is DynamicGroupItem =>
      item && typeof item === 'object' && 'id' in item && 'name' in item && 'type' in item,
  );
}

function defaultForType(type: PropertyType): unknown {
  switch (type) {
    case 'boolean':
      return false;
    case 'number':
      return 0;
    case 'multiselect':
      return [];
    case 'dynamic_group':
      return [];
    default:
      return '';
  }
}

export function DynamicGroupEditor({
  propertyId,
  label,
  value,
  allowedTypes,
  maxItems,
  sensitivity,
  onChange,
  disabled,
}: DynamicGroupEditorProps) {
  const { t } = useTranslation(['common', 'editor']);
  const items = normalizeValue(value);
  const [editingNameId, setEditingNameId] = useState<string | null>(null);

  const availableTypes = allowedTypes?.length ? allowedTypes : ALL_PROPERTY_TYPES;
  const canAdd = maxItems === undefined || items.length < maxItems;

  const handleAdd = useCallback(() => {
    const newItem: DynamicGroupItem = {
      id: crypto.randomUUID(),
      name: t('editor:dynamic_group_new_item', { defaultValue: '新字段' }),
      type: availableTypes[0] || 'text',
      sensitivity,
      value: defaultForType(availableTypes[0] || 'text'),
    };
    onChange([...items, newItem]);
  }, [items, onChange, availableTypes, t, sensitivity]);

  const handleRemove = useCallback(
    (id: string) => {
      onChange(items.filter((i) => i.id !== id));
    },
    [items, onChange],
  );

  const handleMove = useCallback(
    (id: string, direction: -1 | 1) => {
      const idx = items.findIndex((i) => i.id === id);
      if (idx < 0) return;
      const newIdx = idx + direction;
      if (newIdx < 0 || newIdx >= items.length) return;
      const next = [...items];
      [next[idx], next[newIdx]] = [next[newIdx], next[idx]];
      onChange(next);
    },
    [items, onChange],
  );

  const handleUpdate = useCallback(
    (id: string, patch: Partial<DynamicGroupItem>) => {
      onChange(
        items.map((item) => {
          if (item.id !== id) return item;
          const next = { ...item, ...patch };
          // 类型变更时重置默认值
          if (patch.type && patch.type !== item.type) {
            next.value = defaultForType(patch.type);
          }
          return next;
        }),
      );
    },
    [items, onChange],
  );

  return (
    <div className={styles.container}>
      <div className={styles.header}>
        <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <FieldTypeIcon type="dynamic_group" />
          <span className={styles.label}>{label}</span>
        </div>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          {sensitivity && <SensitivityBadge level={sensitivity} />}
          {maxItems !== undefined && (
            <span className={styles.count}>
              {items.length}/{maxItems}
            </span>
          )}
        </div>
      </div>

      <div className={styles.items}>
        {items.map((item, index) => (
          <div key={item.id} className={styles.item}>
            <div className={styles.itemHeader}>
              {editingNameId === item.id ? (
                <Input
                  value={item.name}
                  onChange={(e) => handleUpdate(item.id, { name: e.target.value })}
                  onBlur={() => setEditingNameId(null)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') setEditingNameId(null);
                  }}
                  autoFocus
                  className={styles.nameInput}
                />
              ) : (
                <button
                  type="button"
                  className={styles.nameButton}
                  onClick={() => setEditingNameId(item.id)}
                  disabled={disabled}
                >
                  <FieldTypeIcon type={item.type} />
                  <span>{item.name || t('common:unnamed')}</span>
                </button>
              )}

              <div className={styles.actions}>
                <Button
                  variant="tertiary"
                  size="sm"
                  onClick={() => handleMove(item.id, -1)}
                  disabled={disabled || index === 0}
                  aria-label={t('common:move_up')}
                >
                  <ChevronUp size={14} />
                </Button>
                <Button
                  variant="tertiary"
                  size="sm"
                  onClick={() => handleMove(item.id, 1)}
                  disabled={disabled || index === items.length - 1}
                  aria-label={t('common:move_down')}
                >
                  <ChevronDown size={14} />
                </Button>
                <Button
                  variant="danger-outline"
                  size="sm"
                  onClick={() => handleRemove(item.id)}
                  disabled={disabled}
                  aria-label={t('common:delete')}
                >
                  <Trash2 size={14} />
                </Button>
              </div>
            </div>

            <div className={styles.itemBody}>
              <div className={styles.typeSelectRow}>
                <select
                  className={styles.typeSelect}
                  value={item.type}
                  onChange={(e) =>
                    handleUpdate(item.id, { type: e.target.value as PropertyType })
                  }
                  disabled={disabled}
                >
                  {availableTypes.map((pt) => (
                    <option key={pt} value={pt}>
                      {t(`editor:field_types.${pt}`, pt)}
                    </option>
                  ))}
                </select>
              </div>
              <TemplateFieldInput
                propertyId={`${propertyId}-${item.id}`}
                label=""
                type={item.type}
                value={item.value}
                onChange={(val) => handleUpdate(item.id, { value: val })}
                disabled={disabled}
              />
            </div>
          </div>
        ))}
      </div>

      {canAdd && (
        <Button
          variant="secondary"
          size="sm"
          onClick={handleAdd}
          disabled={disabled}
          className={styles.addButton}
        >
          <Plus size={14} />
          {t('editor:dynamic_group_add', { defaultValue: '添加字段' })}
        </Button>
      )}
    </div>
  );
}
