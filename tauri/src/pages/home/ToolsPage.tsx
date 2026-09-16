import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { PageShell } from '@/components/layout/PageShell';
import { useMobileNavActions } from '@/components/layout/useNavigationItems';
import { PAGE_ICON_MAP } from '@/lib/pageIcons';

/** 直接复用移动导航入口；扫码/插件/AI 始终进入现有完整页面。 */
export function ToolsPage() {
  const { t } = useTranslation(['common', 'navigation']);
  const navigate = useNavigate();
  const { items } = useMobileNavActions();
  return (
    <PageShell title={t('material.tools')} onBack={() => navigate('/')}>
      <div className="android-page">
        <section className="android-overview">
          <h2>{t('material.tools_title')}</h2>
          <p>{t('material.tools_description')}</p>
        </section>
        <div className="android-tool-grid">
          {items.map((item) => {
            const Icon = PAGE_ICON_MAP[item.iconKey];
            return (
              <button
                type="button"
                key={item.iconKey}
                className="android-tool"
                onClick={() =>
                  item.type === 'link'
                    ? navigate(item.path, { state: { from: '/tools' } })
                    : item.action()
                }
              >
                <Icon size={28} />
                <strong>{t(`navigation:${item.labelKey}`)}</strong>
              </button>
            );
          })}
        </div>
      </div>
    </PageShell>
  );
}
