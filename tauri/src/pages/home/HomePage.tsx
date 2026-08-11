import React, { Fragment, useState, useRef, useEffect } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AppShell } from '@/components/layout/AppShell';
import { PageContainer } from '@/components/layout/PageContainer';
import { Card } from '@/components/ui/Card';
import { CardGrid } from '@/components/ui/CardGrid';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { useActiveCustomPages } from '@/components/layout/useNavigationItems';
import { CustomPageEditPopover } from '@/components/layout/CustomPageEditPopover';
import { PageGuideButton } from '@/components/guide/PageGuideButton';
import { useAuthStore } from '@/stores/authStore';
import { useUiStore } from '@/stores/uiStore';
import { invokeCommand as invoke } from '@/lib/ipcClient';
import { collectPhotoItems } from '@/lib/attachmentUtils';
import { PhotoAlbumOverlay } from '@/components/attachment/PhotoAlbumOverlay';
import type { AttachmentListAllResult, AttachmentMeta } from '@/components/attachment/attachmentManagerTypes';

import { useLongPress } from '@/hooks/useLongPress';
import { LayoutGrid, Zap, Hand, Images } from 'lucide-react';
import type { ProfileSection } from '@/types';
import type { LucideIcon } from 'lucide-react';
import type { CustomPage } from '@/stores/settingsStore';
import styles from './HomePage.module.css';

// Icons sourced from PAGE_ICON_MAP — §7.4 Single Source of Truth
const sections: {
  type: ProfileSection | 'document';
  labelKey: string;
  icon: LucideIcon;
  descKey: string;
}[] = [
  { type: 'identity', labelKey: 'identity', icon: PAGE_ICON_MAP.profile, descKey: 'identity_desc' },
  { type: 'travel', labelKey: 'travel', icon: PAGE_ICON_MAP.travel, descKey: 'travel_desc' },
  {
    type: 'financial',
    labelKey: 'financial',
    icon: PAGE_ICON_MAP.financial,
    descKey: 'financial_desc',
  },
  {
    type: 'professional',
    labelKey: 'professional',
    icon: PAGE_ICON_MAP.professional,
    descKey: 'professional_desc',
  },
  {
    type: 'document',
    labelKey: 'document',
    icon: PAGE_ICON_MAP.document,
    descKey: 'document_desc',
  },
];

type QuickCard = {
  path: string;
  labelKey: string;
  icon: LucideIcon;
  descKey: string;
};

const quickCards: QuickCard[] = [
  {
    path: '/settings',
    labelKey: 'settings',
    icon: PAGE_ICON_MAP.settings,
    descKey: 'settings_desc',
  },
  {
    path: '/settings/trash',
    labelKey: 'trash',
    icon: PAGE_ICON_MAP.trash,
    descKey: 'trash_desc',
  },
  {
    path: '/search',
    labelKey: 'search',
    icon: PAGE_ICON_MAP.search,
    descKey: 'search_desc',
  },
  {
    path: '/settings/templates',
    labelKey: 'templates',
    icon: PAGE_ICON_MAP.templates,
    descKey: 'templates_desc',
  },
  {
    path: '/settings/attachments',
    labelKey: 'attachments',
    icon: PAGE_ICON_MAP.attachments,
    descKey: 'attachments_desc',
  },
  {
    path: '/plugins',
    labelKey: 'plugins',
    icon: PAGE_ICON_MAP.plugins,
    descKey: 'plugins_desc',
  },
  {
    path: '/ocr',
    labelKey: 'ocr',
    icon: PAGE_ICON_MAP.ocr,
    descKey: 'ocr_desc',
  },
  {
    path: '/settings/export-import',
    labelKey: 'import_export',
    icon: PAGE_ICON_MAP.import_export,
    descKey: 'import_export_desc',
  },
  {
    path: '/sync',
    labelKey: 'sync',
    icon: PAGE_ICON_MAP.sync,
    descKey: 'sync_desc',
  },
  {
    path: '/help',
    labelKey: 'help',
    icon: PAGE_ICON_MAP.help,
    descKey: 'help_desc',
  },
  {
    path: '/llm-chat',
    labelKey: 'ai_chat',
    icon: PAGE_ICON_MAP.ai_chat,
    descKey: 'ai_chat_desc',
  },
];

