import { describe, it, expect } from 'vitest';
import { render } from '@testing-library/react';
import { RecoveryDialogSkeleton } from './RecoveryDialogSkeleton';

describe('RecoveryDialogSkeleton', () => {
  it('renders the dialog-like layout as a purely decorative placeholder', () => {
    const { container } = render(<RecoveryDialogSkeleton />);

    // 与 RouteLoadingSkeleton 同策略：整体 aria-hidden（纯装饰占位，无硬编码文案）
    expect(container.querySelector('[aria-hidden="true"]')).not.toBeNull();
    // 布局结构：遮罩 + 卡片 + 标题/标签页/内容行/内容区占位块
    const overlay = container.firstChild as HTMLElement;
    expect(overlay.className).toContain('overlay');
    expect(overlay.querySelectorAll('div').length).toBeGreaterThanOrEqual(8);
  });
});
