import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { TrashAttachmentsSection } from './TrashDetailSections';
import type { TrashAttachment } from './types';

const baseAtt: TrashAttachment = {
  id: 'att-1',
  fileName: 'photo.png',
  mimeType: 'image/png',
  sizeBytes: 1024,
  createdAt: '2026-07-01T00:00:00Z',
  description: '护照扫描件',
  tags: ['旅行', '证件'],
  // 文件在磁盘（后端探测通过）——预览按钮可用
  vaultPath: '/vault/attachments/obj-1/att-1/photo.png',
};

const baseProps = {
  activeAttachments: [baseAtt],
  deletedAttachments: [] as TrashAttachment[],
  expanded: true,
  showTrash: false,
  onToggle: () => {},
  onSetShowTrash: () => {},
};

describe('TrashAttachmentsSection 附件名称/描述/标签折叠展开', () => {
  it('描述与标签透传到附件行（短内容无展开箭头）', () => {
    render(<TrashAttachmentsSection {...baseProps} />);
    // 名称 + 描述 + 标签 + 大小·日期元信息
    expect(screen.getByText('photo.png')).toBeInTheDocument();
    expect(screen.getByText('护照扫描件')).toBeInTheDocument();
    expect(screen.getByText('旅行')).toBeInTheDocument();
    expect(screen.getByText('证件')).toBeInTheDocument();
    expect(screen.getByText(/1\.0 KB/)).toBeInTheDocument();
    // 短描述 + ≤4 个标签 + jsdom 无溢出：不出现无意义的展开箭头
    // （预览按钮是固定入口，不作为「展开按钮」断言目标）
    expect(screen.queryByTitle(/展开/i)).not.toBeInTheDocument();
  });

  it('标签超过 4 个默认折叠显示前 4 个 +「+N」+ 展开箭头，点击展开全部、再点收起', () => {
    const tags = ['a', 'b', 'c', 'd', 'e'];
    render(<TrashAttachmentsSection {...baseProps} activeAttachments={[{ ...baseAtt, tags }]} />);
    ['a', 'b', 'c', 'd'].forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
    expect(screen.queryByText('e')).not.toBeInTheDocument();
    expect(screen.getByText('+1')).toBeInTheDocument();
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();

    fireEvent.click(toggle);
    tags.forEach((t) => expect(screen.getByText(t)).toBeInTheDocument());
    expect(screen.queryByText('+1')).not.toBeInTheDocument();
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { expanded: true }));
    expect(screen.queryByText('e')).not.toBeInTheDocument();
    expect(screen.getByText('+1')).toBeInTheDocument();
  });

  it('附件行预览按钮：vaultPath 存在时点击回调携带该附件', () => {
    const onPreviewAttachment = vi.fn();
    render(<TrashAttachmentsSection {...baseProps} onPreviewAttachment={onPreviewAttachment} />);
    // 测试环境 useTranslation mock：object-form defaultValue 原样返回 → 'Preview'
    const btn = screen.getByRole('button', { name: /Preview/i });
    expect(btn).toBeEnabled();
    fireEvent.click(btn);
    expect(onPreviewAttachment).toHaveBeenCalledTimes(1);
    expect(onPreviewAttachment).toHaveBeenCalledWith(expect.objectContaining({ id: 'att-1' }));
  });

  it('附件行预览按钮：vaultPath 缺失（文件已删/旧数据）时禁用并提示文件不存在', () => {
    const onPreviewAttachment = vi.fn();
    render(
      <TrashAttachmentsSection
        {...baseProps}
        onPreviewAttachment={onPreviewAttachment}
        activeAttachments={[{ ...baseAtt, vaultPath: null }]}
      />,
    );
    const btn = screen.getByRole('button', {
      name: /The attachment file no longer exists/i,
    });
    expect(btn).toBeDisabled();
    fireEvent.click(btn);
    expect(onPreviewAttachment).not.toHaveBeenCalled();
  });

  it('长描述（mock 溢出）显示展开箭头，点击展开全文、再点收起', () => {
    const longDesc = '这是一段非常长的附件描述文本，'.repeat(40);
    const { rerender } = render(
      <TrashAttachmentsSection
        {...baseProps}
        activeAttachments={[{ ...baseAtt, description: longDesc }]}
      />,
    );
    // 初始 jsdom 无布局：无展开箭头按钮（预览按钮为固定入口，不算展开按钮）
    expect(screen.queryByTitle(/展开/i)).not.toBeInTheDocument();

    // 模拟真实布局：描述文本溢出（scrollWidth > clientWidth）
    const descEl = screen.getByText(longDesc) as HTMLElement;
    Object.defineProperty(descEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(descEl, 'clientWidth', { configurable: true, value: 300 });

    // 以不同描述 rerender 触发溢出检测 effect 重测 → 折叠态出现展开箭头
    rerender(
      <TrashAttachmentsSection
        {...baseProps}
        activeAttachments={[{ ...baseAtt, description: longDesc + '（续）' }]}
      />,
    );
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();

    fireEvent.click(toggle);
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
    // 展开态点击全文不折叠（可选中/复制）
    fireEvent.click(screen.getByText(longDesc + '（续）'));
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
    // 点击「描述」折叠把手行 → 收起
    fireEvent.click(screen.getByText('描述'));
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
  });

  it('回收站态（showTrash）名称保留删除线且仍可折叠展开', () => {
    // showTrash=true 时展示的是「已删除附件」桶——附件须放在 deletedAttachments 中
    const longName = '待删除的极长附件文件名称'.repeat(12) + '.zip';
    const { rerender } = render(
      <TrashAttachmentsSection
        {...baseProps}
        showTrash
        activeAttachments={[]}
        deletedAttachments={[{ ...baseAtt, fileName: longName }]}
      />,
    );
    const nameEl = screen.getByText(longName) as HTMLElement;
    expect(nameEl).toHaveStyle({ textDecoration: 'line-through' });
    Object.defineProperty(nameEl, 'scrollWidth', { configurable: true, value: 2000 });
    Object.defineProperty(nameEl, 'clientWidth', { configurable: true, value: 300 });
    rerender(
      <TrashAttachmentsSection
        {...baseProps}
        showTrash
        activeAttachments={[]}
        deletedAttachments={[{ ...baseAtt, fileName: longName + '（回收站）' }]}
      />,
    );
    const toggle = screen.getByRole('button', { expanded: false });
    expect(toggle).toBeInTheDocument();
    fireEvent.click(toggle);
    expect(screen.getByRole('button', { expanded: true })).toBeInTheDocument();
    fireEvent.click(screen.getByText('名称'));
    expect(screen.getByRole('button', { expanded: false })).toBeInTheDocument();
  });
});
