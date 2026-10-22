import { useTranslation } from 'react-i18next';
import { ChevronRight } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { deriveContractBindings } from '@/lib/plugin';
import type { TemplateProperty } from '@/types/template';
import type { UseTemplateEditorStateResult } from '../useTemplateEditorState';
import { AndroidTemplateChoiceList } from './AndroidTemplateChoiceList';
import type { TemplateEditorState } from './types';
import styles from './AndroidTemplateEditor.module.css';

export function AndroidTemplateBindings({
  editor,
  state,
  prop,
  idx,
  view,
  navigate,
}: {
  editor: TemplateEditorState;
  state: UseTemplateEditorStateResult;
  prop: TemplateProperty;
  idx: number;
  view: 'binding' | 'contract' | 'role';
  navigate: (kind: 'binding' | 'contract' | 'role') => void;
}) {
  const { t } = useTranslation(['settings', 'common']);
  const contractId = state.selectedContractId[prop.id] ?? '';
  const roleId = state.selectedRoleId[prop.id] ?? '';
  const selected = state.flattenContracts.find(
    (entry) => `${entry.pluginId}::${entry.contract.typeId}` === contractId,
  );
  const role = selected?.contract.roles.find((role) => role.roleId === roleId);
  const bindings = prop.contractBindings?.length
    ? prop.contractBindings
    : prop.contractField && editor.editContractTypeId
      ? deriveContractBindings(editor.editContractTypeId, prop.id, state.installedPlugins)
      : [];
  if (view === 'contract') {
    return (
      <AndroidTemplateChoiceList
        label={t('settings:select_plugin_contract')}
        value={contractId}
        choices={state.flattenContracts.map((entry) => ({
          value: `${entry.pluginId}::${entry.contract.typeId}`,
          label: entry.contract.displayName || entry.contract.typeId,
          description: entry.pluginName,
        }))}
        onChoose={(value) => {
          state.setSelectedContractId((previous) => ({ ...previous, [prop.id]: value }));
          state.setSelectedRoleId((previous) => ({ ...previous, [prop.id]: '' }));
          navigate('binding');
        }}
      />
    );
  }
  if (view === 'role') {
    return (
      <AndroidTemplateChoiceList
        label={t('settings:select_role')}
        value={roleId}
        choices={(selected?.contract.roles ?? []).map((role) => ({
          value: role.roleId,
          label: `${role.label || role.roleId}${role.required ? ' *' : ''}`,
        }))}
        onChoose={(value) => {
          state.setSelectedRoleId((previous) => ({ ...previous, [prop.id]: value }));
          navigate('binding');
        }}
      />
    );
  }
  return (
    <div className={styles.config}>
      {bindings.map((binding) => {
        const entry = state.flattenContracts.find(
          (item) => item.contract.typeId === binding.contractTypeId,
        );
        const label =
          entry?.contract.roles.find((role) => role.roleId === binding.roleId)?.label ||
          binding.roleId;
        return (
          <div className={styles.binding} key={`${binding.contractTypeId}::${binding.roleId}`}>
            <span>
              {entry?.pluginName ?? binding.contractTypeId}
              <small className={styles.hint}>
                {entry?.contract.displayName || binding.contractTypeId} / {label}
              </small>
            </span>
            {prop.contractBindings?.length ? (
              <DeleteButton
                iconOnly
                title={`${t('common:remove')}: ${label}`}
                onClick={() =>
                  editor.updatePropertyContractBindings(
                    idx,
                    bindings.filter((item) => item !== binding),
                  )
                }
              />
            ) : null}
          </div>
        );
      })}
      {state.flattenContracts.length ? (
        <>
          <button type="button" className={styles.choice} onClick={() => navigate('contract')}>
            <span>
              <small>{t('settings:select_plugin_contract')}</small>
              {selected?.contract.displayName ||
                selected?.contract.typeId ||
                t('settings:select_plugin_contract')}
            </span>
            <ChevronRight size={20} />
          </button>
          <button
            type="button"
            className={styles.choice}
            disabled={!selected}
            onClick={() => navigate('role')}
          >
            <span>
              <small>{t('settings:select_role')}</small>
              {role?.label || role?.roleId || t('settings:select_role')}
            </span>
            <ChevronRight size={20} />
          </button>
          <Button
            disabled={
              !selected ||
              !role ||
              bindings.some(
                (binding) =>
                  binding.contractTypeId === selected.contract.typeId &&
                  binding.roleId === role.roleId,
              )
            }
            onClick={() => {
              if (!selected || !role) return;
              editor.updatePropertyContractBindings(idx, [
                ...bindings,
                { contractTypeId: selected.contract.typeId, roleId: role.roleId },
              ]);
              if (editor.editContractTypeId !== selected.contract.typeId)
                editor.setEditContractTypeId(selected.contract.typeId);
            }}
          >
            {t('common:add')}
          </Button>
        </>
      ) : (
        <p className={styles.hint}>{t('settings:no_plugin_contracts_available')}</p>
      )}
    </div>
  );
}
