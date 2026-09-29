import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { GuideCategoryMeta, GuideIndexEntry } from '@/lib/guideApi';
import { GuideIndex } from './GuideIndex';

const categories: GuideCategoryMeta[] = [
  { id: 'advanced', title: { zh: '进阶', en: 'Advanced' }, order: 2 },
  { id: 'basics', title: { zh: '基础', en: 'Basics' }, order: 1 },
  { id: 'empty', title: { zh: '空分类', en: 'Empty' }, order: 3 },
];

const guides: GuideIndexEntry[] = [
  {
    id: 'backup',
    title: { zh: '备份', en: 'Backup' },
    category: 'basics',
    order: 2,
    keywords: [],
    files: {},
  },
  {
    id: 'login',
    title: { zh: '登录', en: 'Login' },
    category: 'basics',
    order: 1,
    keywords: [],
    files: {},
  },
  {
    id: 'sync',
    title: { zh: '同步', en: 'Sync' },
    category: 'advanced',
    order: 1,
    keywords: [],
    files: {},
  },
];

describe('GuideIndex', () => {
  it('空索引按当前语言提示无文档', () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <GuideIndex guides={[]} categories={[]} language="zh-CN" onSelect={onSelect} />,
    );
    expect(screen.getByText('暂无帮助文档')).toBeInTheDocument();

    rerender(<GuideIndex guides={[]} categories={[]} language="en-US" onSelect={onSelect} />);
    expect(screen.getByText('No guides available')).toBeInTheDocument();
    expect(onSelect).not.toHaveBeenCalled();
  });

  it('分类和文档按顺序显示，鼠标与键盘均打开正确文档', () => {
    const onSelect = vi.fn();
    const { rerender } = render(
      <GuideIndex guides={guides} categories={categories} language="zh-CN" onSelect={onSelect} />,
    );
    expect(
      screen.getAllByRole('heading', { level: 3 }).map((heading) => heading.textContent),
    ).toEqual(['基础', '进阶']);
    expect(
      screen.getAllByRole('button').map((button) => button.textContent?.replace('›', '')),
    ).toEqual(['登录', '备份', '同步']);
    expect(screen.queryByText('空分类')).not.toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /登录/ }));
    fireEvent.keyDown(screen.getByRole('button', { name: /同步/ }), { key: 'Enter' });
    expect(onSelect).toHaveBeenNthCalledWith(1, 'login');
    expect(onSelect).toHaveBeenNthCalledWith(2, 'sync');

    rerender(
      <GuideIndex guides={guides} categories={categories} language="en-US" onSelect={onSelect} />,
    );
    expect(screen.getByRole('heading', { name: 'Basics' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /Backup/ })).toBeInTheDocument();
  });

  it('无文档但有附加入口的分类仍显示，其他空分类保持隐藏', () => {
    render(
      <GuideIndex
        guides={[]}
        categories={categories}
        language="zh-CN"
        onSelect={vi.fn()}
        extraItems={{ empty: <button>创建指南</button> }}
      />,
    );

    expect(screen.getByRole('heading', { name: '空分类' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: '创建指南' })).toBeInTheDocument();
    expect(screen.queryByText('基础')).not.toBeInTheDocument();
    expect(screen.queryByText('进阶')).not.toBeInTheDocument();
    expect(screen.queryByText('暂无帮助文档')).not.toBeInTheDocument();
  });
});
