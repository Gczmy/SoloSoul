import { describe, it, expect, vi } from 'vitest';
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

  it('描述溢出（mock 布局）时显示折叠箭头，点击展开全文、再点收起', () => {
    // U006: 描述溢出测量分支此前零行为测试覆盖——仿照标签侧方案：mock
    // scrollWidth/clientWidth 后以不同内容 rerender 触发溢出检测 effect 重测。
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    const { rerender } = render(<AttachmentFileNameBlock {...base} description={longDesc} />);
    // 初始 jsdom 无布局：无按钮
    expect(screen.queryByRole('button')).not.toBeInTheDocument();

    // 模拟真实布局：描述文本溢出（scrollWidth > clientWidth）
    const descEl = screen.getByText(longDesc) as HTMLElement;
    Object.defineProperty(descEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(descEl, 'clientWidth', { configurable: true, value: 300 });

    // 以不同内容 rerender 触发 effect 重测（trim 后内容变化）→ 折叠态出现展开箭头
    rerender(<AttachmentFileNameBlock {...base} description={longDesc + '（续）'} />);
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();

    // 点击展开 → 全文可见、箭头变收起态；再点收起 → 箭头回展开态
    fireEvent.click(toggle);
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
    expect(screen.getByText(longDesc + '（续）')).toHaveStyle({ whiteSpace: 'pre-wrap' });
    fireEvent.click(screen.getByRole('button', { expanded: true }));
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
  });

  it('T003 触控优化：收起态整行可点展开；展开态仅「描述」把手行可收起（全文点击不折叠）', () => {
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    const { rerender } = render(<AttachmentFileNameBlock {...base} description={longDesc} />);
    const descEl = screen.getByText(longDesc) as HTMLElement;
    Object.defineProperty(descEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(descEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(<AttachmentFileNameBlock {...base} description={longDesc + '（续）'} />);

    const toggle = screen.getByRole('button', { expanded: false });
    const row = toggle.parentElement!;
    // 收起态整行可点（cursor pointer + touchAction manipulation）
    expect(row).toHaveStyle({ cursor: 'pointer', touchAction: 'manipulation' });
    // 点击描述文本（非按钮）→ 展开
    fireEvent.click(screen.getByText(longDesc + '（续）'));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
    // 展开态：点击全文**不**折叠（用户可自由选中/拖选复制文本）
    fireEvent.click(screen.getByText(longDesc + '（续）'));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
    // 点击「描述」折叠把手行 → 收起
    fireEvent.click(screen.getByText('描述'));
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
    // 按钮点击不冒泡到整行（不会双重切换）：一次点击按钮仅切一次状态
    fireEvent.click(screen.getByRole('button', { expanded: false }));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
  });

  it('展开态「描述」把手行叠双层半透明主题色背景（更深）；全文区仅第一层背景', () => {
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    const { rerender } = render(<AttachmentFileNameBlock {...base} description={longDesc} />);
    const descEl = screen.getByText(longDesc) as HTMLElement;
    Object.defineProperty(descEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(descEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(<AttachmentFileNameBlock {...base} description={longDesc + '（续）'} />);

    // 收起态：无「描述」标签行（把手行仅展开态出现）
    expect(screen.queryByText('描述')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { expanded: false }));

    // 展开态：「描述」把手行出现，带第二层更深的背景；外层块带第一层背景
    const label = screen.getByText('描述');
    const handleRow = label.parentElement!;
    expect(handleRow).toHaveStyle({
      background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
      cursor: 'pointer',
    });
    const block = handleRow.parentElement!;
    expect(block).toHaveStyle({
      background: 'color-mix(in srgb, var(--accent-primary) 6%, transparent)',
      borderRadius: '8px',
    });
  });

  it('T003 触控优化：按钮保持 18×18 不占额外行（无扩展热区，触控由整行承担）', () => {
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    const { rerender } = render(<AttachmentFileNameBlock {...base} description={longDesc} />);
    const descEl = screen.getByText(longDesc) as HTMLElement;
    Object.defineProperty(descEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(descEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(<AttachmentFileNameBlock {...base} description={longDesc + '（续）'} />);

    const toggle = screen.getByRole('button', { expanded: false });
    // 按钮自身即 18×18 点击目标，无绝对定位扩展子元素（不撑高行/不占两行）
    expect(toggle).toHaveStyle({ width: '18px', height: '18px' });
    expect(toggle.querySelector('span')).toBeNull();
    // 必须显式清零 min/min 尺寸，覆盖 global.css 移动端 button 44×44 触控基线——
    // 否则安卓端按钮被 min-height/min-width 撑成 44×44，视觉占两行（T003 根因）。
    expect(toggle).toHaveStyle({ minWidth: '0px', minHeight: '0px' });
  });

  it('T004 状态驱动样式：收起态常态、展开态保持点击效果（非 hover 残留）', () => {
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    const { rerender } = render(<AttachmentFileNameBlock {...base} description={longDesc} />);
    const descEl = screen.getByText(longDesc) as HTMLElement;
    Object.defineProperty(descEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(descEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(<AttachmentFileNameBlock {...base} description={longDesc + '（续）'} />);

    // 收起态：常态（tertiary 色 + 透明底）
    const collapsed = screen.getByRole('button', { expanded: false });
    expect(collapsed).toHaveStyle({ color: 'var(--text-tertiary)', background: 'transparent' });

    // 展开态：保持点击效果（accent 色 + 高亮底）——样式随 expanded 状态而非 hover 事件
    fireEvent.click(collapsed);
    const expanded = screen.getByRole('button', { expanded: true });
    expect(expanded).toHaveStyle({
      color: 'var(--accent-primary)',
      background: 'var(--bg-hover)',
    });

    // 再收起 → 恢复常态
    fireEvent.click(expanded);
    expect(screen.getByRole('button', { expanded: false })).toHaveStyle({
      color: 'var(--text-tertiary)',
      background: 'transparent',
    });
  });

  it('Y001 选区守卫：拖选文本（有非空选区）时点击描述行不切换折叠态，无选区时正常切换', () => {
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    const { rerender } = render(<AttachmentFileNameBlock {...base} description={longDesc} />);
    const descEl = screen.getByText(longDesc) as HTMLElement;
    Object.defineProperty(descEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(descEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(<AttachmentFileNameBlock {...base} description={longDesc + '（续）'} />);
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();

    // 模拟拖选完成后的状态：非空选区（getSelection 返回选中文本）
    const spy = vi
      .spyOn(window, 'getSelection')
      .mockReturnValue({ toString: () => '选中文本' } as Selection);
    try {
      // 点击描述文本区（click 照常派发到行容器）→ 有选区不切换，仍折叠
      fireEvent.click(screen.getByText(longDesc + '（续）'));
      expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
    } finally {
      spy.mockRestore();
    }

    // 无选区（正常点击）→ 切换展开
    fireEvent.click(screen.getByText(longDesc + '（续）'));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
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

  it('T003 触控优化：收起态整行可点展开；展开态仅「标签」把手行可收起（chip 点击不折叠）', () => {
    const tags = ['a', 'b', 'c', 'd', 'e', 'f', 'g'];
    render(<AttachmentFileNameBlock {...base} tags={tags} />);
    const toggle = screen.getByRole('button', { expanded: false });
    const row = toggle.parentElement!;
    expect(row).toHaveStyle({ cursor: 'pointer', touchAction: 'manipulation' });
    // 点击标签文本（非按钮）→ 展开全部
    fireEvent.click(screen.getByText('a'));
    ['e', 'f', 'g'].forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
    // 展开态：点击 chip **不**收起（标签可选中/复制）
    fireEvent.click(screen.getByText('a'));
    ['e', 'f', 'g'].forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
    // 点击「标签」折叠把手行 → 收起
    fireEvent.click(screen.getByText('标签'));
    expect(screen.queryByText('e')).not.toBeInTheDocument();
    // 按钮点击不冒泡到整行（不会双重切换）
    fireEvent.click(screen.getByRole('button', { expanded: false }));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
  });

  it('展开态「标签」把手行叠双层半透明主题色背景（更深）；chip 区域仅第一层背景', () => {
    const tags = ['a', 'b', 'c', 'd', 'e'];
    render(<AttachmentFileNameBlock {...base} tags={tags} />);
    // 收起态：无「标签」把手行、无半透明背景
    expect(screen.queryByText('标签')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { expanded: false }));

    const label = screen.getByText('标签');
    const handleRow = label.parentElement!;
    // 把手行：第二层更深的背景（叠在外层块之上 → 颜色更深）
    expect(handleRow).toHaveStyle({
      background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
      cursor: 'pointer',
    });
    // 外层块：第一层半透明主题色背景，覆盖把手行与下方 chip 区域
    const block = handleRow.parentElement!;
    expect(block).toHaveStyle({
      background: 'color-mix(in srgb, var(--accent-primary) 6%, transparent)',
      borderRadius: '8px',
    });
    // chip 区域自身不带第二层（10%）背景（chip 自身已有底色）
    const chipsRow = handleRow.nextElementSibling as HTMLElement;
    expect(chipsRow).not.toHaveStyle({
      background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
    });
  });

  it('Y001 选区守卫：有非空选区时点击标签 chip 区不切换，无选区时正常切换', () => {
    const tags = ['a', 'b', 'c', 'd', 'e'];
    render(<AttachmentFileNameBlock {...base} tags={tags} />);
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();

    const spy = vi
      .spyOn(window, 'getSelection')
      .mockReturnValue({ toString: () => 'x' } as Selection);
    try {
      // 点击标签文本（非按钮）→ 有选区不展开
      fireEvent.click(screen.getByText('a'));
      expect(screen.queryByText('e')).not.toBeInTheDocument();
      expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
    } finally {
      spy.mockRestore();
    }

    // 无选区 → 正常展开全部
    fireEvent.click(screen.getByText('a'));
    expect(screen.getByText('e')).toBeInTheDocument();
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
    // 展开后：箭头移入「标签」折叠把手行（把手行 = 折叠入口，不再位于原外层 flex）
    fireEvent.click(toggle);
    const toggleAfter = screen.getByRole('button', { expanded: true });
    expect(toggleAfter.parentElement).not.toBe(outerFlex);
    expect(screen.getByText('标签').parentElement!.contains(toggleAfter)).toBe(true);
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

describe('AttachmentFileNameBlock 名称折叠/展开', () => {
  it('短名称不显示展开箭头', () => {
    render(<AttachmentFileNameBlock {...base} fileName="photo.png" />);
    expect(screen.getByText('photo.png')).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('T006 名称折叠：超长名称（mock 溢出）显示展开箭头；展开态仅「名称」把手行可收起', () => {
    const longName = '一个非常非常长的附件文件名称'.repeat(12) + '.pdf';
    const { rerender } = render(<AttachmentFileNameBlock {...base} fileName={longName} />);
    // 初始 jsdom 无布局：无按钮
    expect(screen.queryByRole('button')).not.toBeInTheDocument();

    // 模拟真实布局：名称溢出（scrollWidth > clientWidth）
    const nameEl = screen.getByText(longName) as HTMLElement;
    Object.defineProperty(nameEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(nameEl, 'clientWidth', { configurable: true, value: 300 });

    // 以不同文件名 rerender 触发溢出检测 effect 重测 → 折叠态出现展开箭头
    rerender(<AttachmentFileNameBlock {...base} fileName={longName + '（重命名）'} />);
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();
    // 收起态整行可点（cursor pointer + touchAction manipulation）
    expect(toggle.parentElement).toHaveStyle({ cursor: 'pointer', touchAction: 'manipulation' });

    // 点击名称文本（非按钮）→ 展开
    fireEvent.click(screen.getByText(longName + '（重命名）'));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();

    // 展开态：全名换行显示 +「名称」折叠把手行（半透明主题色背景，唯一折叠入口）
    expect(screen.getByText(longName + '（重命名）')).toHaveStyle({ whiteSpace: 'pre-wrap' });
    const handleLabel = screen.getByText('名称');
    expect(handleLabel.parentElement).toHaveStyle({
      background: 'color-mix(in srgb, var(--accent-primary) 10%, transparent)',
      cursor: 'pointer',
    });

    // 展开后点击全名**不**折叠（可选中/复制）
    fireEvent.click(screen.getByText(longName + '（重命名）'));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();

    // 点击「名称」把手行 → 收起
    fireEvent.click(screen.getByText('名称'));
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
  });

  it('Y001 选区守卫：有非空选区时点击名称行不展开，无选区时正常展开', () => {
    const longName = '一个非常非常长的附件文件名称'.repeat(12) + '.pdf';
    const { rerender } = render(<AttachmentFileNameBlock {...base} fileName={longName} />);
    const nameEl = screen.getByText(longName) as HTMLElement;
    Object.defineProperty(nameEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(nameEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(<AttachmentFileNameBlock {...base} fileName={longName + '（重命名）'} />);
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();

    const spy = vi
      .spyOn(window, 'getSelection')
      .mockReturnValue({ toString: () => '选中文本' } as Selection);
    try {
      // 点击名称文本（click 照常派发到行容器）→ 有选区不展开
      fireEvent.click(screen.getByText(longName + '（重命名）'));
      expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
    } finally {
      spy.mockRestore();
    }

    // 无选区（正常点击）→ 展开
    fireEvent.click(screen.getByText(longName + '（重命名）'));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
  });

  it('回收站态名称仍可折叠展开（删除线/半透明保留）', () => {
    const longName = '待删除的极长附件文件名称'.repeat(12) + '.zip';
    const { rerender } = render(
      <AttachmentFileNameBlock {...base} fileName={longName} showTrash />,
    );
    const nameEl = screen.getByText(longName) as HTMLElement;
    Object.defineProperty(nameEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(nameEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(<AttachmentFileNameBlock {...base} fileName={longName + '（回收站）'} showTrash />);

    // 折叠态：名称删除线；整块（外层容器）半透明（与描述块同模式，含把手行/箭头）
    const collapsed = screen.getByText(longName + '（回收站）');
    expect(collapsed).toHaveStyle({ textDecoration: 'line-through' });
    expect(collapsed.parentElement!.parentElement).toHaveStyle({ opacity: '0.5' });
    fireEvent.click(screen.getByRole('button', { expanded: false }));

    // 展开态：全名保留删除线，把手行与全名同在半透明块内
    expect(screen.getByText(longName + '（回收站）')).toHaveStyle({
      textDecoration: 'line-through',
      whiteSpace: 'pre-wrap',
    });
    expect(screen.getByText('名称').parentElement!.parentElement).toHaveStyle({ opacity: '0.5' });
    // 收起
    fireEvent.click(screen.getByText('名称'));
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
  });
});
