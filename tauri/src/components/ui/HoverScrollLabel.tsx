import { useEffect, useState } from 'react';
import { useScrollingNavLabel } from '@/hooks/useScrollingNavLabel';
import { supportsHover } from '@/lib/platform';
import styles from './HoverScrollLabel.module.css';

/** 与侧栏名称共用滚动节奏；按钮内响应整行悬停/聚焦，只移动文字层。 */
export function HoverScrollLabel({
  text,
  className,
  focusable = false,
}: {
  text: string;
  className?: string;
  focusable?: boolean;
}) {
  const [hovered, setHovered] = useState(false);
  const [focused, setFocused] = useState(false);
  const { viewportRef, textRef, isOverflowing, staticMode } = useScrollingNavLabel(
    text,
    true,
    hovered || focused,
  );
  useEffect(() => {
    const viewport = viewportRef.current;
    const target = viewport?.closest('button') ?? viewport;
    if (!target) return;
    const enter = () => setHovered(supportsHover());
    const leave = () => setHovered(false);
    const focus = () => setFocused(target.matches(':focus-visible'));
    const blur = () => setFocused(false);
    target.addEventListener('mouseenter', enter);
    target.addEventListener('mouseleave', leave);
    target.addEventListener('focusin', focus);
    target.addEventListener('focusout', blur);
    return () => {
      target.removeEventListener('mouseenter', enter);
      target.removeEventListener('mouseleave', leave);
      target.removeEventListener('focusin', focus);
      target.removeEventListener('focusout', blur);
    };
  }, [viewportRef]);
  return (
    <span
      ref={viewportRef}
      className={`${styles.viewport} ${className ?? ''}`}
      data-hover-scroll-label
      data-overflow={isOverflowing}
      data-static={staticMode || undefined}
      tabIndex={focusable && isOverflowing ? 0 : undefined}
      title={staticMode && isOverflowing ? text : undefined}
    >
      <span ref={textRef} className={styles.track} data-hover-scroll-track>
        {text}
      </span>
    </span>
  );
}
