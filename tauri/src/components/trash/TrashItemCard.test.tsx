import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TrashItemCard } from './TrashItemCard';
import type { TrashItemSummary } from '@/stores/trashStore';

const baseItem: TrashItemSummary = {
  id: 'trash-1',
  itemType: 'object',
  originalId: 'obj-1',
  name: 'Old Object',
  deletedAt: Date.now() - 3600_000,
  originalSectionType: 'identity',
};

const baseProps = {
  item: baseItem,
  isSelected: false,
  onOpenDetail: vi.fn(),
  onRestore: vi.fn(),
  onDelete: vi.fn(),
  onToggle: vi.fn(),
};

describe('TrashItemCard', () => {
  it('renders the item name and type', () => {
    render(<TrashItemCard {...baseProps} />);
    expect(screen.getByText('Old Object')).toBeInTheDocument();
  });

  it('P119: passes trashId to stable callbacks', () => {
    const onOpenDetail = vi.fn();
    const onRestore = vi.fn();
    const onDelete = vi.fn();
    const onToggle = vi.fn();
    render(
      <TrashItemCard
        item={baseItem}
        isSelected={false}
        onOpenDetail={onOpenDetail}
        onRestore={onRestore}
        onDelete={onDelete}
        onToggle={onToggle}
      />,
    );

    // 点击卡片主体 → onOpenDetail(item.id)
    fireEvent.click(screen.getByText('Old Object'));
    expect(onOpenDetail).toHaveBeenCalledWith('trash-1');

    // 还原按钮 → onRestore(item.id)（setup 的 i18n mock 返回翻译 key）
    fireEvent.click(screen.getAllByTitle('common:restore')[0]);
    expect(onRestore).toHaveBeenCalledWith('trash-1');

    // 永久删除按钮 → onDelete(item.id)
    fireEvent.click(screen.getAllByTitle('common:delete_permanently')[0]);
    expect(onDelete).toHaveBeenCalledWith('trash-1');
  });

  it('P119: renders checked state from isSelected', () => {
    const { rerender } = render(<TrashItemCard {...baseProps} isSelected={true} />);
    expect(screen.getAllByRole('checkbox')[0]).toBeChecked();

    rerender(<TrashItemCard {...baseProps} isSelected={false} />);
    expect(screen.getAllByRole('checkbox')[0]).not.toBeChecked();
  });
});
