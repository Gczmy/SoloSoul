import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { SampleTemplateGallery } from './SampleTemplateGallery';
import { SAMPLE_TEMPLATES_EN, SAMPLE_TEMPLATES_ZH } from '@/lib/sampleTemplates';

describe('SampleTemplateGallery', () => {
  // 筛选后卡片仍在 DOM 中，但被 visibility:hidden + data-visible="false" 屏蔽。
  // 关键产品行为是“可见卡”的数量，所以这里提供独立 helper 统计可见项。
  const getVisibleCards = () =>
    screen.getAllByTestId('sample-template-card').filter(
      (el) => el.getAttribute('data-visible') !== 'false',
    );

  it('does not render when closed', () => {
    const { container } = render(
      <SampleTemplateGallery isOpen={false} onClose={vi.fn()} onSelect={vi.fn()} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it('renders the English sample template cards by default', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    expect(getVisibleCards()).toHaveLength(SAMPLE_TEMPLATES_EN.length);
    SAMPLE_TEMPLATES_EN.forEach((tpl) => {
      expect(screen.getByText(tpl.name)).toBeInTheDocument();
    });
  });

  it('calls onSelect with the chosen template', () => {
    const onSelect = vi.fn();
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={onSelect} />);

    const cards = getVisibleCards();
    fireEvent.click(cards[0]);
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect).toHaveBeenCalledWith(SAMPLE_TEMPLATES_EN[0]);
  });

  it('calls onClose when clicking the close button', () => {
    const onClose = vi.fn();
    render(<SampleTemplateGallery isOpen={true} onClose={onClose} onSelect={vi.fn()} />);

    fireEvent.click(screen.getByTestId('sample-gallery-close'));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when clicking the overlay', () => {
    const onClose = vi.fn();
    const { container } = render(
      <SampleTemplateGallery isOpen={true} onClose={onClose} onSelect={vi.fn()} />,
    );

    const overlay = container.firstChild as HTMLElement;
    fireEvent.click(overlay);
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('filters sample templates by page category', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const travelCount = SAMPLE_TEMPLATES_EN.filter((t) => t.category === 'travel').length;
    fireEvent.click(screen.getByTestId('page-filter-travel'));

    // 为了让弹卡高度稳定，即使过滤后仍渲染所有卡，只是隐藏不匹配的项。
    // DOM 卡总数保持不变；可见卡收敛到目标分类。
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(SAMPLE_TEMPLATES_EN.length);
    expect(getVisibleCards()).toHaveLength(travelCount);
  });

  it('filters sample templates by search query', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: SAMPLE_TEMPLATES_EN[0].name } });

    // DOM 不变，仅可见卡收敛为命中项。
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(SAMPLE_TEMPLATES_EN.length);
    expect(getVisibleCards()).toHaveLength(1);
    expect(screen.getByText(SAMPLE_TEMPLATES_EN[0].name)).toBeInTheDocument();
  });

  it('switches to the Chinese tab and resets filters', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    // Apply a category filter first.
    fireEvent.click(screen.getByTestId('page-filter-travel'));
    const travelCount = SAMPLE_TEMPLATES_EN.filter((t) => t.category === 'travel').length;
    expect(getVisibleCards()).toHaveLength(travelCount);

    // Switch locale tab.
    fireEvent.click(screen.getByTestId('locale-tab-zh'));

    // Should show all Chinese templates with filter reset.
    expect(getVisibleCards()).toHaveLength(SAMPLE_TEMPLATES_ZH.length);
    SAMPLE_TEMPLATES_ZH.forEach((tpl) => {
      expect(screen.getByText(tpl.name)).toBeInTheDocument();
    });
  });

  it('resets search query when switching locale tabs', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: SAMPLE_TEMPLATES_EN[0].name } });
    expect(getVisibleCards()).toHaveLength(1);

    fireEvent.click(screen.getByTestId('locale-tab-zh'));
    expect(getVisibleCards()).toHaveLength(SAMPLE_TEMPLATES_ZH.length);
    expect(input).toHaveValue('');
  });

  it('places visible cards first via CSS order so empty slots do not appear at the top', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    // 切到“仅财务”：可见卡收敛到顶上，不可见卡被 CSS order 推到后面。
    fireEvent.click(screen.getByTestId('page-filter-financial'));
    const financialCount = SAMPLE_TEMPLATES_EN.filter((t) => t.category === 'financial').length;
    expect(financialCount).toBeGreaterThan(0);

    const allCards = screen.getAllByTestId('sample-template-card');
    const visibleCards = allCards.filter((c) => c.getAttribute('data-visible') !== 'false');
    const hiddenCards = allCards.filter((c) => c.getAttribute('data-visible') === 'false');

    expect(visibleCards.length).toBe(financialCount);
    expect(hiddenCards.length).toBe(SAMPLE_TEMPLATES_EN.length - financialCount);

    // 关键：可见卡片都设了 order:-1，隐藏卡片都是 order:0。
    visibleCards.forEach((c) => expect(c.style.order).toBe('-1'));
    hiddenCards.forEach((c) => expect(c.style.order).toBe('0'));
  });

  it('shows the empty-state overlay when nothing matches the search query', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const input = screen.getByRole('textbox');
    fireEvent.change(input, { target: { value: 'zzz-no-match-token-zzz' } });

    expect(getVisibleCards()).toHaveLength(0);
    expect(screen.getAllByTestId('sample-template-card')).toHaveLength(SAMPLE_TEMPLATES_EN.length);
    expect(screen.getByRole('status')).toBeInTheDocument();
  });

  it('keeps card size stable across filter changes (DOM always renders full template list)', () => {
    render(<SampleTemplateGallery isOpen={true} onClose={vi.fn()} onSelect={vi.fn()} />);

    const fullDomCount = screen.getAllByTestId('sample-template-card').length;

    // 在“全部”状态下 DOM 与可见数量一致。
    expect(getVisibleCards()).toHaveLength(SAMPLE_TEMPLATES_EN.length);

    // 切换到仅财务：DOM 仍是全量，但可见数收敛。
    fireEvent.click(screen.getByTestId('page-filter-financial'));
    const financialCount = SAMPLE_TEMPLATES_EN.filter((t) => t.category === 'financial').length;
    expect(screen.getAllByTestId('sample-template-card').length).toBe(fullDomCount);
    expect(getVisibleCards()).toHaveLength(financialCount);

    // 再切回身份：DOM 仍为全量，可见卡跟随新的过滤。
    fireEvent.click(screen.getByTestId('page-filter-identity'));
    const identityCount = SAMPLE_TEMPLATES_EN.filter((t) => t.category === 'identity').length;
    expect(screen.getAllByTestId('sample-template-card').length).toBe(fullDomCount);
    expect(getVisibleCards()).toHaveLength(identityCount);

    // 回到“全部”后可见恢复全量。
    fireEvent.click(screen.getByTestId('page-filter-all'));
    expect(getVisibleCards()).toHaveLength(SAMPLE_TEMPLATES_EN.length);
  });
});
