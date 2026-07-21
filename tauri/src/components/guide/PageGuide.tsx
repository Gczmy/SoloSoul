import { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
import { AnimatePresence, motion } from 'framer-motion';
import { CircleHelp, X, ChevronLeft, ChevronRight, ArrowRight } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import type { LucideIcon } from 'lucide-react';

export interface GuideStep {
  icon: LucideIcon;
  title: string;
  description: string;
}

export interface GuideHelpLink {
  title: string;
  description: string;
  href: string;
}

export interface GuidePage {
  icon: LucideIcon;
  title: string;
  steps: GuideStep[];
  helpLinks: GuideHelpLink[];
}

interface PageGuideProps {
  pages: GuidePage[];
  label?: string;
}

/**
 * PageGuide — 多页面分步指南组件。
 *
 * 渲染一个「圆圈问号图标 + 文本」触发器按钮（无背景色、外边框 outline、悬停强调色），
 * 点击后弹出多页指南卡片，每页包含：步骤列表 + 相关帮助文档跳转卡片。
 * 支持上/下一页导航、页面指示圆点、移动端左右滑动手势翻页 + 页面切换动画。
 */
export function PageGuide({ pages, label }: PageGuideProps) {
  const { t } = useTranslation('common');
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [pageIndex, setPageIndex] = useState(0);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const touchStartX = useRef(0);
  const touchStartY = useRef(0);
  // Tracks animation direction: 1 = forward (right), -1 = backward (left)
  const direction = useRef(1);

  // 每次打开时重置到第一页
  useEffect(() => {
    if (open) setPageIndex(0);
  }, [open]);

  // 点击外部关闭卡片
  useEffect(() => {
    if (!open) return;
    const handleClickOutside = (e: MouseEvent) => {
      if (
        cardRef.current &&
        !cardRef.current.contains(e.target as Node) &&
        triggerRef.current &&
        !triggerRef.current.contains(e.target as Node)
      ) {
        setOpen(false);
      }
    };
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handleClickOutside);
    }, 0);
    return () => {
      clearTimeout(timer);
      document.removeEventListener('mousedown', handleClickOutside);
    };
  }, [open]);

  // Escape 关闭
  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') setOpen(false);
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [open]);

  const displayLabel = label ?? t('guide') ?? '指南';
  const active = hovered;
  const currentPage = pages[pageIndex];
  const isFirst = pageIndex === 0;
  const isLast = pageIndex === pages.length - 1;

  const goTo = useCallback(
    (index: number) => {
      if (index < 0 || index >= pages.length) return;
      if (index === pageIndex) return;
      direction.current = index > pageIndex ? 1 : -1;
      setPageIndex(index);
    },
    [pageIndex, pages.length],
  );

  const handlePrev = useCallback(() => {
    if (!isFirst) goTo(pageIndex - 1);
  }, [isFirst, goTo, pageIndex]);

  const handleNext = useCallback(() => {
    if (!isLast) goTo(pageIndex + 1);
  }, [isLast, goTo, pageIndex]);

  // Touch swipe handlers
  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    touchStartX.current = e.touches[0].clientX;
    touchStartY.current = e.touches[0].clientY;
  }, []);

  const handleTouchEnd = useCallback(
    (e: React.TouchEvent) => {
      const dx = e.changedTouches[0].clientX - touchStartX.current;
      const dy = e.changedTouches[0].clientY - touchStartY.current;
      const threshold = 50;

      // Only trigger swipe if horizontal distance exceeds threshold
      // and horizontal movement is greater than vertical (prevent scroll interference)
      if (Math.abs(dx) > threshold && Math.abs(dx) > Math.abs(dy)) {
        if (dx > 0) {
          // Swipe right → prev page
          if (!isFirst) goTo(pageIndex - 1);
        } else {
          // Swipe left → next page
          if (!isLast) goTo(pageIndex + 1);
        }
      }
    },
    [isFirst, isLast, goTo, pageIndex],
  );

  const handleHelpLinkClick = (href: string) => {
    setOpen(false);
    navigate(href);
  };

  const handleClose = () => {
    setOpen(false);
  };

  // Animation variants for page slide transition
  const pageVariants = {
    enter: (dir: number) => ({
      x: dir > 0 ? 300 : -300,
      opacity: 0,
    }),
    center: {
      x: 0,
      opacity: 1,
    },
    exit: (dir: number) => ({
      x: dir > 0 ? -300 : 300,
      opacity: 0,
    }),
  };

  return (
    <>
      {/* 触发器按钮 */}
      <button
        ref={triggerRef}
        onClick={() => setOpen((prev) => !prev)}
        onMouseEnter={() => setHovered(true)}
        onMouseLeave={() => setHovered(false)}
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 6,
          padding: '6px 10px',
          borderRadius: 8,
          border: '1px solid var(--border-subtle)',
          borderColor: active ? 'var(--accent-primary)' : 'var(--border-subtle)',
          background: active
            ? 'color-mix(in srgb, var(--accent-primary) 6%, transparent)'
            : 'transparent',
          color: active ? 'var(--accent-primary)' : 'var(--text-tertiary)',
          cursor: 'pointer',
          fontSize: 'var(--text-badge)',
          fontWeight: 500,
          transition: 'all 0.15s ease',
        }}
      >
        <CircleHelp size={ICON_SIZE.sm} />
        <span>{displayLabel}</span>
      </button>

      {/* 指南卡片 — 居中浮层 */}
      {open && currentPage && (
        <div
          style={{
            position: 'fixed',
            inset: 0,
            zIndex: 'var(--z-onboarding)',
            display: 'flex',
            alignItems: 'center',
            justifyContent: 'center',
            background: 'var(--bg-overlay)',
            backdropFilter: 'blur(4px)',
          }}
          onClick={handleClose}
        >
          <div
            ref={cardRef}
            onClick={(e) => e.stopPropagation()}
            style={{
              background: 'var(--bg-elevated)',
              borderRadius: 16,
              boxShadow: '0 20px 60px rgba(0,0,0,0.18)',
              border: '1px solid var(--border-subtle)',
              maxWidth: 500,
              width: '90%',
              maxHeight: '85vh',
              display: 'flex',
              flexDirection: 'column',
              padding: 0,
            }}
          >
            {/* Header */}
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                padding: '16px 20px',
                borderBottom: '1px solid var(--border-subtle)',
                flexShrink: 0,
              }}
            >
              <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                <div
                  style={{
                    width: 32,
                    height: 32,
                    borderRadius: 10,
                    background: 'color-mix(in srgb, var(--accent-primary) 12%, transparent)',
                    display: 'flex',
                    alignItems: 'center',
                    justifyContent: 'center',
                  }}
                >
                  <currentPage.icon
                    size={ICON_SIZE.md}
                    style={{ color: 'var(--accent-primary)' }}
                  />
                </div>
                <span
                  style={{
                    fontSize: 'var(--text-section-title)',
                    fontWeight: 700,
                    color: 'var(--text-primary)',
                  }}
                >
                  {currentPage.title}
                </span>
              </div>
              <button
                onClick={handleClose}
                style={{
                  padding: 6,
                  borderRadius: 8,
                  border: 'none',
                  background: 'transparent',
                  cursor: 'pointer',
                  color: 'var(--text-tertiary)',
                  transition: 'all 0.15s ease',
                }}
                onMouseEnter={(e) => {
                  e.currentTarget.style.background =
                    'color-mix(in srgb, var(--accent-primary) 12%, transparent)';
                  e.currentTarget.style.color = 'var(--accent-primary)';
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'transparent';
                  e.currentTarget.style.color = 'var(--text-tertiary)';
                }}
              >
                <X size={ICON_SIZE.xl} />
              </button>
            </div>

            {/* 页面指示圆点 */}
            <div
              style={{
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'center',
                gap: 8,
                padding: '12px 20px 0',
                flexShrink: 0,
              }}
            >
              {pages.map((p, i) => (
                <button
                  key={i}
                  onClick={() => goTo(i)}
                  style={{
                    width: i === pageIndex ? 24 : 8,
                    height: 8,
                    borderRadius: 4,
                    border: 'none',
                    background: i === pageIndex ? 'var(--accent-primary)' : 'var(--border-subtle)',
                    cursor: 'pointer',
                    transition: 'all 0.25s ease',
                    padding: 0,
                  }}
                  title={p.title}
                />
              ))}
            </div>

            {/* 可滑动内容区域 — 触摸翻页 + 动画 */}
            <div
              ref={contentRef}
              onTouchStart={handleTouchStart}
              onTouchEnd={handleTouchEnd}
              style={{
                flex: 1,
                minHeight: 0,
                overflowY: 'auto',
                overflowX: 'hidden',
                position: 'relative',
                overscrollBehavior: 'contain',
              }}
            >
              <AnimatePresence mode="wait" custom={direction.current}>
                <motion.div
                  key={pageIndex}
                  custom={direction.current}
                  variants={pageVariants}
                  initial="enter"
                  animate="center"
                  exit="exit"
                  transition={{
                    x: { type: 'spring', stiffness: 300, damping: 30 },
                    opacity: { duration: 0.2 },
                  }}
                  style={{ padding: '16px 20px 8px' }}
                >
                  {/* Steps */}
                  <div style={{ display: 'flex', flexDirection: 'column', gap: 0 }}>
                    {currentPage.steps.map((step, index) => {
                      const Icon = step.icon;
                      const isLastStep = index === currentPage.steps.length - 1;
                      return (
                        <div
                          key={index}
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
                                {index + 1}
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
                  {currentPage.helpLinks.length > 0 && (
                    <div
                      style={{
                        margin: '8px 0',
                        padding: '12px',
                        borderRadius: 10,
                        background: 'var(--bg-toolbar)',
                        border: '1px solid var(--border-subtle)',
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
                        {t('related_docs') ?? '相关帮助文档'}
                      </div>
                      <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
                        {currentPage.helpLinks.map((link, i) => (
                          <button
                            key={i}
                            onClick={() => handleHelpLinkClick(link.href)}
                            style={{
                              display: 'flex',
                              alignItems: 'center',
                              gap: 10,
                              padding: '8px 10px',
                              borderRadius: 8,
                              border: '1px solid var(--border-subtle)',
                              background: 'var(--bg-elevated)',
                              cursor: 'pointer',
                              textAlign: 'left',
                              width: '100%',
                              transition: 'all 0.15s ease',
                              fontFamily: 'inherit',
                            }}
                            onMouseEnter={(e) => {
                              e.currentTarget.style.borderColor = 'var(--accent-primary)';
                              e.currentTarget.style.background =
                                'color-mix(in srgb, var(--accent-primary) 4%, transparent)';
                            }}
                            onMouseLeave={(e) => {
                              e.currentTarget.style.borderColor = 'var(--border-subtle)';
                              e.currentTarget.style.background = 'var(--bg-elevated)';
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
                </motion.div>
              </AnimatePresence>
            </div>

            {/* Footer — 导航 + 关闭 */}
            <div
              style={{
                padding: '10px 20px',
                borderTop: '1px solid var(--border-subtle)',
                display: 'flex',
                alignItems: 'center',
                justifyContent: 'space-between',
                flexShrink: 0,
              }}
            >
              {/* Prev */}
              <button
                onClick={handlePrev}
                disabled={isFirst}
                style={{
                  display: 'inline-flex',
                  alignItems: 'center',
                  gap: 4,
                  padding: '6px 12px',
                  borderRadius: 8,
                  border: '1px solid var(--border-subtle)',
                  background: 'var(--bg-toolbar)',
                  color: isFirst ? 'var(--text-disabled)' : 'var(--text-secondary)',
                  cursor: isFirst ? 'default' : 'pointer',
                  fontSize: 'var(--text-badge)',
                  fontWeight: 500,
                  opacity: isFirst ? 0.4 : 1,
                  transition: 'all 0.15s ease',
                  fontFamily: 'inherit',
                }}
                onMouseEnter={(e) => {
                  if (!isFirst) {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }
                }}
                onMouseLeave={(e) => {
                  e.currentTarget.style.background = 'var(--bg-toolbar)';
                  e.currentTarget.style.borderColor = 'var(--border-subtle)';
                  e.currentTarget.style.color = isFirst
                    ? 'var(--text-disabled)'
                    : 'var(--text-secondary)';
                }}
              >
                <ChevronLeft size={ICON_SIZE.xs} />
                {t('previous') ?? '上一页'}
              </button>

              {/* Page count */}
              <span
                style={{
                  fontSize: 'var(--text-badge)',
                  color: 'var(--text-tertiary)',
                  fontWeight: 500,
                }}
              >
                {pageIndex + 1} / {pages.length}
              </span>

              {/* Next or Got it */}
              {!isLast ? (
                <button
                  onClick={handleNext}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 4,
                    padding: '6px 12px',
                    borderRadius: 8,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                    fontSize: 'var(--text-badge)',
                    fontWeight: 500,
                    transition: 'all 0.15s ease',
                    fontFamily: 'inherit',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-secondary)';
                  }}
                >
                  {t('next') ?? '下一页'}
                  <ChevronRight size={ICON_SIZE.xs} />
                </button>
              ) : (
                <button
                  onClick={handleClose}
                  style={{
                    display: 'inline-flex',
                    alignItems: 'center',
                    gap: 4,
                    padding: '6px 14px',
                    borderRadius: 8,
                    border: '1px solid var(--border-subtle)',
                    background: 'var(--bg-toolbar)',
                    color: 'var(--text-secondary)',
                    cursor: 'pointer',
                    fontSize: 'var(--text-badge)',
                    fontWeight: 500,
                    transition: 'all 0.15s ease',
                    fontFamily: 'inherit',
                  }}
                  onMouseEnter={(e) => {
                    e.currentTarget.style.background =
                      'color-mix(in srgb, var(--accent-primary) 10%, transparent)';
                    e.currentTarget.style.borderColor = 'var(--accent-primary)';
                    e.currentTarget.style.color = 'var(--accent-primary)';
                  }}
                  onMouseLeave={(e) => {
                    e.currentTarget.style.background = 'var(--bg-toolbar)';
                    e.currentTarget.style.borderColor = 'var(--border-subtle)';
                    e.currentTarget.style.color = 'var(--text-secondary)';
                  }}
                >
                  {t('got_it') ?? '知道了'}
                  <ChevronRight size={ICON_SIZE.xs} />
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
}
