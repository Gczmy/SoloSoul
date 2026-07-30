import { PageGuide, type GuidePage } from '@/components/guide/PageGuide';
import { isMobilePlatformSync } from '@/lib/platform';

interface PageGuideButtonProps {
  pages: GuidePage[];
  label?: string;
}

/**
 * PageGuideButton — 页面级指南触发按钮。
 *
 * - 桌面端：显示图标 + 文本标签
 * - 移动端（Android/iOS）：仅显示图标，节省空间
 */
export function PageGuideButton({ pages, label }: PageGuideButtonProps) {
  return <PageGuide pages={pages} label={label} compact={isMobilePlatformSync()} />;
}
