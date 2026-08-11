import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { AttachmentMetaEditDialog } from './AttachmentMetaEditDialog';

const mockInvoke = vi.mocked(invoke);

const baseItem = {
  objectId: 'obj-1',
  id: 'att-1',
  fileName: 'photo.png',
  description: '现有描述',
  tags: ['旅行'],
};

describe('AttachmentMetaEditDialog', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    mockInvoke.mockResolvedValue(undefined);
  });

  it('预填描述与标签，添加标签后保存调用 attachment_update_meta', async () => {
    const onSaved = vi.fn();
    const onClose = vi.fn();
    render(
      <AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={onClose} />,
    );

    // 预填
    expect(screen.getByDisplayValue('现有描述')).toBeInTheDocument();
    expect(screen.getByText('旅行')).toBeInTheDocument();

    // 输入新标签并回车（单测 i18n 下 placeholder 为 defaultValue）
    const tagInput = screen.getByPlaceholderText(/Type a tag/i);
    fireEvent.change(tagInput, { target: { value: '出差' } });
    fireEvent.keyDown(tagInput, { key: 'Enter' });
    expect(screen.getByText('出差')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('attachment_update_meta', {
        objectId: 'obj-1',
        attachmentId: 'att-1',
        description: '现有描述',
        tags: ['旅行', '出差'],
      });
    });
    expect(onSaved).toHaveBeenCalledWith({ description: '现有描述', tags: ['旅行', '出差'] });
    expect(onClose).toHaveBeenCalled();
  });

  it('空描述保存为 null（清除），重复标签去重', async () => {
    const onSaved = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={vi.fn()} />);

    fireEvent.change(screen.getByDisplayValue('现有描述'), { target: { value: '   ' } });
    const tagInput = screen.getByPlaceholderText(/Type a tag/i);
    // 重复标签 + 空白输入均不新增
    fireEvent.change(tagInput, { target: { value: '旅行' } });
    fireEvent.keyDown(tagInput, { key: 'Enter' });
    fireEvent.change(tagInput, { target: { value: '   ' } });
    fireEvent.keyDown(tagInput, { key: 'Enter' });

    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledWith({ description: null, tags: ['旅行'] });
    });
  });

  it('X 按钮移除标签', () => {
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={vi.fn()} onClose={vi.fn()} />);

    fireEvent.click(screen.getByRole('button', { name: /Remove tag/i }));
    expect(screen.queryByText('旅行')).not.toBeInTheDocument();
  });

  it('保存失败时不关闭对话框', async () => {
    mockInvoke.mockRejectedValue(new Error('boom'));
    const onClose = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={vi.fn()} onClose={onClose} />);

    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    // 等待 IPC 调用结算（错误 toast 由全局 ToastContainer 渲染，单测中不挂载）
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalled();
    });
    expect(onClose).not.toHaveBeenCalled();
  });
});
