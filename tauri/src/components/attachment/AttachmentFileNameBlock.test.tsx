import { describe, it, expect } from 'vitest';
import { render, screen, fireEvent, act } from '@testing-library/react';
import { AttachmentFileNameBlock, hasTagOverflow } from './AttachmentFileNameBlock';
import { resizeObserverInstances } from '@/test/setup';

const base = {
  fileName: 'photo.png',
  sizeBytes: 1024,
  createdAt: '2026-07-01T00:00:00Z',
  showTrash: false,
};

describe('AttachmentFileNameBlock 描述折叠/展开', () => {
  it('短描述不显示展开箭头（未溢出时无意义按钮）', () => {
    render(<AttachmentFileNameBlock {...base} description="短描述" />);
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(screen.getByText('短描述')).toBeInTheDocument();
  });

  it('长描述（溢出）默认折叠显示省略箭头，点击展开全文后箭头变为收起', () => {
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    render(<AttachmentFileNameBlock {...base} description={longDesc} />);
    // jsdom 下无真实布局，scrollWidth 不溢出 → 箭头按溢出检测不显示。
    // 通过 CSS 断言折叠态截断样式存在（nowrap + ellipsis 语义由样式保证）。
    const descEl = screen.getByText(longDesc);
    expect(descEl).toHaveStyle({ whiteSpace: 'nowrap' });
    expect(descEl).toHaveStyle({ textOverflow: 'ellipsis' });
  });
});

