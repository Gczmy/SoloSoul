import { useTranslation } from 'react-i18next';
import { Input } from '@/components/ui/Input';
import { SelectCheckbox } from '@/components/ui/SelectCheckbox';
import { resolveFieldSensitivity } from '@/lib/fieldSensitivity';
import type { SensitivityLevel } from '@/types/template';
import { AndroidTemplateChoiceList } from './AndroidTemplateChoiceList';
import { FIELD_TYPES, SENSITIVITY_LEVELS, type TemplateEditorState } from './types';
import styles from './AndroidTemplateEditor.module.css';

export function AndroidTemplateDynamicGroup({ editor }: { editor: TemplateEditorState }) {
  const { t } = useTranslation('editor');
  const allowed = editor.dynamicGroupAllowedTypes?.length
    ? editor.dynamicGroupAllowedTypes
    : FIELD_TYPES;
  const sensitivity = resolveFieldSensitivity({
    fieldId: 'dynamic_group',
    template: { sensitivityLevel: editor.dynamicGroupSensitivity },
  });
  return (
    <div className={styles.config}>
      <p className={styles.label}>{t('dynamic_group_allowed_types')}</p>
      <div className={styles.typeGrid}>
        {FIELD_TYPES.map((type) => (
          <label key={type} className={styles.checkRow}>
            <SelectCheckbox
              checked={allowed.includes(type)}
              disabled={allowed.length === 1 && allowed.includes(type)}
              onChange={() =>
                editor.handleDynamicGroupAllowedTypesChange(
                  allowed.includes(type)
                    ? allowed.filter((value) => value !== type)
                    : [...allowed, type],
                )
              }
            />
            <span>{t(`field_types.${type}`)}</span>
          </label>
        ))}
      </div>
      <Input
        label={t('dynamic_group_max_items_label')}
        type="number"
        min={1}
        step={1}
        inputMode="numeric"
        placeholder={t('dynamic_group_no_limit')}
        value={editor.dynamicGroupMaxItems ?? ''}
        onChange={(event) => {
          const raw = event.target.value;
          if (!raw || raw === '0') editor.handleDynamicGroupMaxItemsChange(undefined);
          else if (Number.isInteger(Number(raw)) && Number(raw) >= 1)
            editor.handleDynamicGroupMaxItemsChange(Number(raw));
        }}
      />
      <p className={styles.label}>{t('dynamic_group_sensitivity')}</p>
      <AndroidTemplateChoiceList
        label={t('dynamic_group_sensitivity')}
        value={sensitivity}
        choices={SENSITIVITY_LEVELS.map((value) => ({
          value,
          label: t(`sensitivity_levels.${value}`),
        }))}
        onChoose={(value) => editor.handleDynamicGroupSensitivityChange(value as SensitivityLevel)}
      />
    </div>
  );
}
