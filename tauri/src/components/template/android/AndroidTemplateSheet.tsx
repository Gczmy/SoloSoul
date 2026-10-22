import type { Dispatch, SetStateAction } from 'react';
import { useTranslation } from 'react-i18next';
import { ArrowLeft } from 'lucide-react';
import { AndroidSheet } from '@/components/android/AndroidSheet';
import { Button } from '@/components/ui/Button';
import { FieldTypeIcon } from '@/components/ui/FieldTypeIcon';
import { useSettingsStore } from '@/stores/settingsStore';
import { resolveFieldSensitivity } from '@/lib/fieldSensitivity';
import type { PropertyType, SensitivityLevel } from '@/types/template';
import { IconPicker } from '../IconPicker';
import type { UseTemplateEditorStateResult } from '../useTemplateEditorState';
import { AndroidTemplateChoiceList, type TemplateChoice } from './AndroidTemplateChoiceList';
import { AndroidTemplateOptions } from './AndroidTemplateOptions';
import { AndroidTemplateBindings } from './AndroidTemplateBindings';
import { AndroidTemplateDynamicGroup } from './AndroidTemplateDynamicGroup';
import {
  FIELD_TYPES,
  SENSITIVITY_LEVELS,
  SYSTEM_PAGES,
  type TemplateEditorState,
  type TemplatePanel,
} from './types';
import styles from './AndroidTemplateEditor.module.css';

export function AndroidTemplateSheet({
  panel,
  setPanel,
  editor,
  state,
  trigger,
  onAdd,
}: {
  panel: TemplatePanel;
  setPanel: Dispatch<SetStateAction<TemplatePanel | null>>;
  editor: TemplateEditorState;
  state: UseTemplateEditorStateResult;
  trigger: HTMLElement | null;
  onAdd: (type: PropertyType) => void;
}) {
  const { t } = useTranslation(['settings', 'editor', 'common', 'navigation']);
  const customPages = useSettingsStore((store) => store.settings.customPages);
  const close = () => setPanel(null);
  const idx =
    'fieldId' in panel ? editor.editProperties.findIndex((prop) => prop.id === panel.fieldId) : -1;
  const prop = editor.editProperties[idx];
  const inner = panel.kind === 'contract' || panel.kind === 'role';
  const back = () => {
    if ('fieldId' in panel) setPanel({ kind: 'binding', fieldId: panel.fieldId });
  };
  const titles = {
    page: t('settings:template_category'),
    icon: t('settings:template_icon'),
    add: t('settings:add_field'),
    type: t('settings:template_mobile.field_type'),
    sensitivity: t('settings:search_sensitivity'),
    options: t('common:edit_options'),
    binding: t('settings:plugin_binding'),
    contract: t('settings:select_plugin_contract'),
    role: t('settings:select_role'),
    dynamic: t('settings:template_mobile.dynamic_settings'),
  };
  const title = titles[panel.kind];
  const types = FIELD_TYPES.map((value) => ({
    value,
    label: t(`editor:field_types.${value}`),
    icon: <FieldTypeIcon type={value} size={22} />,
  }));
  const pages: TemplateChoice[] = [
    ...SYSTEM_PAGES.map((value) => ({
      value,
      label: t(`navigation:${value}`),
      group: t('settings:system_pages'),
    })),
    ...customPages
      .filter((page) => !page.deletedAt)
      .map((page) => ({ value: page.id, label: page.name, group: t('settings:custom_pages') })),
    ...customPages
      .filter((page) => page.deletedAt)
      .map((page) => ({
        value: page.id,
        label: page.name,
        group: t('settings:custom_pages_trash'),
        disabled: true,
      })),
  ];
  if (editor.editCategory && !pages.some((page) => page.value === editor.editCategory))
    pages.push({ value: editor.editCategory, label: t('settings:deleted_page'), disabled: true });
  return (
    <AndroidSheet
      className={styles.sheet}
      inertRoot={false}
      title={title}
      onClose={close}
      trigger={trigger}
      material="solid"
      zIndex={5000}
      innerOpen={inner}
      onBack={back}
    >
      <div className={`${styles.config} ${styles.sheetBody}`} data-template-sheet-scroll>
        {inner && (
          <Button variant="tertiary" onClick={back}>
            <ArrowLeft size={18} />
            {t('common:back')}
          </Button>
        )}
        {prop && <p className={styles.hint}>{prop.name}</p>}
        {panel.kind === 'page' && (
          <AndroidTemplateChoiceList
            label={title}
            value={editor.editCategory}
            choices={pages}
            onChoose={(value) => {
              editor.setEditCategory(value);
              close();
            }}
          />
        )}
        {panel.kind === 'icon' && (
          <IconPicker
            value={editor.editIconId}
            onChange={(value) => {
              editor.setEditIconId(value);
              close();
            }}
          />
        )}
        {panel.kind === 'add' && (
          <AndroidTemplateChoiceList
            label={title}
            choices={types}
            onChoose={(value) => onAdd(value as PropertyType)}
          />
        )}
        {panel.kind === 'type' && prop && (
          <AndroidTemplateChoiceList
            label={title}
            choices={types}
            value={prop.type}
            onChoose={(value) => {
              editor.updatePropertyType(idx, value as PropertyType);
              close();
            }}
          />
        )}
        {panel.kind === 'sensitivity' && prop && (
          <AndroidTemplateChoiceList
            label={title}
            value={resolveFieldSensitivity({ fieldId: prop.id, template: prop })}
            choices={SENSITIVITY_LEVELS.map((value) => ({
              value,
              label: t(`editor:sensitivity_levels.${value}`),
            }))}
            onChoose={(value) => {
              editor.updatePropertySensitivity(idx, value as SensitivityLevel);
              close();
            }}
          />
        )}
        {panel.kind === 'options' && prop && (
          <AndroidTemplateOptions
            options={prop.options ?? []}
            onSave={(options) => {
              editor.updatePropertyOptions(idx, options);
              close();
            }}
            onClose={close}
          />
        )}
        {(panel.kind === 'binding' || panel.kind === 'contract' || panel.kind === 'role') &&
          prop && (
            <AndroidTemplateBindings
              editor={editor}
              state={state}
              prop={prop}
              idx={idx}
              view={panel.kind}
              navigate={(kind) => setPanel({ kind, fieldId: prop.id })}
            />
          )}
        {panel.kind === 'dynamic' && <AndroidTemplateDynamicGroup editor={editor} />}
      </div>
    </AndroidSheet>
  );
}
