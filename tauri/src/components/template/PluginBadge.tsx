import { Puzzle } from 'lucide-react';
import { useTranslation } from 'react-i18next';

interface PluginBadgeProps {
  contractTypeId?: string;
  size?: 'sm' | 'md';
}

const PLUGIN_NAME_MAP: Record<string, string> = {
  'com.solosoul.official.address-fmt/v1': '地址格式化器',
};

const PLUGIN_NAME_MAP_EN: Record<string, string> = {
  'com.solosoul.official.address-fmt/v1': 'Address Formatter',
};

export function PluginBadge({ contractTypeId, size = 'sm' }: PluginBadgeProps) {
  const { t, i18n } = useTranslation(['settings']);
  if (!contractTypeId) return null;

  const pluginName = i18n.language.startsWith('zh')
    ? PLUGIN_NAME_MAP[contractTypeId] || contractTypeId
    : PLUGIN_NAME_MAP_EN[contractTypeId] || contractTypeId;

  const isSmall = size === 'sm';

  return (
    <span
      title={t('settings:plugin_badge_tooltip', { pluginName })}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: isSmall ? 2 : 4,
        padding: isSmall ? '1px 5px' : '2px 8px',
        borderRadius: 4,
        background: 'var(--accent-primary-soft, rgba(99,102,241,0.12))',
        color: 'var(--accent-primary, #6366f1)',
        fontSize: isSmall ? 10 : 12,
        fontWeight: 500,
        lineHeight: 1.4,
        whiteSpace: 'nowrap',
      }}
    >
      <Puzzle size={isSmall ? 10 : 12} />
      {t('settings:plugin_badge_label')}
    </span>
  );
}
