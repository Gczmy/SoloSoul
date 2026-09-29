import { fireEvent, render, screen } from '@testing-library/react';
import { BookOpen } from 'lucide-react';
import { MemoryRouter, Route, Routes } from 'react-router-dom';
import { describe, expect, it, vi } from 'vitest';
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

describe('PageGuide 页面导航', () => {
  it('通过底栏翻页、返回，并在末页关闭后重新从首页打开', () => {
    render(
      <MemoryRouter>
        <PageGuide pages={pages} label="打开指南" />
      </MemoryRouter>,
    );
    const trigger = screen.getByRole('button', { name: '打开指南' });
    fireEvent.click(trigger);

    expect(screen.getByRole('dialog', { name: '第一页' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '上一页' })).toBeDisabled();
    expect(screen.getByText('1 / 2')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '下一页' }));
    expect(screen.getByRole('dialog', { name: '第二页' })).toBeInTheDocument();
    expect(screen.getByText('2 / 2')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: '上一页' }));
    expect(screen.getByRole('dialog', { name: '第一页' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: '下一页' }));
    fireEvent.click(screen.getByRole('button', { name: '知道了' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();

    fireEvent.click(trigger);
    expect(screen.getByRole('dialog', { name: '第一页' })).toBeInTheDocument();
    expect(screen.getByText('1 / 2')).toBeInTheDocument();
  });

  it('点击相关帮助后关闭指南并打开目标页面', () => {
    const linkedPages: GuidePage[] = [
      {
        ...pages[0],
        helpLinks: [{ title: '同步帮助', description: '查看同步说明', href: '/help/sync' }],
      },
    ];
    render(
      <MemoryRouter>
        <PageGuide pages={linkedPages} label="打开指南" />
        <Routes>
          <Route path="/help/sync" element={<div>同步帮助页面</div>} />
        </Routes>
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('button', { name: '打开指南' }));
    fireEvent.click(screen.getByRole('button', { name: /同步帮助/ }));

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(screen.getByText('同步帮助页面')).toBeInTheDocument();
  });

  it('Escape 和遮罩点击均可关闭，并将焦点还给触发器', () => {
    render(
      <MemoryRouter>
        <PageGuide pages={pages} label="打开指南" />
      </MemoryRouter>,
    );
    const trigger = screen.getByRole('button', { name: '打开指南' });
    fireEvent.click(trigger);
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();

    fireEvent.click(trigger);
    fireEvent.click(document.querySelector('[data-page-guide-overlay]')!);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it('卡片和触发器内的鼠标按下保持打开，点击外部才关闭', async () => {
    render(
      <MemoryRouter>
        <PageGuide pages={pages} label="打开指南" />
      </MemoryRouter>,
    );
    const trigger = screen.getByRole('button', { name: '打开指南' });
    fireEvent.click(trigger);
    await new Promise((resolve) => setTimeout(resolve, 0));

    fireEvent.mouseDown(screen.getByRole('dialog', { name: '第一页' }));
    fireEvent.mouseDown(trigger);
    expect(screen.getByRole('dialog', { name: '第一页' })).toBeInTheDocument();

    fireEvent.mouseDown(document.body);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it('Tab 键在指南卡片首尾操作之间循环', () => {
    render(
      <MemoryRouter>
        <PageGuide pages={pages} label="打开指南" />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('button', { name: '打开指南' }));
    const dialog = screen.getByRole('dialog', { name: '第一页' });
    const visibleRects = vi.spyOn(HTMLElement.prototype, 'getClientRects').mockReturnValue({
      length: 1,
    } as DOMRectList);

    try {
      fireEvent.keyDown(document, { key: 'Tab', shiftKey: true });
      expect(screen.getByRole('button', { name: '下一页' })).toHaveFocus();

      fireEvent.keyDown(document, { key: 'Tab' });
      expect(screen.getByRole('button', { name: '关闭' })).toHaveFocus();
      expect(dialog).toBeInTheDocument();
    } finally {
      visibleRects.mockRestore();
    }
  });
});

describe('PageGuide 触摸方向与边界', () => {
  it('竖向滚动不翻页，末页继续左划不越界，右划可返回', () => {
    render(
      <MemoryRouter>
        <PageGuide pages={pages} label="打开指南" />
      </MemoryRouter>,
    );
    fireEvent.click(screen.getByRole('button', { name: '打开指南' }));
    const dialog = screen.getByRole('dialog', { name: '第一页' });
    const container = dialog.querySelector<HTMLElement>('[style*="touch-action: pan-y"]')!;
    Object.defineProperty(container, 'offsetWidth', { value: 300 });
    const strip = container.firstElementChild as HTMLElement;

    fireEvent.touchStart(container, { touches: [{ clientX: 250, clientY: 100 }] });
    fireEvent.touchMove(container, { touches: [{ clientX: 245, clientY: 170 }] });
    fireEvent.touchEnd(container, { changedTouches: [{ clientX: 245, clientY: 170 }] });
    expect(strip.style.transform).toBe('translateX(-0%)');
    expect(screen.getByRole('dialog', { name: '第一页' })).toBeInTheDocument();

    fireEvent.touchStart(container, { touches: [{ clientX: 250, clientY: 100 }] });
    fireEvent.touchMove(container, { touches: [{ clientX: 170, clientY: 100 }] });
    fireEvent.touchEnd(container, { changedTouches: [{ clientX: 170, clientY: 100 }] });
    expect(screen.getByRole('dialog', { name: '第二页' })).toBeInTheDocument();

    fireEvent.touchStart(container, { touches: [{ clientX: 250, clientY: 100 }] });
    fireEvent.touchMove(container, { touches: [{ clientX: 170, clientY: 100 }] });
    fireEvent.touchEnd(container, { changedTouches: [{ clientX: 170, clientY: 100 }] });
    expect(screen.getByRole('dialog', { name: '第二页' })).toBeInTheDocument();

    fireEvent.touchStart(container, { touches: [{ clientX: 100, clientY: 100 }] });
    fireEvent.touchMove(container, { touches: [{ clientX: 180, clientY: 100 }] });
    fireEvent.touchEnd(container, { changedTouches: [{ clientX: 180, clientY: 100 }] });
    expect(screen.getByRole('dialog', { name: '第一页' })).toBeInTheDocument();
  });
});
