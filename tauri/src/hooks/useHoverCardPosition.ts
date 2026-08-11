import { useCallback, useEffect, useState } from 'react';
import type { CSSProperties, RefObject } from 'react';
import { supportsHover } from '@/lib/platform';

interface UseHoverCardPositionOptions {
  isHorizontal: boolean;
  isBottom: boolean;
  isRight: boolean;
}

/**
 * P010: 共享悬停名称卡片定位逻辑（AddPageButton / NavButton 原本各复制一份）。
 *
 * 依据 wrapper 元素与按钮方位计算固定定位卡片样式，返回悬停状态与鼠标进出回调；
 * 悬停期间随 window scroll（捕获阶段）/resize 实时更新位置，触屏设备（无 hover 能力）
 * 一律不触发卡片。
 */
export function useHoverCardPosition(
  wrapperRef: RefObject<HTMLDivElement | null>,
  { isHorizontal, isBottom, isRight }: UseHoverCardPositionOptions,
) {
  const [cardStyle, setCardStyle] = useState<CSSProperties | null>(null);
  const [isHovered, setIsHovered] = useState(false);

  const updateCardPosition = useCallback(() => {
    if (wrapperRef.current) {
      const rect = wrapperRef.current.getBoundingClientRect();
      if (isHorizontal) {
        if (isBottom) {
          setCardStyle({
            top: 'auto',
            bottom: window.innerHeight - rect.top + 8,
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        } else {
          setCardStyle({
            top: rect.bottom + 8,
            bottom: 'auto',
            left: rect.left + rect.width / 2,
            transform: 'translateX(-50%)',
          });
        }
      } else if (isRight) {
        setCardStyle({
          top: rect.top + rect.height / 2,
          bottom: 'auto',
          left: 'auto',
          right: window.innerWidth - rect.left + 8,
          transform: 'translateY(-50%)',
        });
      } else {
        setCardStyle({
          top: rect.top + rect.height / 2,
          bottom: 'auto',
          left: rect.right + 8,
          transform: 'translateY(-50%)',
        });
      }
    }
  }, [isHorizontal, isBottom, isRight, wrapperRef]);

  const handleMouseEnter = useCallback(() => {
    // 触屏设备不触发悬停卡片（Android WebView hover 会粘住）
    if (!supportsHover()) return;
    setIsHovered(true);
    updateCardPosition();
  }, [updateCardPosition]);

  const handleMouseLeave = useCallback(() => {
    setIsHovered(false);
  }, []);

  useEffect(() => {
    if (!isHovered) return;
    window.addEventListener('scroll', updateCardPosition, true);
    window.addEventListener('resize', updateCardPosition);
    return () => {
      window.removeEventListener('scroll', updateCardPosition, true);
      window.removeEventListener('resize', updateCardPosition);
    };
  }, [isHovered, updateCardPosition]);

  return { cardStyle, isHovered, handleMouseEnter, handleMouseLeave };
}