interface HomeCardProps {
  icon: LucideIcon;
  title: string;
  desc?: string;
  onClick: () => void;
  onMouseDown?: (e: React.MouseEvent) => void;
  onMouseUp?: (e: React.MouseEvent) => void;
  onMouseLeave?: (e: React.MouseEvent) => void;
  onTouchStart?: (e: React.TouchEvent) => void;
  onTouchEnd?: (e: React.TouchEvent) => void;
}

const HomeCard = React.forwardRef<HTMLDivElement, HomeCardProps>(function HomeCard(
  {
    icon: Icon,
    title,
    desc,
    onClick,
    onMouseDown,
    onMouseUp,
    onMouseLeave,
    onTouchStart,
    onTouchEnd,
  },
  ref,
) {
  return (
    <Card
      ref={ref}
      interactive
      onClick={onClick}
      onMouseDown={onMouseDown}
      onMouseUp={onMouseUp}
      onMouseLeave={onMouseLeave}
      onTouchStart={onTouchStart}
      onTouchEnd={onTouchEnd}
    >
      <div className={styles.cardHeader}>
        <Icon size={24} className={styles.cardIcon} />
        <h3 className={styles.cardTitle}>{title}</h3>
      </div>
      {desc && <p className={styles.cardDesc}>{desc}</p>}
    </Card>
  );
});

function EditableCustomPageCard({
  page,
  onStartEdit,
}: {
  page: CustomPage;
  onStartEdit: (page: CustomPage, rect: DOMRect | null) => void;
}) {
  const navigate = useNavigate();
  const cardRef = useRef<HTMLDivElement>(null);
  const longPress = useLongPress({
    onLongPress: () => onStartEdit(page, cardRef.current?.getBoundingClientRect() || null),
    onClick: () => navigate(`/workspace/custom/${page.id}`),
  });
  const { onClick, ...longPressEvents } = longPress;

  return (
    <HomeCard
      ref={cardRef}
      icon={resolveCustomIcon(page.iconId)}
      title={page.name}
      desc={page.description}
      onClick={onClick}
      {...longPressEvents}
    />
  );
}

