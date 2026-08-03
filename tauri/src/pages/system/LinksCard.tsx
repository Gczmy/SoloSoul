import { useTranslation } from 'react-i18next';
import { open } from '@tauri-apps/plugin-shell';
import { ExternalLink } from 'lucide-react';
import { Card } from '@/components/ui/Card';
import { ICON_SIZE } from '@/lib/constants';
import type { ReactNode } from 'react';

interface AboutLink {
  labelKey: string;
  url: string;
  icon: ReactNode;
}

interface LinksCardProps {
  links: AboutLink[];
  showToast: (toast: { type: 'error'; message: string }) => void;
}

/**
 * 外部链接卡片（P224-④ 拆分）。
 * 链接数组由 AboutPage 构建后透传。
 */
export function LinksCard({ links, showToast }: LinksCardProps) {
  const { t } = useTranslation(['settings', 'common']);
  return (
    <Card>
      <div style={{ display: 'flex', flexDirection: 'column', gap: 2 }}>
        {links.map((link, i) => (
          <div key={link.url}>
            {i > 0 && (
              <div style={{ height: 1, background: 'var(--border-subtle)', margin: '0 4px' }} />
            )}
            <a
              href={link.url}
              target="_blank"
              rel="noopener noreferrer"
              className="interactive-link-row"
              style={{
                display: 'flex',
                alignItems: 'center',
                gap: 10,
                padding: '12px 4px',
                borderRadius: 8,
                color: 'var(--text-primary)',
                fontSize: 'var(--text-sm)',
                textDecoration: 'none',
              }}
              onClick={(e) => {
                e.preventDefault();
                // P231: Tauri webview 中 window.open 无效，shell 打开失败时
                // 以应用内 toast 反馈，不再使用无效的 window.open 兜底。
                open(link.url).catch((err) => {
                  showToast({
                    type: 'error',
                    message:
                      t('settings:link_open_failed', {
                        defaultValue: '无法打开链接',
                      }) + (err ? `: ${err}` : ''),
                  });
                });
              }}
            >
              <span style={{ color: 'var(--text-tertiary)', display: 'flex' }}>{link.icon}</span>
              <span style={{ flex: 1 }}>{t('settings:' + link.labelKey)}</span>
              <ExternalLink
                size={ICON_SIZE.xs}
                style={{ color: 'var(--text-tertiary)', opacity: 0.5 }}
              />
            </a>
          </div>
        ))}
      </div>
    </Card>
  );
}
