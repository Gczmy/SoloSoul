import { useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { DeleteButton } from '@/components/ui/DeleteButton';
import { Button } from '@/components/ui/Button';
import { Plus, Pencil, Trash2, Undo2 } from 'lucide-react';
import { formatRelative } from '@/lib/time';
import { ICON_SIZE } from '@/lib/iconSizes';


interface ConversationSummary {
  id: string;
  name: string;
  updatedAt: string;
  messageCount: number;
  deletedAt?: string;
}

interface ConversationSidebarProps {
  conversations: ConversationSummary[];
  trashList: ConversationSummary[];
  currentConvId: string | null;
  showTrash: boolean;
  onNewConversation: () => void;
  onLoadConversation: (id: string) => void;
  onSoftDelete: (id: string) => void;
  onRename: (id: string, newName: string) => void;
  onToggleTrash: () => void;
  onRestore: (id: string) => void;
  onRequestPermanentDelete: (id: string) => void;
  onViewTrashConv: (id: string) => void;
  confirmPermanentDeleteId: string | null;
}

export function ConversationSidebar({
  conversations,
  trashList,
  currentConvId,
  showTrash,
  onNewConversation,
  onLoadConversation,
  onSoftDelete,
  onRename,
  onToggleTrash,
  onRestore,
  onRequestPermanentDelete,
  onViewTrashConv,
  confirmPermanentDeleteId,
}: ConversationSidebarProps) {
  const { t } = useTranslation(['settings', 'common']);
  const [renamingId, setRenamingId] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState('');
  const renameInputRef = useRef<HTMLInputElement>(null);

  const handleRenameStart = (conv: ConversationSummary) => {
    setRenamingId(conv.id);
    setRenameValue(conv.name);
    setTimeout(() => renameInputRef.current?.focus(), 0);
  };

  const handleRenameConfirm = () => {
    if (renamingId && renameValue.trim()) {
      onRename(renamingId, renameValue.trim());
    }
    setRenamingId(null);
  };

  return (
    <div
      style={{
        width: 220,
        minWidth: 180,
        maxWidth: 360,
        borderRight: '1px solid var(--border-subtle)',
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--bg-toolbar)',
        overflow: 'hidden',
        height: '100%',
      }}
    >
      <div
        style={{
          padding: '10px 12px',
          borderBottom: '1px solid var(--border-subtle)',
          flexShrink: 0,
        }}
      >
        <Button
          variant="secondary"
          size="sm"
          onClick={onNewConversation}
          style={{ width: '100%' }}
        >
          <Plus size={ICON_SIZE.sm} style={{ marginRight: 4 }} /> {t('settings:ai_new_conv')}
        </Button>
      </div>
      <div style={{ flex: 1, overflowY: 'auto', padding: '6px 0', minHeight: 0 }}>
        {conversations.length === 0 && (
          <p
            style={{
              fontSize: 'var(--text-caption)',
              color: 'var(--text-tertiary)',
              textAlign: 'center',
              padding: '24px 12px',
            }}
          >
            {t('settings:ai_no_convs')}
          </p>
        )}
        {conversations.map((conv) => (
          <div
            key={conv.id}
            className="conv-item"
            onClick={() => onLoadConversation(conv.id)}
            style={{
              display: 'flex',
              alignItems: 'center',
              gap: 6,
              padding: '8px 12px',
              cursor: 'pointer',
              fontSize: 'var(--text-body-sm)',
              background: currentConvId === conv.id ? 'rgba(91,124,153,0.08)' : 'transparent',
              borderLeft:
                currentConvId === conv.id
                  ? '2px solid var(--accent-primary)'
                  : '2px solid transparent',
            }}
          >
            <div style={{ flex: 1, overflow: 'hidden' }}>
              {renamingId === conv.id ? (
                <input
                  ref={renameInputRef}
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === 'Enter') handleRenameConfirm();
                    if (e.key === 'Escape') setRenamingId(null);
                  }}
                  onBlur={handleRenameConfirm}
                  style={{
                    width: '100%',
                    padding: '2px 4px',
                    fontSize: 'var(--text-body-sm)',
                    border: '1px solid var(--accent-primary)',
                    borderRadius: 4,
                    background: 'var(--bg-elevated)',
                    color: 'var(--text-primary)',
                    outline: 'none',
                  }}
                  autoFocus
                />
              ) : (
                <>
                  <div
                    style={{
                      fontWeight: 500,
                      whiteSpace: 'nowrap',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      color: 'var(--text-primary)',
                    }}
                  >
                    {conv.name || t('settings:ai_untitled')}
                  </div>
                  <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)', marginTop: 1 }}>
                    {formatRelative(conv.updatedAt)}
                  </div>
                </>
              )}
            </div>
            <button
              onClick={(e) => {
                e.stopPropagation();
                handleRenameStart(conv);
              }}
              title={t('common:rename')}
              style={{
                padding: 3,
                borderRadius: 4,
                border: 'none',
                background: 'transparent',
                cursor: 'pointer',
                color: 'var(--text-tertiary)',
                opacity: 0,
              }}
              className="sidebar-action-btn"
            >
              <Pencil size={ICON_SIZE.xs} />
            </button>
            <DeleteButton
              onClick={(e) => {
                e.stopPropagation();
                onSoftDelete(conv.id);
              }}
              title={t('common:delete')}
              iconOnly
            />
          </div>
        ))}
      </div>

      {/* Trash entry */}
      <div style={{ borderTop: '1px solid var(--border-subtle)', flexShrink: 0 }}>
        <button
          onClick={onToggleTrash}
          style={{
            width: '100%',
            display: 'flex',
            alignItems: 'center',
            gap: 8,
            padding: '10px 12px',
            border: 'none',
            background: showTrash ? 'rgba(91,124,153,0.08)' : 'transparent',
            cursor: 'pointer',
            fontSize: 'var(--text-body-sm)',
            color: 'var(--text-tertiary)',
          }}
        >
          <Trash2 size={ICON_SIZE.sm} />
          <span>{t('settings:ai_trash')}</span>
          {trashList.length > 0 && (
            <span
              style={{
                marginLeft: 'auto',
                fontSize: 'var(--text-badge)',
                background: 'rgba(231,76,60,0.15)',
                color: '#e74c3c',
                padding: '1px 6px',
                borderRadius: 8,
              }}
            >
              {trashList.length}
            </span>
          )}
        </button>
        {showTrash && (
          <div
            style={{
              maxHeight: 200,
              overflowY: 'auto',
              borderTop: '1px solid var(--border-subtle)',
            }}
          >
            {trashList.length === 0 ? (
              <p
                style={{
                  fontSize: 'var(--text-caption)',
                  color: 'var(--text-tertiary)',
                  textAlign: 'center',
                  padding: '16px 12px',
                }}
              >
                {t('settings:ai_trash_empty')}
              </p>
            ) : (
              trashList.map((conv) => (
                <div
                  key={conv.id}
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 4,
                    padding: '6px 12px',
                    fontSize: 'var(--text-caption)',
                  }}
                >
                  <div
                    style={{ flex: 1, overflow: 'hidden', cursor: 'pointer' }}
                    onClick={() => onViewTrashConv(conv.id)}
                  >
                    <div
                      style={{
                        whiteSpace: 'nowrap',
                        overflow: 'hidden',
                        textOverflow: 'ellipsis',
                        color: 'var(--text-secondary)',
                      }}
                    >
                      {conv.name || t('settings:ai_untitled')}
                    </div>
                    <div style={{ fontSize: 'var(--text-badge)', color: 'var(--text-tertiary)' }}>
                      {conv.deletedAt ? formatRelative(conv.deletedAt) : ''}
                    </div>
                  </div>
                  <button
                    onClick={() => onRestore(conv.id)}
                    title="恢复"
                    style={{
                      padding: 3,
                      borderRadius: 4,
                      border: 'none',
                      background: 'transparent',
                      cursor: 'pointer',
                      color: '#27ae60',
                    }}
                  >
                    <Undo2 size={ICON_SIZE.xs} />
                  </button>
                  {confirmPermanentDeleteId === conv.id ? (
                    <button
                      onClick={() => onRequestPermanentDelete(conv.id)}
                      title={t('settings:ai_confirm_delete')}
                      style={{
                        padding: '2px 6px',
                        borderRadius: 4,
                        border: '1px solid #e74c3c',
                        background: '#e74c3c',
                        cursor: 'pointer',
                        color: 'white',
                        fontSize: 'var(--text-badge)',
                      }}
                    >
                      {t('settings:ai_confirm_btn')}
                    </button>
                  ) : (
                    <DeleteButton
                      onClick={() => onRequestPermanentDelete(conv.id)}
                      title="永久删除"
                      iconOnly
                    />
                  )}
                </div>
              ))
            )}
          </div>
        )}
      </div>
    </div>
  );
}
