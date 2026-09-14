import { useTranslation } from 'react-i18next';
import { useStore } from 'zustand';
import { setOpaqueWindow } from '@/lib/nativeWindow';
import { isMacOSSync } from '@/lib/platform';
import { useUiStore } from '@/stores/uiStore';
import { windowAppearanceStore } from '@/stores/windowAppearanceStore';

export function WindowTransparencySetting({ compact = false }: { compact?: boolean }) {
  const { t } = useTranslation('settings');
  const opaque = useStore(windowAppearanceStore, (s) => s.opaque);
  const isSaving = useStore(windowAppearanceStore, (s) => s.isSaving);
  if (!isMacOSSync()) return null;

  return (
    <div style={{ marginTop: compact ? 12 : 0, textAlign: 'left' }}>
      {!compact && (
        <h3 style={{ fontSize: 'var(--text-sm)', fontWeight: 600, marginBottom: 12 }}>
          {t('window_background')}
        </h3>
      )}
      <label
        style={{
          display: 'flex',
          justifyContent: compact ? 'center' : undefined,
          alignItems: 'center',
          gap: 8,
          fontSize: compact ? 'var(--text-caption)' : 'var(--text-sm)',
          color: compact ? 'var(--text-secondary)' : 'var(--text-primary)',
          cursor: isSaving ? 'wait' : 'pointer',
        }}
        title={t('opaque_window_description')}
      >
        <input
          type="checkbox"
          checked={opaque}
          disabled={isSaving}
          style={{ accentColor: 'var(--accent-primary)' }}
          onChange={(event) => {
            void setOpaqueWindow(event.target.checked).catch(() => {
              useUiStore.getState().showToast({
                type: 'error',
                message: t('window_background_failed'),
              });
            });
          }}
        />
        {t('opaque_window')}
      </label>
      {!compact && (
        <p
          style={{ fontSize: 'var(--text-caption)', color: 'var(--text-secondary)', marginTop: 8 }}
        >
          {t('opaque_window_description')}
        </p>
      )}
    </div>
  );
}
