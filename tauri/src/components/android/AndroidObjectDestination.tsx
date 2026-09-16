import { ChevronRight } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { SYSTEM_PAGE_KEYS, useActiveCustomPages } from '@/components/layout/useNavigationItems';
import { PAGE_ICON_MAP, resolveCustomIcon } from '@/lib/pageIcons';

/** 全局新建先选择归属；用 replace 保留返回来源，避免保存后回到选择页。 */
export function AndroidObjectDestination() {
  const { t } = useTranslation(['editor', 'navigation']);
  const navigate = useNavigate();
  const customPages = useActiveCustomPages();
  const destinations = [
    ...SYSTEM_PAGE_KEYS.map((id) => ({
      id,
      name: t(`navigation:${id}`),
      Icon: PAGE_ICON_MAP[id],
      query: `section=${encodeURIComponent(id)}`,
    })),
    ...customPages.map((page) => ({
      id: page.id,
      name: page.name,
      Icon: resolveCustomIcon(page.iconId),
      query: `parentId=${encodeURIComponent(page.id)}`,
    })),
  ];
  return (
    <div className="android-page" data-testid="object-destination-picker">
      <div className="android-home-intro">
        <h2>{t('choose_destination')}</h2>
        <p>{t('choose_destination_description')}</p>
      </div>
      <div className="android-group">
        {destinations.map(({ id, name, Icon, query }) => (
          <button
            key={id}
            type="button"
            className="android-row-button"
            onClick={() => navigate(`/editor?${query}`, { replace: true })}
          >
            <span className="android-row-icon">
              <Icon size={24} />
            </span>
            <span className="android-row-copy">
              <strong>{name}</strong>
            </span>
            <ChevronRight size={20} aria-hidden="true" />
          </button>
        ))}
      </div>
    </div>
  );
}
