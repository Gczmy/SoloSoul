import { memo } from 'react';
import { useTranslation } from 'react-i18next';
import { useSettingsStore } from '@/stores/settingsStore';

const SYSTEM_PAGES = ['identity', 'travel', 'financial', 'professional'] as const;

interface TemplatePageSelectProps {
  value: string;
  onChange: (v: string) => void;
  label?: string;
}

export const TemplatePageSelect = memo(function TemplatePageSelect({
  value,
  onChange,
  label,
}: TemplatePageSelectProps) {
  const { t } = useTranslation(['settings', 'navigation']);
  const customPages = useSettingsStore((s) => s.settings.customPages) || [];

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 4 }}>
      {label && <label style={{ fontSize: 12, color: 'var(--text-secondary)' }}>{label}</label>}
      <select
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onMouseEnter={(e) => {
          e.currentTarget.style.borderColor = 'var(--accent-primary)';
          e.currentTarget.style.boxShadow = '0 0 0 2px color-mix(in srgb, var(--accent-primary) 10%, transparent)';
        }}
        onMouseLeave={(e) => {
          if (document.activeElement !== e.currentTarget) {
            e.currentTarget.style.borderColor = 'var(--border-subtle)';
            e.currentTarget.style.boxShadow = 'none';
          }
        }}
        onFocus={(e) => {
          e.currentTarget.style.borderColor = 'var(--accent-primary)';
          e.currentTarget.style.boxShadow = '0 0 0 2px color-mix(in srgb, var(--accent-primary) 15%, transparent)';
        }}
        onBlur={(e) => {
          e.currentTarget.style.borderColor = 'var(--border-subtle)';
          e.currentTarget.style.boxShadow = 'none';
        }}
        style={{
          padding: '8px 10px',
          borderRadius: 6,
          border: '1px solid var(--border-subtle)',
          background: 'var(--bg-elevated)',
          color: 'var(--text-primary)',
          fontSize: 13,
          cursor: 'pointer',
          transition: 'border-color 0.2s, box-shadow 0.2s',
        }}
      >
        <optgroup label={t('settings:system_pages') || '系统页面'}>
          {SYSTEM_PAGES.map((cat) => (
            <option key={cat} value={cat}>
              {t(`navigation:${cat}`, cat)}
            </option>
          ))}
        </optgroup>
        {customPages.filter((p) => !p.deletedAt).length > 0 && (
          <optgroup label={t('settings:custom_pages') || '自定义页面'}>
            {customPages
              .filter((p) => !p.deletedAt)
              .map((page) => (
                <option key={page.id} value={page.id}>
                  {page.name}
                </option>
              ))}
          </optgroup>
        )}
        {customPages.filter((p) => p.deletedAt).length > 0 && (
          <optgroup label={t('settings:custom_pages_trash') || '自定义页面（回收站）'}>
            {customPages
              .filter((p) => p.deletedAt)
              .map((page) => (
                <option key={page.id} value={page.id} disabled>
                  {page.name}
                </option>
              ))}
          </optgroup>
        )}
        {value &&
          !SYSTEM_PAGES.includes(value as (typeof SYSTEM_PAGES)[number]) &&
          !customPages.find((p) => p.id === value) && (
            <option value={value} disabled>
              {t('settings:deleted_page') || '（页面已删除）'}
            </option>
          )}
      </select>
    </div>
  );
});
