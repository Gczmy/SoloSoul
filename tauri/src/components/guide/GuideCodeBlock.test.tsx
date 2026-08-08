import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { GuideCodeBlock } from './GuideCodeBlock';

describe('GuideCodeBlock sensitivity badge rendering', () => {
  it('renders a real SensitivityBadge (with colored border) for inline level keys', () => {
    render(<GuideCodeBlock>critical</GuideCodeBlock>);
    const label = screen.getByText('critical');
    const badgeSpan = label.closest('span');
    // 应用内徽章：带颜色边框（1px solid + 前景色）+ 圆角 + 背景色
    expect(badgeSpan).toHaveStyle({
      border: '1px solid #C0392B',
      borderRadius: '4px',
      display: 'inline-flex',
      fontWeight: '600',
    });
    expect(badgeSpan).toHaveAttribute('title', expect.stringContaining('critical'));
  });

  it.each(['public', 'internal', 'sensitive', 'critical'])(
    'renders a bordered badge for %s',
    (level) => {
      const { unmount } = render(<GuideCodeBlock>{level}</GuideCodeBlock>);
      const label = screen.getByText(level);
      const badgeSpan = label.closest('span');
      expect(badgeSpan).toHaveStyle({ display: 'inline-flex', fontWeight: '600' });
      expect(badgeSpan?.style.border).toMatch(/^1px solid /);
      unmount();
    }
  );

  it('renders plain inline code for non-level tokens', () => {
    render(<GuideCodeBlock>passport_number</GuideCodeBlock>);
    const code = screen.getByText('passport_number');
    // 普通行内代码：直接是 <code> 元素（无徽章包裹 span、无颜色边框）
    expect(code.tagName).toBe('CODE');
    expect(code.closest('span')).toBeNull();
  });

  it('does not render a badge inside block code', () => {
    // 带语言 class 的块级代码即使内容为级别 key，也必须是代码块而非徽章
    render(<GuideCodeBlock className="language-md">critical</GuideCodeBlock>);
    expect(screen.getByText('critical').tagName).toBe('CODE');
  });
});
