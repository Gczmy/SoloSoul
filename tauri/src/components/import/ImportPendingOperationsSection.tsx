import { useTranslation } from 'react-i18next';
import { Card } from '@/components/ui/Card';
import { TransferButton } from '@/components/transfer/TransferButton';
import { SecurePasswordInput } from '@/components/forms/PasswordInput';
import type { ImportOperationsUi } from '@/types/exportImport';

/** 独立继续入口，不要求 manifest、解密预览或重新选择对象。 */
export function ImportPendingOperationsSection({ state }: { state: ImportOperationsUi }) {
  const { t } = useTranslation(['settings', 'common']);
  const selected = state.selected;
  const unavailableRecoveryCredential =
    selected?.sourceKind === 'recovery' && selected.passwordRequired;
  return (
    <Card>
      <div
        style={{ display: 'flex', alignItems: 'center', justifyContent: 'space-between', gap: 8 }}
      >
        <h3 style={{ fontSize: 'var(--text-body)', fontWeight: 600 }}>
          {t('settings:import_pending_title')}
        </h3>
        <TransferButton
          disabled={state.loading || state.busy}
          busy={state.loading}
          onClick={() => void state.onRefresh()}
        >
          {t('common:refresh')}
        </TransferButton>
      </div>
      {!state.loading && state.items.length === 0 && !state.currentId && (
        <p style={{ color: 'var(--text-secondary)', fontSize: 'var(--text-body-sm)' }}>
          {t('settings:import_pending_empty')}
        </p>
      )}
      {state.items.map((item) => (
        <div
          key={item.operationId}
          style={{ display: 'flex', alignItems: 'center', gap: 8, marginTop: 8 }}
        >
          <span style={{ flex: 1, overflowWrap: 'anywhere' }}>{item.sourceName}</span>
          <TransferButton
            disabled={state.busy || state.loadingDetails}
            onClick={() => void state.onSelect(item.operationId)}
          >
            {t('settings:import_resume')}
          </TransferButton>
        </div>
      ))}
      {state.currentId && !selected && (
        <div style={{ marginTop: 12 }}>
          <p>{t('settings:import_resume_check_status')}</p>
          <TransferButton
            disabled={state.busy || state.loadingDetails}
            busy={state.busy || state.loadingDetails}
            onClick={() => void state.onRetry()}
          >
            {t('settings:import_resume_check_and_continue')}
          </TransferButton>
        </div>
      )}
      {selected && (
        <section aria-label={t('settings:import_resume')} style={{ marginTop: 12 }}>
          <p style={{ overflowWrap: 'anywhere' }}>{selected.sourceName}</p>
          <p style={{ fontSize: 'var(--text-body-sm)', color: 'var(--text-secondary)' }}>
            {t('settings:import_resume_counts', {
              objects: selected.outcome.objectCount,
              attachments: selected.outcome.attachmentCount,
            })}
          </p>
          <p style={{ fontSize: 'var(--text-caption)', color: 'var(--text-tertiary)' }}>
            {t('settings:import_resume_options_readonly')}
          </p>
          {selected.phase === 'complete' ? (
            <p role="status">{t('settings:import_resume_complete')}</p>
          ) : (
            <>
              {unavailableRecoveryCredential ? (
                <p role="alert">{t('settings:import_resume_recovery_credential_unavailable')}</p>
              ) : (
                selected.passwordRequired && (
                  <SecurePasswordInput
                    value={state.password}
                    onChange={state.onSetPassword}
                    label={t('settings:import_resume_password')}
                    disabled={state.busy}
                    showHintButton={false}
                    autoComplete="off"
                  />
                )
              )}
              {selected.sourceRequired && (
                <div style={{ marginTop: 8 }}>
                  <p>{t('settings:import_resume_source_required')}</p>
                  {state.replacementSource && (
                    <p style={{ overflowWrap: 'anywhere' }}>{state.replacementSource}</p>
                  )}
                  <TransferButton disabled={state.busy} onClick={() => void state.onPickSource()}>
                    {t('settings:select_file')}
                  </TransferButton>
                </div>
              )}
              <TransferButton
                variant="accent"
                disabled={!state.canResume}
                busy={state.busy}
                onClick={() => void state.onResume()}
              >
                {t('settings:import_resume')}
              </TransferButton>
            </>
          )}
        </section>
      )}
      {(state.currentId || selected || state.busy) && (
        <div style={{ display: 'flex', gap: 8, marginTop: 12, flexWrap: 'wrap' }}>
          <TransferButton onClick={state.onContinueLater}>
            {t('settings:import_resume_later')}
          </TransferButton>
          <TransferButton onClick={state.onNewImport}>
            {t('settings:import_new_task')}
          </TransferButton>
        </div>
      )}
      {state.busy && (
        <p role="status" style={{ fontSize: 'var(--text-caption)' }}>
          {t('settings:import_resume_background_note')}
        </p>
      )}
    </Card>
  );
}
