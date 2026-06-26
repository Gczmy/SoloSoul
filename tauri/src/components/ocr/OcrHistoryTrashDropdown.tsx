import { useTranslation } from 'react-i18next';
import { FileText, Trash2, RotateCcw } from 'lucide-react';
import { useOcrScanStore, type OcrScanEntry } from '@/stores/ocrScanStore';
import { ICON_SIZE } from '@/lib/iconSizes';


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
          onMouseEnter={(e) => {
            if (showTrash) {
              e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }
          }}
          onMouseLeave={(e) => {
            if (showTrash) {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = 'var(--text-tertiary)';
            }
          }}
          style={{
            fontSize: 'var(--text-caption)',
            padding: '4px 8px',
            borderRadius: 6,
            border: 'none',
            background: !showTrash ? 'var(--bg-toolbar)' : 'transparent',
            color: !showTrash ? 'var(--text-primary)' : 'var(--text-tertiary)',
            cursor: 'pointer',
            transition: 'all 0.15s ease',
          }}
        >
          {t('ocr:history_tab')} ({activeHistory.length})
        </button>
        <button
          onClick={() => onShowTrashChange(true)}
          onMouseEnter={(e) => {
            if (!showTrash) {
              e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
              e.currentTarget.style.color = 'var(--accent-primary)';
            }
          }}
          onMouseLeave={(e) => {
            if (!showTrash) {
              e.currentTarget.style.background = 'transparent';
              e.currentTarget.style.color = 'var(--text-tertiary)';
            }
          }}
          style={{
            fontSize: 'var(--text-caption)',
            padding: '4px 8px',
            borderRadius: 6,
            border: 'none',
            background: showTrash ? 'var(--bg-toolbar)' : 'transparent',
            color: showTrash ? 'var(--text-primary)' : 'var(--text-tertiary)',
            cursor: 'pointer',
            transition: 'all 0.15s ease',
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
                  className={isSelected ? 'ocr-history-item ocr-history-item--selected' : 'ocr-history-item'}
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
                      {new Date(entry.timestamp).toLocaleString()} · {entry.mode === 'mrz' ? 'MRZ' : 'OCR'}
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
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  store.softDeleteEntry(entry.id);
                }}
                title={t('common:delete')}
                className="ocr-history-item__btn"
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 12%, transparent)';
                  e.currentTarget.style.color = '#e74c3c';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                  e.currentTarget.style.color = 'var(--text-tertiary)';
                }}
                style={{
                  padding: 2,
                  borderRadius: 4,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: 'var(--text-tertiary)',
                  flexShrink: 1,
                  transition: 'all 0.15s ease',
                }}
              >
                <Trash2 size={ICON_SIZE.xs} />
              </button>
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
              <FileText size={ICON_SIZE.xs} style={{ color: 'var(--text-tertiary)', flexShrink: 0 }} />
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
                <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: 1 }}>
                  {new Date(entry.timestamp).toLocaleString()}
                </div>
              </div>
              <button
                onClick={() => store.restoreEntry(entry.id)}
                title={t('ocr:restore')}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                }}
                style={{
                  padding: 2,
                  borderRadius: 4,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: 'var(--accent-primary)',
                  transition: 'all 0.15s ease',
                }}
              >
                <RotateCcw size={ICON_SIZE.xs} />
              </button>
              <button
                onClick={() => store.permanentlyDeleteEntry(entry.id)}
                title={t('ocr:permanently_delete')}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background = 'color-mix(in srgb, #e74c3c 12%, transparent)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                }}
                style={{
                  padding: 2,
                  borderRadius: 4,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: '#e74c3c',
                  transition: 'all 0.15s ease',
                }}
              >
                <Trash2 size={ICON_SIZE.xs} />
              </button>
            </div>
          ))}
          {trash.length > 1 && (
            <button
              onClick={() => store.clearTrash()}
              onMouseEnter={(e) => {
                e.currentTarget.style.background = '#e74c3c';
                e.currentTarget.style.color = 'white';
              }}
              onMouseLeave={(e) => {
                e.currentTarget.style.background = 'transparent';
                e.currentTarget.style.color = '#e74c3c';
              }}
              style={{
                margin: '6px 12px',
                padding: '6px 10px',
                borderRadius: 6,
                border: '1px solid #e74c3c',
                background: 'transparent',
                color: '#e74c3c',
                fontSize: 'var(--text-caption)',
                cursor: 'pointer',
                textAlign: 'center',
                transition: 'all 0.15s ease',
              }}
            >
              {t('ocr:clear_trash')}
            </button>
          )}
        </>
      )}
    </div>
  );
}