describe('AttachmentFileNameBlock 标签折叠/展开', () => {
  it('标签不超过 4 个时全部显示且无 +N、无箭头', () => {
    const tags = ['旅行', '出差', '发票', '报销'];
    render(<AttachmentFileNameBlock {...base} tags={tags} />);
    tags.forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
    expect(screen.queryByText(/^\+/)).not.toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('超过 4 个标签默认折叠：只显示前 4 个 +「+N」pill + 展开箭头', () => {
    const tags = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];
    render(<AttachmentFileNameBlock {...base} tags={tags} />);
    // 折叠态：前 4 个可见
    ['a', 'b', 'c', 'd'].forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
    // 后 3 个不可见
    ['e', 'f', 'g'].forEach((t) => expect(screen.queryByText(t)).not.toBeInTheDocument());
    // +N pill
    expect(screen.getByText('+3')).toBeInTheDocument();
    // 展开箭头（默认折叠，aria-expanded=false）
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();
  });

  it('点击箭头展开全部标签，+N 消失、箭头变为收起态', () => {
    const tags = ['a', 'b', 'c', 'd', 'e'];
    render(<AttachmentFileNameBlock {...base} tags={tags} />);
    fireEvent.click(screen.getByRole('button', { expanded: false }));
    // 展开后全部标签可见
    tags.forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
    expect(screen.queryByText('+1')).not.toBeInTheDocument();
    // 箭头变收起态
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
    // 再点一次收起
    fireEvent.click(screen.getByRole('button', { expanded: true }));
    expect(screen.queryByText('e')).not.toBeInTheDocument();
    expect(screen.getByText('+1')).toBeInTheDocument();
  });

  it('hasTagOverflow：任一标签被省略（scrollWidth>clientWidth）即判定溢出', () => {
    // 模拟折叠态 DOM：两个 chip，一个被压缩省略
    const container = {
      querySelectorAll: () => [
        { scrollWidth: 60, clientWidth: 40 }, // 溢出 → 省略号
        { scrollWidth: 30, clientWidth: 40 }, // 未溢出
      ],
    } as unknown as HTMLElement;
    expect(hasTagOverflow(container)).toBe(true);

    // 全部未溢出 → false
    const okContainer = {
      querySelectorAll: () => [{ scrollWidth: 30, clientWidth: 40 }],
    } as unknown as HTMLElement;
    expect(hasTagOverflow(okContainer)).toBe(false);

    // 空容器 → false
    expect(hasTagOverflow(null)).toBe(false);
  });

  it('标签数量未超限但过长被省略时仍显示折叠按钮', () => {
    const longTag = '一个非常非常长的标签'.repeat(20);
    const tags = [longTag, '短标签'];
    const { rerender } = render(<AttachmentFileNameBlock {...base} tags={tags} />);
    // jsdom 无真实布局：初始渲染无溢出、无按钮
    expect(screen.queryByRole('button')).not.toBeInTheDocument();

    // 模拟真实布局：chip 文本溢出（scrollWidth > clientWidth）
    const chip = screen.getByText(longTag) as HTMLElement;
    Object.defineProperty(chip, 'scrollWidth', { configurable: true, value: 1000 });
    Object.defineProperty(chip, 'clientWidth', { configurable: true, value: 50 });

    // 换 tags 引用触发溢出检测 effect 重测 → 数量 2 未超 4、无 +N，但出现折叠按钮
    rerender(<AttachmentFileNameBlock {...base} tags={[...tags]} />);
    expect(screen.queryByText(/^\+/)).not.toBeInTheDocument();
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();

    // 点击展开 → 收起态按钮
    fireEvent.click(toggle);
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
  });

  it('折叠态单行（nowrap）压缩标签留出 +N 与按钮；展开态恢复换行', () => {
    const tags = ['超长的第一个标签文本'.repeat(30), 'b', 'c', 'd', 'e'];
    render(<AttachmentFileNameBlock {...base} tags={tags} />);
    // 折叠态：标签容器 nowrap + hidden，标签 chip 可压缩（flexShrink 1）且省略号截断
    const container = screen.getByText('超长的第一个标签文本'.repeat(30)).parentElement!;
    expect(container).toHaveStyle({ flexWrap: 'nowrap', overflow: 'hidden' });
    expect(screen.getByText('超长的第一个标签文本'.repeat(30))).toHaveStyle({
      flexShrink: '1',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      whiteSpace: 'nowrap',
    });
    // +N 不可压缩（flexShrink 0），保证留在行尾
    expect(screen.getByText('+1')).toHaveStyle({ flexShrink: '0' });
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();
    // 按钮固定在右侧外层 flex：不是 chips 容器（container）的子元素
    expect(container.contains(toggle)).toBe(false);
    expect(toggle.parentElement).not.toBe(container);
    // 展开态：换行显示全部
    fireEvent.click(toggle);
    expect(container).toHaveStyle({ flexWrap: 'wrap', overflow: 'visible' });
    expect(screen.getByText('超长的第一个标签文本'.repeat(30))).toHaveStyle({ flexShrink: '0' });
    tags.forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
  });

  it('容器变窄（ResizeObserver 触发）后标签被截断即出现折叠按钮', () => {
    const longTag = '一个非常长的标签'.repeat(20);
    const tags = [longTag, '短标签'];

    // 清空模块级实例收集（其他测试 render 也会 push 实例），只触发本测试的 observer
    resizeObserverInstances.length = 0;
    const { unmount } = render(<AttachmentFileNameBlock {...base} tags={tags} />);
    // 初始无布局信息：无按钮
    expect(screen.queryByRole('button')).not.toBeInTheDocument();

    // 模拟容器变窄后 chip 被压缩省略（scrollWidth > clientWidth）
    const chip = screen.getByText(longTag) as HTMLElement;
    Object.defineProperty(chip, 'scrollWidth', { configurable: true, value: 1000 });
    Object.defineProperty(chip, 'clientWidth', { configurable: true, value: 50 });

    // 触发 ResizeObserver 回调 → 重测溢出 → 出现折叠按钮（act 包裹 flush setState）
    act(() => {
      for (const inst of resizeObserverInstances) {
        inst.trigger();
      }
    });
    expect(screen.queryByText(/^\+/)).not.toBeInTheDocument();
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();

    unmount();
  });

  it('展开后长标签 chip 允许换行（whiteSpace normal + wordBreak）且按钮位置不变', () => {
    const longTag = '超长标签文本'.repeat(40);
    const tags = [longTag, 'b', 'c', 'd', 'e'];
    const { rerender } = render(<AttachmentFileNameBlock {...base} tags={tags} />);
    // 模拟折叠态溢出 → 出现按钮
    const chip = screen.getByText(longTag) as HTMLElement;
    Object.defineProperty(chip, 'scrollWidth', { configurable: true, value: 1000 });
    Object.defineProperty(chip, 'clientWidth', { configurable: true, value: 50 });
    rerender(<AttachmentFileNameBlock {...base} tags={[...tags]} />);

    const toggle = screen.getByRole('button', { expanded: false });
    const outerFlex = toggle.parentElement!;
    // 展开后：按钮仍在同一外层 flex（位置不变）
    fireEvent.click(toggle);
    const toggleAfter = screen.getByRole('button', { expanded: true });
    expect(toggleAfter.parentElement).toBe(outerFlex);
    // 展开后 chip 允许换行（阅读舒适），不再 nowrap 省略
    const chipAfter = screen.getByText(longTag) as HTMLElement;
    expect(chipAfter).toHaveStyle({ whiteSpace: 'normal', wordBreak: 'break-word' });
    expect(chipAfter).toHaveStyle({ textOverflow: undefined });
    // flex-basis:auto 的 item 默认宽度为内容 max-content，长标签会撑出容器——
    // 必须 maxWidth:100% 将 item 钳制在容器宽度内，whiteSpace:normal 才会触发换行
    expect(chipAfter).toHaveStyle({ maxWidth: '100%' });
    // 全部标签可见
    tags.forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
  });
});
