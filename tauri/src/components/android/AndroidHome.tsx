import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { ChevronRight, Images, MoreHorizontal, Layers } from 'lucide-react';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';
import { SYSTEM_PAGE_KEYS, useActiveCustomPages } from '@/components/layout/useNavigationItems';
import type { CustomPage } from '@/stores/settingsStore';
import { useAuthStore } from '@/stores/authStore';
import { usePrefetchData } from '@/lib/prefetch/usePrefetchData';
import { prefetchRegistry } from '@/lib/prefetch/registry';

export function AndroidHome({
  onEditPage,
  onPhotos,
}: {
  onEditPage: (page: CustomPage, rect: DOMRect) => void;
  onPhotos: () => void;
}) {
  const { t, i18n } = useTranslation(['common', 'navigation']);
  const navigate = useNavigate();
  const accountName = useAuthStore((s) => s.currentAccount?.name);
  const pages = useActiveCustomPages();
  const { data, error, reload } = usePrefetchData(prefetchRegistry.androidOverview);
  const categories = [
    ...SYSTEM_PAGE_KEYS.map((type) => ({
      id: type,
      name: t(`navigation:${type}`),
      Icon: PAGE_ICON_MAP[type],
      path: `/workspace?section=${type}`,
      page: undefined as CustomPage | undefined,
    })),
    ...pages.map((page) => ({
      id: page.id,
      name: page.name,
      Icon: resolveCustomIcon(page.iconId),
      path: `/workspace/custom/${page.id}`,
      page,
    })),
  ];
  return (
    <div className="android-page" data-testid="android-home">
      <section className="android-overview">
        <h2>{t('welcome_back_name', { name: accountName })}</h2>
        <p>{t('material.home_subtitle')}</p>
        <div className="android-overview-stat">
          <strong>{data?.count ?? '—'}</strong>
          <span>{t('material.object_count_label')}</span>
        </div>
        <p>{t('material.local_vault')}</p>
      </section>
      <div className="android-home-columns">
        <section>
          <div className="android-section-heading">
            <h2>{t('data_sections')}</h2>
            <button
              type="button"
              className="android-text-button"
              onClick={() => navigate('/workspace')}
            >
              {t('material.view_all')}
              <ChevronRight size={18} />
            </button>
          </div>
          <div className="android-category-grid">
            {categories.map(({ id, name, Icon, path, page }) => (
              <div className="android-category" key={id}>
                <button type="button" onClick={() => navigate(path)}>
                  <Icon size={24} />
                  <strong>{name}</strong>
                  <small>{t('material.category_count', { count: data?.counts[id] ?? 0 })}</small>
                </button>
                {page && (
                  <button
                    type="button"
                    className="android-icon-button android-category-edit"
                    aria-label={t('material.edit_page', { name })}
                    onClick={(event) =>
                      onEditPage(page, event.currentTarget.getBoundingClientRect())
                    }
                  >
                    <MoreHorizontal size={20} />
                  </button>
                )}
              </div>
            ))}
          </div>
        </section>
        <section>
          <div className="android-section-heading">
            <h2>{t('material.recent')}</h2>
            <button
              type="button"
              className="android-icon-button"
              aria-label={t('navigation:photo_album')}
              onClick={onPhotos}
            >
              <Images size={24} />
            </button>
          </div>
          <div className="android-group">
            {data?.recent.map((obj) => (
              <button
                type="button"
                className="android-row-button"
                key={obj.id}
                onClick={() => navigate(`/workspace?objectId=${encodeURIComponent(obj.id)}`)}
              >
                <span className="android-row-icon">
                  <Layers size={22} />
                </span>
                <span className="android-row-copy">
                  <strong>{obj.name}</strong>
                  <small>
                    {categories.find((cat) => cat.id === obj.typeId)?.name ?? obj.typeId} ·{' '}
                    {new Date(obj.updatedAt).toLocaleDateString(i18n.language, {
                      month: 'short',
                      day: 'numeric',
                    })}
                  </small>
                </span>
                <ChevronRight size={18} />
              </button>
            ))}
            {data?.recent.length === 0 && <p className="android-row-button">{t('no_objects')}</p>}
            {error && (
              <button type="button" className="android-text-button" onClick={() => void reload()}>
                {t('material.retry')}
              </button>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
