import { DeleteButton } from '@/components/ui/DeleteButton';
import { useTranslation } from 'react-i18next';
import { FileText, RotateCcw } from 'lucide-react';
import { useOcrScanStore, type OcrScanEntry } from '@/stores/ocrScanStore';
import { ICON_SIZE } from '@/lib/constants';

interface OcrHistoryTrashDropdownProps {
  showTrash: boolean;
  onShowTrashChange: (v: boolean) => void;
  activeHistory: OcrScanEntry[];
  trash: OcrScanEntry[];
  currentEntryId: string | null;
  onSelectEntry: (entry: OcrScanEntry) => void;
}

export function OcrHistoryTrashDropdown({
  showTrash,
  onShowTrashChange,
  activeHistory,
  trash,
  currentEntryId,
  onSelectEntry,
}: OcrHistoryTrashDropdownProps) {
  const { t } = useTranslation(['ocr', 'common']);
  const store = useOcrScanStore();

  return (
    <div
      style={{
        position: 'absolute',
        top: 44,
        left: 8,
        right: 8,
        maxHeight: 260,
        background: 'var(--bg-elevated)',
        borderRadius: 10,
        border: '1px solid var(--border-subtle)',
        boxShadow: 'var(--shadow-lg)',
        zIndex: 10,
        overflowY: 'auto',
        padding: '6px 2px',
        display: 'flex',
        flexDirection: 'column',
      }}
    >
      {/* Tabs */}
      <div
        style={{
          display: 'flex',
          gap: 4,
          padding: '0 10px 6px',
          borderBottom: '1px solid var(--border-subtle)',
        }}
      >
        <button
          onClick={() => onShowTrashChange(false)}
          className={!showTrash ? 'interactive-toolbar selected-neutral' : 'interactive-accent'}
          style={{
            fontSize: 'var(--text-caption)',
            padding: '4px 8px',
            borderRadius: 6,
            border: 'none',
            cursor: 'pointer',
          }}
        >
          {t('ocr:history_tab')} ({activeHistory.length})
        </button>
        <button
          onClick={() => onShowTrashChange(true)}
          className={showTrash ? 'interactive-toolbar selected-neutral' : 'interactive-accent'}
          style={{
            fontSize: 'var(--text-caption)',
            padding: '4px 8px',
            borderRadius: 6,
            border: 'none',
            cursor: 'pointer',
          }}
        >
          {t('ocr:trash_tab')} ({trash.length})
        </button>
      </div>

      {!showTrash ? (
        activeHistory.length === 0 ? (
          <p
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
              padding: '16px 12px',
              margin: 0,
            }}
          >
            {t('ocr:no_history')}
          </p>
        ) : (
          <>
            {activeHistory.map((entry) => {
              const isSelected = currentEntryId === entry.id;
              const hasError = !!entry.error;
              return (
                <div
                  key={entry.id}
                  className={
                    isSelected ? 'ocr-history-item ocr-history-item--selected' : 'ocr-history-item'
                  }
                  onClick={() => onSelectEntry(entry)}
                  title={entry.fileName}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 8,
                    padding: '7px 12px',
                    cursor: 'pointer',
                    fontSize: 'var(--text-caption)',
                  }}
                >
                  <FileText
                    size={ICON_SIZE.xs}
                    style={{
                      color: hasError ? '#e74c3c' : 'var(--text-tertiary)',
                      flexShrink: 0,
                    }}
                  />
                  <div style={{ flex: 1, overflow: 'hidden' }}>
                    <div
                      style={{
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        color: 'var(--text-primary)',
                        fontWeight: isSelected ? 500 : 400,
                      }}
                    >
                      {entry.fileName}
                    </div>
                    <div
                      style={{
                        fontSize: 'var(--text-badge)',
                        color: hasError ? '#e74c3c' : 'var(--text-tertiary)',
                        marginTop: 1,
                      }}
                    >
                      {new Date(entry.timestamp).toLocaleString()} ·{' '}
                      {entry.mode === 'mrz' ? 'MRZ' : 'OCR'}
                      {hasError ? ` · ${t('common:error')}` : ''}
                    </div>
                  </div>
                  {isSelected && (
                    <span
                      style={{
                        width: 6,
                        height: 6,
                        borderRadius: '50%',
                        background: 'var(--accent-primary)',
                        flexShrink: 0,
                      }}
                    />
                  )}
                  <DeleteButton
                    onClick={(e) => {
                      e.stopPropagation();
                      store.softDeleteEntry(entry.id);
                    }}
                    title={t('common:delete')}
                    iconOnly
                  />
                </div>
              );
            })}
          </>
        )
      ) : trash.length === 0 ? (
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-tertiary)',
            textAlign: 'center',
            padding: '16px 12px',
            margin: 0,
          }}
        >
          {t('ocr:trash_empty')}
        </p>
      ) : (
        <>
          {trash.map((entry) => (
            <div
              key={entry.id}
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 8,
                padding: '7px 12px',
                fontSize: 'var(--text-caption)',
              }}
            >
              <FileText
                size={ICON_SIZE.xs}
                style={{ color: 'var(--text-tertiary)', flexShrink: 0 }}
              />
              <div style={{ flex: 1, overflow: 'hidden' }}>
                <div
                  style={{
                    whiteSpace: 'nowrap',
                    overflow: 'hidden',
                    textOverflow: 'ellipsis',
                    color: 'var(--text-secondary)',
                  }}
                >
                  {entry.fileName}
                </div>
                <div
                  style={{
                    fontSize: 'var(--text-badge)',
                    color: 'var(--text-tertiary)',
                    marginTop: 1,
                  }}
                >
                  {new Date(entry.timestamp).toLocaleString()}
                </div>
              </div>
              <button
                onClick={() => store.restoreEntry(entry.id)}
                title={t('ocr:restore')}
                className="interactive-icon"
                style={{
                  padding: 2,
                  borderRadius: 4,
                  border: 'none',
                  cursor: 'pointer',
                  color: 'var(--accent-primary)',
                }}
              >
                <RotateCcw size={ICON_SIZE.xs} />
              </button>
              <DeleteButton
                onClick={() => store.permanentlyDeleteEntry(entry.id)}
                title={t('ocr:permanently_delete')}
                iconOnly
              />
            </div>
          ))}
          {trash.length > 1 && (
            <DeleteButton onClick={() => store.clearTrash()} title={t('ocr:clear_trash')}>
              {t('ocr:clear_trash')}
            </DeleteButton>
          )}
        </>
      )}
    </div>
  );
}
