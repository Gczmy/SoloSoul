import { fireEvent, render, screen } from '@testing-library/react';
import { BookOpen } from 'lucide-react';
import { MemoryRouter } from 'react-router-dom';
import { describe, expect, it } from 'vitest';
import { PageGuide, type GuidePage } from './PageGuide';

const pages: GuidePage[] = [
  { icon: BookOpen, title: '第一页', steps: [], helpLinks: [] },
  { icon: BookOpen, title: '第二页', steps: [], helpLinks: [] },
];

describe('PageGuide 触摸中断', () => {
  it('系统取消横向手势时回到当前页，下一次滑动仍能翻页', () => {
    render(
      <MemoryRouter>
        <PageGuide pages={pages} label="打开指南" />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('button', { name: '打开指南' }));
    const dialog = screen.getByRole('dialog', { name: '第一页' });
    const container = dialog.querySelector<HTMLElement>('[style*="touch-action: pan-y"]');
    expect(container).not.toBeNull();
    Object.defineProperty(container, 'offsetWidth', { value: 300 });
    const strip = container!.firstElementChild as HTMLElement;

    fireEvent.touchStart(container!, { touches: [{ clientX: 250, clientY: 100 }] });
    fireEvent.touchMove(container!, { touches: [{ clientX: 170, clientY: 100 }] });
    expect(strip.style.transform).toContain('-80px');

    fireEvent.touchCancel(container!, { changedTouches: [{ clientX: 170, clientY: 100 }] });
    expect(strip.style.transform).toBe('translateX(-0%)');
    expect(screen.getByRole('dialog', { name: '第一页' })).toBeInTheDocument();

    fireEvent.touchStart(container!, { touches: [{ clientX: 250, clientY: 100 }] });
    fireEvent.touchMove(container!, { touches: [{ clientX: 170, clientY: 100 }] });
    fireEvent.touchEnd(container!, { changedTouches: [{ clientX: 170, clientY: 100 }] });
    expect(screen.getByRole('dialog', { name: '第二页' })).toBeInTheDocument();
  });
});

describe('PageGuide 关闭操作', () => {
  it('图标关闭按钮有可访问名称，关闭后焦点回到触发器', () => {
    render(
      <MemoryRouter>
        <PageGuide pages={pages} label="打开指南" />
      </MemoryRouter>,
    );
    const trigger = screen.getByRole('button', { name: '打开指南' });
    fireEvent.click(trigger);

    fireEvent.click(screen.getByRole('button', { name: '关闭' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });
});
