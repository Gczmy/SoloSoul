import { useEffect, useMemo, useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { X, Search } from 'lucide-react';
import { motion } from 'framer-motion';
import {
  SAMPLE_TEMPLATES_BY_LOCALE,
  getDefaultLocaleTab,
  type SampleTemplate,
  type SampleTemplateLocale,
} from '@/lib/sampleTemplates';
import { resolveCustomIcon } from '@/lib/pageIcons';
import { SensitivityBadge } from '@/components/ui/SensitivityBadge';
import { PluginBadge } from './PluginBadge';
import { Input } from '@/components/ui/Input';
import type { SensitivityLevel } from '@/types/template';
import { ICON_SIZE } from '@/lib/constants';

const SENSITIVITY_ORDER: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];
const SAMPLE_PAGES = ['identity', 'travel', 'financial', 'professional'] as const;

interface SampleTemplateGalleryProps {
  isOpen: boolean;
  onClose: () => void;
  onSelect: (tpl: SampleTemplate) => void;
}

export function SampleTemplateGallery({ isOpen, onClose, onSelect }: SampleTemplateGalleryProps) {
  const { t, i18n } = useTranslation(['settings', 'navigation', 'common']);
  const [localeTab, setLocaleTab] = useState<SampleTemplateLocale>(() =>
    getDefaultLocaleTab(i18n.language),
  );
  const [pageFilter, setPageFilter] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');

  useEffect(() => {
    setLocaleTab(getDefaultLocaleTab(i18n.language));
  }, [i18n.language]);

  const currentSamples = SAMPLE_TEMPLATES_BY_LOCALE[localeTab];

  // 用于卡片可见判断与“点击护送”联动，避免重复计算 q。
  const normalizedQuery = useMemo(() => searchQuery.trim().toLowerCase(), [searchQuery]);

  const pageOptions = useMemo(
    () => [
      { id: 'all', label: t('settings:filter_all') },
      ...SAMPLE_PAGES.map((id) => ({ id, label: t(`navigation:${id}`) })),
    ],
    [t],
  );

  // 单一可见性谓词：同时供 `filteredSamples` 与网格渲染逻辑复用，避免重复实现。
  // 为了让弹卡在切换选项时大小保持稳定（含全部示例时的高度），
  // 渲染侧始终遍历 `currentSamples`，对不匹配的卡片使用 `visibility: hidden` +
  // `pointer-events: none` 占位，DOM 上保留全部卡，但可见数量随过滤收敛，
  // 上面筛选按钮位置不会跳动。
  const isSampleMatch = useCallback(
    (tpl: SampleTemplate) => {
      if (pageFilter !== 'all' && tpl.category !== pageFilter) return false;
      if (normalizedQuery && !tpl.name.toLowerCase().includes(normalizedQuery)) return false;
      return true;
    },
    [pageFilter, normalizedQuery],
  );

  const filteredSamples = useMemo(
    () => currentSamples.filter(isSampleMatch),
    [currentSamples, isSampleMatch],
  );

  const switchLocale = (locale: SampleTemplateLocale) => {
    setLocaleTab(locale);
    setPageFilter('all');
    setSearchQuery('');
  };

  if (!isOpen) return null;

  return (
    <div
      style={{
        position: 'fixed',
        inset: 0,
        zIndex: 'var(--z-modal-important)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(0,0,0,0.35)',
        backdropFilter: 'blur(4px)',
      }}
      onClick={onClose}
    >
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.2 }}
        onClick={(e) => e.stopPropagation()}
        style={{
          background: 'var(--bg-elevated)',
          borderRadius: 16,
          padding: '24px 28px',
          maxWidth: 720,
          width: '90%',
          maxHeight: '85vh',
          overflowY: 'auto',
          boxShadow: 'var(--shadow-lg)',
          border: '1px solid var(--border-subtle)',
        }}
      >
        <div
          style={{
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'space-between',
            marginBottom: 8,
          }}
        >
          <h2 style={{ fontSize: 'var(--text-md)', fontWeight: 700, margin: 0 }}>
            {t('settings:sample_templates_title')}
          </h2>
          <button
            onClick={onClose}
            data-testid="sample-gallery-close"
            aria-label={t('common:close')}
            style={{
              padding: 6,
              borderRadius: 8,
              border: 'none',
              background: 'transparent',
              cursor: 'pointer',
              color: 'var(--text-tertiary)',
            }}
          >
            <X size={ICON_SIZE.xl} />
          </button>
        </div>
        <p
          style={{
            fontSize: 'var(--text-caption)',
            color: 'var(--text-secondary)',
            margin: '0 0 16px',
          }}
        >
          {t('settings:sample_templates_desc')}
        </p>

        <div
          style={{
            display: 'flex',
            gap: 8,
            marginBottom: 16,
          }}
        >
          <button
            type="button"
            data-testid="locale-tab-zh"
            aria-pressed={localeTab === 'zh'}
            onClick={() => switchLocale('zh')}
            onMouseEnter={
              localeTab !== 'zh'
                ? (e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  }
                : undefined
            }
            onMouseLeave={
              localeTab !== 'zh'
                ? (e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }
                : undefined
            }
            style={{
              flex: 1,
              padding: '8px 12px',
              borderRadius: 8,
              border:
                localeTab === 'zh'
                  ? '1px solid var(--accent-primary)'
                  : '1px solid var(--border-subtle)',
              background:
                localeTab === 'zh'
                  ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                  : 'var(--bg-toolbar)',
              color: localeTab === 'zh' ? 'var(--accent-primary)' : 'var(--text-primary)',
              boxShadow: localeTab === 'zh' ? '0 0 0 1px var(--accent-primary)' : 'none',
              fontSize: 'var(--text-body-sm)',
              fontWeight: 500,
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
            }}
          >
            {t('settings:locale_zh')}
          </button>
          <button
            type="button"
            data-testid="locale-tab-en"
            aria-pressed={localeTab === 'en'}
            onClick={() => switchLocale('en')}
            onMouseEnter={
              localeTab !== 'en'
                ? (e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  }
                : undefined
            }
            onMouseLeave={
              localeTab !== 'en'
                ? (e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }
                : undefined
            }
            style={{
              flex: 1,
              padding: '8px 12px',
              borderRadius: 8,
              border:
                localeTab === 'en'
                  ? '1px solid var(--accent-primary)'
                  : '1px solid var(--border-subtle)',
              background:
                localeTab === 'en'
                  ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                  : 'var(--bg-toolbar)',
              color: localeTab === 'en' ? 'var(--accent-primary)' : 'var(--text-primary)',
              boxShadow: localeTab === 'en' ? '0 0 0 1px var(--accent-primary)' : 'none',
              fontSize: 'var(--text-body-sm)',
              fontWeight: 500,
              cursor: 'pointer',
              transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
            }}
          >
            {t('settings:locale_en')}
          </button>
        </div>

        <div style={{ display: 'flex', flexDirection: 'column', gap: 10, marginBottom: 16 }}>
          <Input
            placeholder={t('settings:search_sample_templates') || '搜索示例模板...'}
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            onClear={() => setSearchQuery('')}
            prefixIcon={<Search size={ICON_SIZE.sm} style={{ color: 'var(--text-tertiary)' }} />}
          />
          <div style={{ display: 'flex', gap: 6, flexWrap: 'wrap' }}>
            {pageOptions.map((opt) => {
              const isActive = pageFilter === opt.id;
              return (
                <button
                  key={opt.id}
                  type="button"
                  data-testid={`page-filter-${opt.id}`}
                  onClick={() => setPageFilter(opt.id)}
                  aria-pressed={isActive}
                  onMouseEnter={
                    !isActive
                      ? (e) => {
                          e.currentTarget.style.background =
                            'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                          e.currentTarget.style.borderColor = 'var(--accent-primary)';
                        }
                      : undefined
                  }
                  onMouseLeave={
                    !isActive
                      ? (e) => {
                          e.currentTarget.style.background = 'var(--bg-toolbar)';
                          e.currentTarget.style.borderColor = 'var(--border-subtle)';
                        }
                      : undefined
                  }
                  style={{
                    padding: '5px 12px',
                    borderRadius: 6,
                    border: isActive
                      ? '1px solid var(--accent-primary)'
                      : '1px solid var(--border-subtle)',
                    background: isActive
                      ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)'
                      : 'var(--bg-toolbar)',
                    color: isActive ? 'var(--accent-primary)' : 'var(--text-primary)',
                    boxShadow: isActive ? '0 0 0 1px var(--accent-primary)' : 'none',
                    fontSize: 'var(--text-caption)',
                    cursor: 'pointer',
                    transition: 'background 0.2s, border-color 0.2s, color 0.2s, box-shadow 0.2s',
                  }}
                >
                  {opt.label}
                </button>
              );
            })}
          </div>
        </div>

        <div style={{ position: 'relative' }}>
          <div
            style={{
              display: 'grid',
              gridTemplateColumns: 'repeat(auto-fill, minmax(220px, 1fr))',
              gap: 12,
            }}
            data-testid="sample-template-grid"
          >
            {currentSamples.map((tpl) => {
              const visible = isSampleMatch(tpl);
              const present = new Set(tpl.properties.map((p) => p.sensitivityLevel));
              const ordered = SENSITIVITY_ORDER.filter((l) => present.has(l));
              const SampleIcon = resolveCustomIcon(tpl.icon);
              return (
                <button
                  key={tpl.key}
                  data-testid="sample-template-card"
                  data-visible={visible ? 'true' : 'false'}
                  aria-hidden={visible ? 'false' : 'true'}
                  tabIndex={visible ? 0 : -1}
                  onClick={() => visible && onSelect(tpl)}
                  style={{
                    display: 'flex',
                    flexDirection: 'column',
                    gap: 10,
                    padding: 16,
                    borderRadius: 12,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    cursor: visible ? 'pointer' : 'default',
                    textAlign: 'left',
                    transition: 'border-color 0.15s, transform 0.1s',
                    // 关键：
                    // 1) visibility:hidden 让不可见卡仍占据网格位置，保持弹卡高度不变。
                    // 2) order:-1 让可见卡排在网格第一项，避免空出 1、2 位。
                    //    CSS Grid 的 auto-placement 顺序：先排 order:-1 项，再排 order:0 项，
                    //    不可见项会被推到可见项之后的位置。
                    visibility: visible ? 'visible' : 'hidden',
                    pointerEvents: visible ? 'auto' : 'none',
                    order: visible ? -1 : 0,
                  }}
                  onMouseEnter={(e) => {
                    if (!visible) return;
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  }}
                >
                  <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                    <SampleIcon
                      size={ICON_SIZE['2xl']}
                      style={{ color: 'var(--text-primary)' }}
                    />
                    <div>
                      <div
                        style={{
                          fontSize: 'var(--text-body)',
                          fontWeight: 600,
                          color: 'var(--text-primary)',
                          display: 'flex',
                          alignItems: 'center',
                          gap: 6,
                        }}
                      >
                        {tpl.name}
                        <PluginBadge contractTypeId={tpl.contractTypeId} size="sm" />
                      </div>
                      <div
                        style={{
                          fontSize: 'var(--text-badge)',
                          color: 'var(--text-tertiary)',
                          marginTop: 2,
                        }}
                      >
                        {t(`navigation:${tpl.category}`, tpl.category)} · {tpl.properties.length}{' '}
                        {t('settings:template_fields')}
                      </div>
                    </div>
                  </div>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 4, flexWrap: 'wrap' }}>
                    {ordered.map((level) => (
                      <SensitivityBadge key={level} level={level} />
                    ))}
                  </div>
                </button>
              );
            })}
          </div>
          {/* 过滤后零命中时用空状态覆盖提示，不改变网格高度。 */}
          {filteredSamples.length === 0 && (
            <div
              role="status"
              style={{
                position: 'absolute',
                inset: 0,
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                color: 'var(--text-tertiary)',
                fontSize: 'var(--text-body-sm)',
                pointerEvents: 'none',
              }}
            >
              {t('common:no_results')}
            </div>
          )}
        </div>
      </motion.div>
    </div>
  );
}
