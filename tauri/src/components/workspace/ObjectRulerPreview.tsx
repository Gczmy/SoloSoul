import { useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowDownToLine, Paperclip } from 'lucide-react';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { useRevealState } from '@/hooks/useRevealState';
import { flattenPropertyEntries } from '@/lib/propertyFlatten';
import type { ObjectSummary } from '@/stores/objectStore';
import type { SensitivityLevel, UserTemplate } from '@/types/template';
import styles from './WorkspaceObjectRuler.module.css';

interface ObjectRulerPreviewProps {
  object: ObjectSummary;
  template?: UserTemplate;
  collectionLabel: string;
  index: number;
  total: number;
  attachmentCount?: number;
  onNavigate: () => void;
}

function sensitivity(value: string | undefined): SensitivityLevel {
  return value === 'public' || value === 'sensitive' || value === 'critical' ? value : 'internal';
}

export function ObjectRulerPreview({
  object,
  template,
  collectionLabel,
  index,
  total,
  attachmentCount,
  onNavigate,
}: ObjectRulerPreviewProps) {
  const { t } = useTranslation('common');
  const { maskValue } = useRevealState();
  // 只展平正在预览的对象，复用现有列表摘要，不额外读取完整对象或附件。
  const fields = useMemo(
    () =>
      flattenPropertyEntries(
        object.properties,
        template?.properties.map((p) => p.id),
      )
        .filter((field) => field.kind === 'field')
        .slice(0, 3),
    [object.properties, template],
  );
  const definitions = object.properties?.__fields as Record<string, { name?: string }> | undefined;

  return (
    <>
      <div className={styles.previewMeta}>
        <span>{collectionLabel}</span>
        <span>
          {index + 1} / {total}
        </span>
      </div>
      <h2 className={styles.previewTitle}>{object.name}</h2>
      <div className={styles.badges}>
        <SensitivityBadge level={sensitivity(object.sensitivityLevel)} />
        {attachmentCount ? (
          <span className={styles.attachments}>
            <Paperclip size={12} />
            {attachmentCount}
          </span>
        ) : null}
      </div>
      {fields.length > 0 && (
        <dl className={styles.fields}>
          {fields.map((field, i) => {
            const property = template?.properties.find((p) => p.id === field.key);
            const level = sensitivity(
              object.propertyLabels?.[field.key] ?? property?.sensitivityLevel,
            );
            return (
              <div key={field.fieldId || `${field.key}-${i}`} className={styles.field}>
                <dt>
                  {field.label || property?.name || definitions?.[field.key]?.name || field.key}
                </dt>
                <dd>
                  <span>
                    {maskValue(field.value, `${object.id}:${field.fieldId || field.key}`, level)}
                  </span>
                  <SensitivityBadge level={level} showText={false} />
                </dd>
              </div>
            );
          })}
        </dl>
      )}
      <button type="button" className={styles.jumpButton} onClick={onNavigate}>
        <ArrowDownToLine size={14} />
        {t('object_ruler_jump')}
      </button>
    </>
  );
}