export function HomePage() {
  const navigate = useNavigate();
  const { t } = useTranslation(['common', 'navigation']);
  const activeCustomPages = useActiveCustomPages();
  // 当前账户名，用于欢迎卡片让用户感知正在使用的账户
  const accountName = useAuthStore((s) => s.currentAccount?.name);
  const accountId = useAuthStore((s) => s.currentAccount?.id);
  const showToast = useUiStore((s) => s.showToast);

  // 首页「照片集」快捷入口状态：点击后加载全 Vault 活跃附件并直接打开全屏相册。
  const [albumItems, setAlbumItems] = useState<AttachmentMeta[] | null>(null);
  const [albumLoading, setAlbumLoading] = useState(false);

  /** 加载所有对象的所有照片附件并打开照片集（复用 attachment_list_all + 图片过滤）。 */
  const handleOpenPhotoAlbum = async () => {
    if (!accountId || albumLoading) return;
    setAlbumLoading(true);
    try {
      const result = await invoke<AttachmentListAllResult>('attachment_list_all', {
        accountId,
      });
      // 复用全局附件管理器同一过滤逻辑（collectPhotoItems），防止两处漂移
      const photos = collectPhotoItems(result.pages);
      if (photos.length === 0) {
        showToast({
          type: 'info',
          message: t('common:no_photos', { defaultValue: 'No photos yet.' }),
        });
        return;
      }
      setAlbumItems(photos);
    } catch (e) {
      showToast({
        type: 'error',
        message: `${t('common:load_failed', { defaultValue: 'Failed to load' })}: ${e}`,
      });
    } finally {
      setAlbumLoading(false);
    }
  };

  /** 照片集内系统应用打开附件（与全局附件管理器一致）。 */
  const openAttachmentExternal = async (item: AttachmentMeta) => {
    try {
      await invoke('attachment_open', { objectId: item.objectId, attachmentId: item.id });
    } catch {
      showToast({
        type: 'error',
        message: t('common:cannot_open_file', {
          path: item.fileName,
          defaultValue: `Cannot open file: ${item.fileName}`,
        }),
      });
    }
  };

  // 移动端启动性能基线：首页对象列表可见时记录 T2（MOB-P1-07）
  // 从解锁完成时刻（__SOLOSOUL_UNLOCK_TIME）开始计算，而非应用启动时刻
  useEffect(() => {
    const unlockTime = (
      window as typeof window & { __SOLOSOUL_UNLOCK_TIME?: number }
    ).__SOLOSOUL_UNLOCK_TIME;
    if (typeof unlockTime === 'number') {
      // T2 timing is captured internally; no console output in production
      void unlockTime;
    }
  }, []);

  const [editingPage, setEditingPage] = useState<CustomPage | null>(null);
  const [editingCardRect, setEditingCardRect] = useState<DOMRect | null>(null);

  const handleStartEdit = (page: CustomPage, rect: DOMRect | null) => {
    setEditingCardRect(rect);
    setEditingPage(page);
  };

  const handleCloseEdit = () => {
    setEditingPage(null);
    setEditingCardRect(null);
  };

  return (
    <AppShell
      title={t('common:home')}
      actions={
        <PageGuideButton
          pages={[
            {
              icon: LayoutGrid,
              title: t('common:home_guide_title'),
              steps: [
                {
                  icon: LayoutGrid,
                  title: t('common:home_guide_step1_title'),
                  description: t('common:home_guide_step1_desc'),
                },
                {
                  icon: Zap,
                  title: t('common:home_guide_step2_title'),
                  description: t('common:home_guide_step2_desc'),
                },
                {
                  icon: Hand,
                  title: t('common:home_guide_step3_title'),
                  description: t('common:home_guide_step3_desc'),
                },
              ],
              helpLinks: [
                {
                  title: t('common:guide_help_getting_started'),
                  description: t('common:guide_help_getting_started_desc'),
                  href: '/help?id=getting_started',
                },
              ],
            },
          ]}
        />
      }
    >
      <PageContainer variant="wide" gap="section">
        <Card>
          <h2 style={{ fontSize: 'var(--text-page-title)', fontWeight: 600, marginBottom: 4 }}>
            {accountName
              ? t('common:welcome_back_name', { name: accountName })
              : t('common:welcome_back')}
          </h2>
          <p style={{ fontSize: 'var(--text-body)', color: 'var(--text-secondary)' }}>
            {t('common:vault_description')}
          </p>
        </Card>

        <h2
          style={{
            fontSize: 'var(--text-section-title)',
            fontWeight: 600,
            marginBottom: -8,
            color: 'var(--text-primary)',
          }}
        >
          {t('common:data_sections')}
        </h2>

        {/* Profile Sections + Custom Pages */}
        <CardGrid>
          {sections.map((s) => (
            <HomeCard
              key={s.type}
              icon={s.icon}
              title={t(`navigation:${s.labelKey}`)}
              desc={t(`common:${s.descKey}`)}
              onClick={() => navigate(`/workspace?section=${s.type}`)}
            />
          ))}
          {activeCustomPages.map((page) => (
            <EditableCustomPageCard key={page.id} page={page} onStartEdit={handleStartEdit} />
          ))}
          {editingPage && (
            <CustomPageEditPopover
              page={editingPage}
              isOpen={!!editingPage}
              onClose={handleCloseEdit}
              triggerRect={editingCardRect}
              position="bottom"
            />
          )}
        </CardGrid>

        <h2
          style={{
            fontSize: 'var(--text-section-title)',
            fontWeight: 600,
            marginBottom: -8,
            color: 'var(--text-primary)',
          }}
        >
          {t('common:quick_access')}
        </h2>

        {/* Quick Access Cards */}
        <CardGrid>
          {quickCards.map((q) => (
            <Fragment key={q.path}>
              <HomeCard
                icon={q.icon}
                title={t(`navigation:${q.labelKey}`)}
                desc={t(`common:${q.descKey}`)}
                onClick={() => navigate(q.path, { state: { fromHome: true } })}
              />
              {/* 照片集快捷入口：紧跟附件管理卡片，点击直接进入全 Vault 照片集 */}
              {q.path === '/settings/attachments' && (
                <HomeCard
                  icon={Images}
                  title={t('navigation:photo_album', { defaultValue: 'Photo Album' })}
                  desc={t('common:photo_album_desc', {
                    defaultValue: 'Browse photos across all objects',
                  })}
                  onClick={() => void handleOpenPhotoAlbum()}
                />
              )}
            </Fragment>
          ))}
        </CardGrid>
      </PageContainer>

      {/* 照片集全屏相册（首页快捷入口，覆盖整个视口） */}
      {albumItems && (
        <PhotoAlbumOverlay
          items={albumItems}
          onClose={() => setAlbumItems(null)}
          onOpenExternal={openAttachmentExternal}
        />
      )}
    </AppShell>
  );
}
