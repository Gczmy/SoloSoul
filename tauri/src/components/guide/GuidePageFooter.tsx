import { ChevronLeft, ChevronRight } from 'lucide-react';
import { ICON_SIZE } from '@/lib/constants';
import type { TFunction } from 'i18next';

/**
 * P013/4: 指南卡片底部导航 — 上一页 / 页码 / 下一页或「知道了」。
 */
export function GuidePageFooter({
  t,
  pageIndex,
  total,
  isFirst,
  isLast,
  onPrev,
  onNext,
  onClose,
}: {
  t: TFunction;
  pageIndex: number;
  total: number;
  isFirst: boolean;
  isLast: boolean;
  onPrev: () => void;
  onNext: () => void;
  onClose: () => void;
}) {
  return (
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
        onClick={onPrev}
        disabled={isFirst}
        className="interactive-toolbar"
        style={{
          display: 'inline-flex',
          alignItems: 'center',
          gap: 4,
          padding: '6px 12px',
          borderRadius: 8,
          borderWidth: 1,
          borderStyle: 'solid',
          cursor: isFirst ? 'default' : 'pointer',
          fontSize: 'var(--text-badge)',
          fontWeight: 500,
          opacity: isFirst ? 0.4 : 1,
          fontFamily: 'inherit',
        }}
      >
        <ChevronLeft size={ICON_SIZE.xs} />
        {t('previous', { defaultValue: '上一页' })}
      </button>

      {/* Page count */}
      <span
        style={{
          fontSize: 'var(--text-badge)',
          color: 'var(--text-tertiary)',
          fontWeight: 500,
        }}
      >
        {pageIndex + 1} / {total}
      </span>

      {/* Next or Got it */}
      {!isLast ? (
        <button
          onClick={onNext}
          className="interactive-toolbar"
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: 4,
            padding: '6px 12px',
            borderRadius: 8,
            borderWidth: 1,
            borderStyle: 'solid',
            cursor: 'pointer',
            fontSize: 'var(--text-badge)',
            fontWeight: 500,
            fontFamily: 'inherit',
          }}
        >
          {t('next', { defaultValue: '下一页' })}
          <ChevronRight size={ICON_SIZE.xs} />
        </button>
      ) : (
        <button
          onClick={onClose}
          className="interactive-toolbar"
          style={{
            display: 'inline-flex',
            alignItems: 'center',
            gap: 4,
            padding: '6px 14px',
            borderRadius: 8,
            borderWidth: 1,
            borderStyle: 'solid',
            cursor: 'pointer',
            fontSize: 'var(--text-badge)',
            fontWeight: 500,
            fontFamily: 'inherit',
          }}
        >
          {t('got_it', { defaultValue: '知道了' })}
          <ChevronRight size={ICON_SIZE.xs} />
        </button>
      )}
    </div>
  );
}
