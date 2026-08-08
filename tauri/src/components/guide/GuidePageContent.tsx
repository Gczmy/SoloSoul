import { ArrowRight } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import type { TFunction } from 'i18next';
import type { GuidePage } from './PageGuide';

/**
 * P013/4: 指南单页内容 — 步骤时间线 + 相关帮助文档卡片。
 */
export function GuidePageContent({
  page,
  t,
  onHelpLinkClick,
}: {
  page: GuidePage;
  t: TFunction;
  onHelpLinkClick: (href: string) => void;
}) {
  return (
    <>
      {/* Steps */}
      <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
        {page.steps.map((step, stepIdx) => {
          const Icon = step.icon;
          const isLastStep = stepIdx === page.steps.length - 1;
          return (
            <div
              key={stepIdx}
              style={{
                display: 'flex',
                gap: 14,
                padding: '12px 0',
                position: 'relative',
              }}
            >
              {/* 左侧：图标 + 连接线 */}
              <div
                style={{
                  display: 'flex',
                  flexDirection: 'column',
                  alignItems: 'center',
                  flexShrink: 0,
                  width: 28,
                }}
              >
                <div
                  style={{
                    width: 28,
                    height: 28,
                    borderRadius: '50%',
                    background:
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
                    border:
                      '1px solid color-mix(in srgb, var(--accent-primary) 25%, transparent)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                    flexShrink: 0,
                    zIndex: 1,
                  }}
                >
                  <Icon size={ICON_SIZE.sm} style={{ color: 'var(--accent-primary)' }} />
                </div>
                {!isLastStep && (
                  <div
                    style={{
                      width: 1,
                      flex: 1,
                      minHeight: 16,
                      background: 'var(--border-subtle)',
                      marginTop: 4,
                    }}
                  />
                )}
              </div>

              {/* 右侧：内容 */}
              <div style={{ flex: 1, minWidth: 0, paddingTop: 2 }}>
                <div
                  style={{
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    marginBottom: 4,
                  }}
                >
                  <span
                    style={{
                      width: 18,
                      height: 18,
                      borderRadius: '50%',
                      background: 'var(--accent-primary)',
                      color: '#fff',
                      fontSize: 10,
                      fontWeight: 700,
                      display: 'inline-flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                      flexShrink: 0,
                    }}
                  >
                    {stepIdx + 1}
                  </span>
                  <span
                    style={{
                      fontSize: 'var(--text-card-title)',
                      fontWeight: 600,
                      color: 'var(--text-primary)',
                    }}
                  >
                    {step.title}
                  </span>
                </div>
                <p
                  style={{
                    margin: 0,
                    fontSize: 'var(--text-body-sm)',
                    color: 'var(--text-secondary)',
                    lineHeight: 1.6,
                    whiteSpace: 'pre-wrap',
                  }}
                >
                  {step.description}
                </p>
              </div>
            </div>
          );
        })}
      </div>

      {/* 相关帮助文档卡片 */}
      {page.helpLinks.length > 0 && (
        <div
          style={{
            margin: '8px 0',
            padding: '12px',
            borderRadius: 10,
            background: 'var(--bg-toolbar)',
          }}
        >
          <div
            style={{
              fontSize: 'var(--text-badge)',
              fontWeight: 600,
              color: 'var(--text-tertiary)',
              marginBottom: 8,
              textTransform: 'uppercase',
              letterSpacing: '0.5px',
            }}
          >
            {t('related_docs', { defaultValue: '相关帮助文档' })}
          </div>
          <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
            {page.helpLinks.map((link, i) => (
              <button
                key={i}
                onClick={() => onHelpLinkClick(link.href)}
                className="interactive-row"
                style={{
                  display: 'flex',
                  alignItems: 'center',
                  gap: 10,
                  padding: '8px 10px',
                  borderRadius: 8,
                  borderWidth: 1,
                  borderStyle: 'solid',
                  cursor: 'pointer',
                  textAlign: 'left',
                  width: '100%',
                  fontFamily: 'inherit',
                }}
              >
                <div style={{ flex: 1, minWidth: 0 }}>
                  <div
                    style={{
                      fontSize: 'var(--text-body-sm)',
                      fontWeight: 600,
                      color: 'var(--text-primary)',
                      marginBottom: 2,
                    }}
                  >
                    {link.title}
                  </div>
                  <div
                    style={{
                      fontSize: 'var(--text-badge)',
                      color: 'var(--text-tertiary)',
                      overflow: 'hidden',
                      textOverflow: 'ellipsis',
                      whiteSpace: 'nowrap',
                    }}
                  >
                    {link.description}
                  </div>
                </div>
                <ArrowRight
                  size={ICON_SIZE.sm}
                  style={{
                    color: 'var(--accent-primary)',
                    flexShrink: 0,
                  }}
                />
              </button>
            ))}
          </div>
        </div>
      )}
    </>
  );
}
