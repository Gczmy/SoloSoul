import { Puzzle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { usePluginStore } from '@/stores/pluginStore';

interface PluginBadgeProps {
  contractTypeId?: string;
  size?: 'sm' | 'md';
  /**
   * 'full' (默认): 图标 + 文本标签
   * 'icon': 仅显示图标（紧凑场景，如卡片内的字段级别）
   */
  variant?: 'full' | 'icon';
}

const PLUGIN_NAME_MAP: Record<string, string> = {
  'com.solosoul.official.address-fmt/v1': '地址格式化器',
  'com.solosoul.expiry/guardian/v1': '到期卫士',
};

const PLUGIN_NAME_MAP_EN: Record<string, string> = {
  'com.solosoul.official.address-fmt/v1': 'Address Formatter',
  'com.solosoul.expiry/guardian/v1': 'Expiry Guardian',
};

/** 检查某个 contractTypeId 是否有已安装的插件提供 */
function isContractTypeInstalled(
  contractTypeId: string,
  installedPlugins: ReturnType<typeof usePluginStore.getState>['installedPlugins'],
): boolean {
  return installedPlugins.some((p) => p.contracts?.some((c) => c.typeId === contractTypeId));
}

export function PluginBadge({ contractTypeId, size = 'sm', variant = 'full' }: PluginBadgeProps) {
  const { t, i18n } = useTranslation(['settings']);
  const installedPlugins = usePluginStore((s) => s.installedPlugins);

  if (!contractTypeId) return null;

  const isInstalled = isContractTypeInstalled(contractTypeId, installedPlugins);

  const pluginName = i18n.language.startsWith('zh')
    ? PLUGIN_NAME_MAP[contractTypeId] || contractTypeId
    : PLUGIN_NAME_MAP_EN[contractTypeId] || contractTypeId;

  const isSmall = size === 'sm';

  // 未安装时使用灰色调
  const bgColor = isInstalled
    ? 'var(--accent-primary-soft, rgba(99,102,241,0.12))'
    : 'color-mix(in srgb, var(--text-tertiary) 15%, transparent)';
  const fgColor = isInstalled ? 'var(--accent-primary, #6366f1)' : 'var(--text-tertiary)';

  if (variant === 'icon') {
    return (
      <span
        title={t('settings:plugin_badge_tooltip', { pluginName })}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          color: fgColor,
          flexShrink: 0,
        }}
      >
        <Puzzle size={isSmall ? 10 : 12} />
      </span>
    );
  }

  return (
    <span
      title={t('settings:plugin_badge_tooltip', { pluginName })}
      style={{
        display: 'inline-flex',
        alignItems: 'center',
        gap: isSmall ? 2 : 4,
        padding: isSmall ? '1px 5px' : '2px 8px',
        borderRadius: 4,
        background: bgColor,
        color: fgColor,
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
