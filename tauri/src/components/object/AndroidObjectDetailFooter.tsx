import { useEffect, useRef, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { CircleHelp, Clock, MoreHorizontal, Paperclip, Pencil, Trash2 } from 'lucide-react';
import { AndroidSheet } from '@/components/android/AndroidSheet';
import { GuidePageContent } from '@/components/guide/GuidePageContent';
import type { GuidePage } from '@/components/guide/PageGuide';

/** 安卓详情底栏：内容/编辑直达，其余操作在底部面板内展开。 */
export function AndroidObjectDetailFooter({
  objectName,
  attachmentCount,
  guidePages,
  onHistory,
  onAttachments,
  onEdit,
  onDelete,
}: {
  objectName: string;
  attachmentCount?: number;
  guidePages: GuidePage[];
  onHistory: () => void;
  onAttachments: () => void;
  onEdit?: () => void;
  onDelete: () => void;
}) {
  const { t } = useTranslation('common');
  const navigate = useNavigate();
  const [panel, setPanel] = useState<'menu' | 'guide' | null>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const guideContent = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (panel === 'guide') guideContent.current?.focus({ preventScroll: true });
  }, [panel]);
  const close = () => setPanel(null);
  const select = (action: () => void) => {
    close();
    action();
  };

  return (
    <>
      <div className="android-object-detail-footer">
        <button
          type="button"
          className="android-object-detail-action"
          aria-label={
            attachmentCount === undefined
              ? t('attachments')
              : t('material.attachments_with_count', { count: attachmentCount })
          }
          onClick={onAttachments}
        >
          <Paperclip size={20} aria-hidden="true" />
          <span>{t('attachments')}</span>
          {attachmentCount !== undefined && (
            <span className="android-object-attachment-count" aria-hidden="true">
              {attachmentCount > 99 ? '99+' : attachmentCount}
            </span>
          )}
        </button>
        {onEdit && (
          <button
            type="button"
            className="android-object-detail-action android-object-detail-edit"
            onClick={onEdit}
          >
            <Pencil size={20} aria-hidden="true" />
            <span>{t('edit')}</span>
          </button>
        )}
        <button
          ref={trigger}
          type="button"
          className="android-icon-button"
          aria-label={t('material.object_actions', { name: objectName })}
          aria-haspopup="dialog"
          aria-expanded={panel !== null}
          onClick={() => setPanel('menu')}
        >
          <MoreHorizontal size={24} aria-hidden="true" />
        </button>
      </div>
      {panel && (
        <AndroidSheet
          title={panel === 'guide' ? guidePages[0]?.title || t('guide') : t('more_actions')}
          onClose={close}
          trigger={trigger.current}
          zIndex="var(--z-preview-overlay)"
        >
          {panel === 'menu' ? (
            <>
              <p className="android-object-menu-name">{objectName}</p>
              <div className="android-object-menu-actions">
                <button type="button" onClick={() => select(onHistory)}>
                  <Clock size={24} aria-hidden="true" />
                  <span>{t('history')}</span>
                </button>
                {guidePages.length > 0 && (
                  <button type="button" onClick={() => setPanel('guide')}>
                    <CircleHelp size={24} aria-hidden="true" />
                    <span>{t('guide')}</span>
                  </button>
                )}
                <hr className="android-object-menu-divider" />
                <button type="button" data-danger onClick={() => select(onDelete)}>
                  <Trash2 size={24} aria-hidden="true" />
                  <span>{t('delete')}</span>
                </button>
              </div>
            </>
          ) : (
            <div ref={guideContent} tabIndex={-1} className="android-object-detail-guide">
              {guidePages.map((page, index) => (
                <section key={page.title}>
                  {index > 0 && <h3>{page.title}</h3>}
                  <GuidePageContent
                    page={page}
                    t={t}
                    onHelpLinkClick={(href) => select(() => navigate(href))}
                  />
                </section>
              ))}
            </div>
          )}
        </AndroidSheet>
      )}
    </>
  );
}
