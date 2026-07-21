import { useState, useRef, useEffect, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { useTranslation } from 'react-i18next';
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
 * 渲染一个「圆圈问号图标 + 文本」触发器按钮，点击后弹出多页指南卡片，
 * 每页包含：步骤列表 + 相关帮助文档跳转卡片。
 * 支持上/下一页导航、页面指示小圆点、移动端左右滑动手势翻页（实时跟手滑动）。
 */
export function PageGuide({ pages, label }: PageGuideProps) {
  const { t } = useTranslation('common');
  const navigate = useNavigate();
  const [open, setOpen] = useState(false);
  const [hovered, setHovered] = useState(false);
  const [pageIndex, setPageIndex] = useState(0);
  const [isSnapping, setIsSnapping] = useState(false);
  const triggerRef = useRef<HTMLButtonElement>(null);
  const cardRef = useRef<HTMLDivElement>(null);
  const stripRef = useRef<HTMLDivElement>(null);
  const touchStartX = useRef(0);
  const touchStartY = useRef(0);
  const containerWidth = useRef(0);
  // 以下 ref 用于拖拽期间的高频更新（不触发 React 重渲染）
  const dragOffsetRef = useRef(0);
  const isDraggingRef = useRef(false);
  const pageIndexRef = useRef(0);

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
      isDraggingRef.current = false;
      dragOffsetRef.current = 0;
      pageIndexRef.current = index;
      setIsSnapping(true);
      setPageIndex(index);
      // 立即设置 strip 到目标位置（带过渡动画），确保浏览器在此帧内开始动画
      if (stripRef.current) {
        stripRef.current.style.transition = 'transform 0.3s cubic-bezier(0.4, 0, 0.2, 1)';
        stripRef.current.style.transform = `translateX(-${index * (100 / pages.length)}%)`;
      }
      setTimeout(() => setIsSnapping(false), 350);
    },
    [pageIndex, pages.length],
  );

  const handlePrev = useCallback(() => {
    if (!isFirst) goTo(pageIndex - 1);
  }, [isFirst, goTo, pageIndex]);

  const handleNext = useCallback(() => {
    if (!isLast) goTo(pageIndex + 1);
  }, [isLast, goTo, pageIndex]);

  // 触摸滑动手势（使用 ref 避免高频重渲染）
  const handleTouchStart = useCallback((e: React.TouchEvent) => {
    touchStartX.current = e.touches[0].clientX;
    touchStartY.current = e.touches[0].clientY;
    containerWidth.current = (e.currentTarget as HTMLElement).offsetWidth;
    isDraggingRef.current = true;
    dragOffsetRef.current = 0;
    pageIndexRef.current = pageIndex;
    setIsSnapping(false); // 快速连续滑动时立即打断上一次快照动画
  }, [pageIndex]);

  const handleTouchMove = useCallback((e: React.TouchEvent) => {
    if (!isDraggingRef.current) return;
    const dx = e.touches[0].clientX - touchStartX.current;
    const dy = e.touches[0].clientY - touchStartY.current;
    dragOffsetRef.current = dx;
    if (stripRef.current && Math.abs(dx) > Math.abs(dy)) {
      // 直接操作 DOM，不触发 React 重渲染
      stripRef.current.style.transition = 'none';
      const pagePct = pageIndexRef.current * (100 / pages.length);
      stripRef.current.style.transform = `translateX(calc(-${pagePct}% + ${dx}px))`;
    }
  }, []);

  const handleTouchEnd = useCallback(
    (e: React.TouchEvent) => {
      isDraggingRef.current = false;
      const dx = e.changedTouches[0].clientX - touchStartX.current;
      const dy = e.changedTouches[0].clientY - touchStartY.current;
      const threshold = Math.min(50, containerWidth.current * 0.15);

      if (Math.abs(dx) > threshold && Math.abs(dx) > Math.abs(dy) * 1.5) {
        if (dx > 0 && pageIndex > 0) {
          goTo(pageIndex - 1);
          return;
        }
        if (dx < 0 && pageIndex < pages.length - 1) {
          goTo(pageIndex + 1);
          return;
        }
      }
      // 未触发翻页：回弹到当前页
      setIsSnapping(true);
      dragOffsetRef.current = 0;
      if (stripRef.current) {
        stripRef.current.style.transition = 'transform 0.3s cubic-bezier(0.4, 0, 0.2, 1)';
        stripRef.current.style.transform = `translateX(-${pageIndex * (100 / pages.length)}%)`;
      }
      setTimeout(() => setIsSnapping(false), 350);
    },
    [isFirst, isLast, goTo, pageIndex, pages.length],
  );

  const handleHelpLinkClick = (href: string) => {
    setOpen(false);
    navigate(href);
  };

  const handleClose = () => {
    setOpen(false);
  };

  // 拖拽结束后通过 state 渲染正确位置；拖拽中通过 ref 直接操作 DOM
  // translateX 百分比相对于 strip 自身宽度，故需除以 pages.length
  const pagePct = pageIndex * (100 / pages.length);
  const stripTransform = `translateX(-${pagePct}%)`;
  const stripTransition = 'transform 0.3s cubic-bezier(0.4, 0, 0.2, 1)';

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

            {/* 页面指示小圆点 */}
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
                    width: 5,
                    height: 5,
                    borderRadius: '50%',
                    border: 'none',
                    padding: 0,
                    cursor: 'pointer',
                    background: i === pageIndex ? 'var(--accent-primary)' : 'var(--border-subtle)',
                    transition: 'all 0.25s ease',
                  }}
                  title={p.title}
                  aria-label={p.title}
                />
              ))}
            </div>

            {/* 滑动条容器 — 左右滑动实时跟手 */}
            <div
              onTouchStart={handleTouchStart}
              onTouchMove={handleTouchMove}
              onTouchEnd={handleTouchEnd}
              style={{
                flex: 1,
                minHeight: 0,
                overflow: 'hidden',
                position: 'relative',
                overscrollBehavior: 'contain',
              }}
            >
              <div
                ref={stripRef}
                style={{
                  display: 'flex',
                  width: `${pages.length * 100}%`,
                  height: '100%',
                  transform: stripTransform,
                  transition: stripTransition,
                  willChange: 'transform',
                }}
              >
                {pages.map((page, pageIdx) => (
                  <div
                    key={pageIdx}
                    style={{
                      width: `${100 / pages.length}%`,
                      flexShrink: 0,
                      overflowY: 'auto',
                      overflowX: 'hidden',
                      padding: '16px 20px 8px',
                      boxSizing: 'border-box',
                    }}
                  >
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
                                <Icon
                                  size={ICON_SIZE.sm}
                                  style={{ color: 'var(--accent-primary)' }}
                                />
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
                          {page.helpLinks.map((link, i) => (
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
                  </div>
                ))}
              </div>
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
