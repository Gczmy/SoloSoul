import type { TFunction } from 'i18next';
import { TransferButton } from '@/components/transfer/TransferButton';
import type { ImportStrategy } from '@/types/exportImport';

/**
 * ImportSection 的操作区（P046 拆分：展示子组件）。
 * 快速导入按钮 / 高级策略选择器（radio + 导入按钮）。
 */
export function ImportActionSection({
  showStrategySelector,
  importStrategy,
  isImporting,
  importPw,
  importTotalSelected,
  onSetShowStrategySelector,
  onSetStrategy,
  onImport,
  t,
}: {
  showStrategySelector: boolean;
  importStrategy: ImportStrategy;
  isImporting: boolean;
  importPw: string;
  importTotalSelected: number;
  onSetShowStrategySelector: (v: boolean) => void;
  onSetStrategy: (s: ImportStrategy) => void;
  onImport: () => void;
  t: TFunction;
}) {
  return (
    <>
      {/* Action buttons */}
      {!showStrategySelector ? (
        <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
          <TransferButton onClick={() => onSetShowStrategySelector(true)}>
            {t('settings:advanced_import')}
          </TransferButton>
          <TransferButton
            variant="accent"
            onClick={onImport}
            disabled={!importPw || isImporting || importTotalSelected === 0}
            busy={isImporting}
          >
            {isImporting
              ? t('common:loading', { defaultValue: '...' })
              : `${t('settings:quick_import')} (${importTotalSelected})`}
          </TransferButton>
        </div>
      ) : (
        <div
          style={{
            marginTop: 12,
            padding: 12,
            border: '1px solid var(--border-subtle)',
            borderRadius: 8,
          }}
        >
          <h4 style={{ fontSize: 'var(--text-body-sm)', fontWeight: 600, marginBottom: 8 }}>
            {t('settings:import_strategy_title')}
          </h4>
          {(['skipExisting', 'overwrite', 'keepBoth'] as ImportStrategy[]).map((s) => (
            <label
              key={s}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '6px 0',
                cursor: 'pointer',
                fontSize: 'var(--text-body-sm)',
              }}
            >
              <input
                type="radio"
                checked={importStrategy === s}
                onChange={() => onSetStrategy(s)}
                style={{ accentColor: 'var(--accent-primary)' }}
              />
              <div>
                <strong>{t(`settings:strategy_${s}`)}</strong>
                <p
                  style={{
                    fontSize: 'var(--text-badge)',
                    color: 'var(--text-tertiary)',
                    margin: 1,
                  }}
                >
                  {t(`settings:strategy_${s}_desc`)}
                </p>
              </div>
            </label>
          ))}
          <div style={{ marginTop: 8, display: 'flex', gap: 8 }}>
            <TransferButton onClick={() => onSetShowStrategySelector(false)}>
              {t('common:cancel')}
            </TransferButton>
            <TransferButton
              variant="accent"
              onClick={onImport}
              disabled={!importPw || isImporting || importTotalSelected === 0}
              busy={isImporting}
            >
              {isImporting
                ? t('common:loading', { defaultValue: '...' })
                : `${t('settings:import_action')} (${importTotalSelected})`}
            </TransferButton>
          </div>
        </div>
      )}
    </>
  );
}
