import { useTranslation } from 'react-i18next';
import { ChevronDown, Link, List, Plus } from 'lucide-react';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { resolveFieldSensitivity } from '@/lib/fieldSensitivity';
import type { TemplateEditorState, TemplatePanel } from './types';
import styles from './AndroidTemplateEditor.module.css';

export type OpenTemplatePanel = (panel: TemplatePanel, trigger: HTMLElement) => void;

export function AndroidTemplateFields({
  editor,
  openPanel,
}: {
  editor: TemplateEditorState;
  openPanel: OpenTemplatePanel;
}) {
  const { t } = useTranslation(['settings', 'editor', 'common']);
  const fields = editor.editProperties.map((prop, idx) => ({ prop, idx }));
  const active = fields.filter(({ prop }) => !prop.deprecatedAt && prop.type !== 'dynamic_group');
  const deprecated = fields.filter(({ prop }) => prop.deprecatedAt);
  return (
    <section className={styles.section} aria-label={t('settings:fields_section_title')}>
      <h3>{t('settings:fields_section_title')}</h3>
      {active.length === 0 && <p className={styles.hint}>{t('settings:empty_template_hint')}</p>}
      {active.map(({ prop, idx }, order) => (
        <div key={prop.id} className={styles.field} data-template-field={prop.id}>
          <div className={styles.fieldName}>
            <Input
              label={`${t('settings:field_name')} ${order + 1}`}
              value={prop.name}
              data-template-field-name={prop.id}
              onChange={(event) => editor.updatePropertyName(idx, event.target.value)}
            />
            <DeleteButton
              iconOnly
              title={`${t('settings:remove_field')}: ${prop.name}`}
              onClick={() => void editor.removeProperty(idx)}
            />
          </div>
          <div className={styles.choices}>
            <button
              type="button"
              className={styles.choice}
              onClick={(event) =>
                openPanel({ kind: 'type', fieldId: prop.id }, event.currentTarget)
              }
            >
              <FieldTypeIcon type={prop.type} size={20} />
              <span>
                <small>{t('settings:template_mobile.field_type')}</small>
                {t(`editor:field_types.${prop.type}`)}
              </span>
              <ChevronDown size={16} />
            </button>
            <button
              type="button"
              className={styles.choice}
              onClick={(event) =>
                openPanel({ kind: 'sensitivity', fieldId: prop.id }, event.currentTarget)
              }
            >
              <span>
                <small>{t('settings:search_sensitivity')}</small>
                {t(
                  `editor:sensitivity_levels.${resolveFieldSensitivity({ fieldId: prop.id, template: prop })}`,
                )}
              </span>
              <ChevronDown size={16} />
            </button>
          </div>
          <div className={styles.actions}>
            {(prop.type === 'select' || prop.type === 'multiselect') && (
              <Button
                variant="secondary"
                onClick={(event) =>
                  openPanel({ kind: 'options', fieldId: prop.id }, event.currentTarget)
                }
              >
                <List size={18} />
                {t('common:edit_options')} ({prop.options?.length ?? 0})
              </Button>
            )}
            <Button
              variant="tertiary"
              onClick={(event) =>
                openPanel({ kind: 'binding', fieldId: prop.id }, event.currentTarget)
              }
            >
              <Link size={18} />
              {t('settings:plugin_binding')}
            </Button>
          </div>
        </div>
      ))}
      <Button
        variant="secondary"
        onClick={(event) => openPanel({ kind: 'add' }, event.currentTarget)}
      >
        <Plus size={20} />
        {t('settings:add_field')}
      </Button>
      {deprecated.length > 0 && (
        <>
          <Button
            variant="tertiary"
            aria-expanded={editor.showDeprecated}
            onClick={editor.toggleShowDeprecated}
          >
            {t('settings:deprecated_fields_count', { count: deprecated.length })}
          </Button>
          {editor.showDeprecated &&
            deprecated.map(({ prop, idx }) => {
              const usage = editor.fieldUsageMap[prop.id];
              const cleanable = usage && usage.active === 0 && usage.softDeleted === 0;
              return (
                <div key={prop.id} className={styles.field}>
                  <span className={styles.deprecatedName}>{prop.name}</span>
                  <div>
                    <SensitivityBadge
                      level={resolveFieldSensitivity({ fieldId: prop.id, template: prop })}
                    />
                  </div>
                  {usage && !cleanable && (
                    <p className={styles.hint}>
                      {t('settings:template_mobile.field_usage', {
                        active: usage.active,
                        trash: usage.softDeleted,
                      })}
                    </p>
                  )}
                  <div className={styles.actions}>
                    <Button variant="secondary" onClick={() => editor.restoreProperty(idx)}>
                      {t('common:restore')}
                    </Button>
                    {cleanable && (
                      <Button
                        variant="danger-outline"
                        onClick={() => editor.permanentlyRemoveProperty(idx)}
                      >
                        {t('common:clean_up')}
                      </Button>
                    )}
                  </div>
                </div>
              );
            })}
        </>
      )}
    </section>
  );
}
