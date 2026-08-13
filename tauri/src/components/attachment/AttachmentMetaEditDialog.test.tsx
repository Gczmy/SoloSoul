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
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={onClose} />);

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

  it('失焦时输入框有内容则直接生成标签', async () => {
    const onSaved = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={vi.fn()} />);

    const tagInput = screen.getByPlaceholderText(/Type a tag/i);
    fireEvent.change(tagInput, { target: { value: '出差' } });
    // 未回车，直接 blur（点击外部）→ 应生成标签
    fireEvent.blur(tagInput);
    expect(screen.getByText('出差')).toBeInTheDocument();
    // 输入框已清空
    expect(tagInput).toHaveValue('');

    // 保存后标签已含失焦生成的条目
    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));
    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledWith({ description: '现有描述', tags: ['旅行', '出差'] });
    });
  });

  it('直接保存时输入框未回车内容也一并生成标签', async () => {
    const onSaved = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={vi.fn()} />);

    const tagInput = screen.getByPlaceholderText(/Type a tag/i);
    fireEvent.change(tagInput, { target: { value: '出差' } });
    // 不回车、不失焦，直接点保存 → 输入框内容应并入保存的标签
    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    await waitFor(() => {
      expect(onSaved).toHaveBeenCalledWith({ description: '现有描述', tags: ['旅行', '出差'] });
    });
  });

  it('修改名称时调用 attachment_rename 且 onSaved 带回 fileName', async () => {
    const onSaved = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={vi.fn()} />);

    // 名称输入框预填原文件名
    expect(screen.getByDisplayValue('photo.png')).toBeInTheDocument();
    fireEvent.change(screen.getByDisplayValue('photo.png'), {
      target: { value: 'new-photo.png' },
    });
    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('attachment_rename', {
        objectId: 'obj-1',
        attachmentId: 'att-1',
        newName: 'new-photo.png',
      });
      expect(mockInvoke).toHaveBeenCalledWith('attachment_update_meta', {
        objectId: 'obj-1',
        attachmentId: 'att-1',
        description: '现有描述',
        tags: ['旅行'],
      });
    });
    expect(onSaved).toHaveBeenCalledWith({
      fileName: 'new-photo.png',
      description: '现有描述',
      tags: ['旅行'],
    });
  });

  it('名称重命名失败时中止保存（不调用 update_meta，对话框不关闭）', async () => {
    mockInvoke.mockRejectedValueOnce(new Error('rename boom'));
    const onSaved = vi.fn();
    const onClose = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={onClose} />);

    fireEvent.change(screen.getByDisplayValue('photo.png'), {
      target: { value: 'new.png' },
    });
    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('attachment_rename', expect.anything());
    });
    // 名称失败 → 中止，不再写描述/标签，不关对话框
    expect(mockInvoke).not.toHaveBeenCalledWith('attachment_update_meta', expect.anything());
    expect(onSaved).not.toHaveBeenCalled();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('名称未改动时不触发 attachment_rename（onSaved 不带 fileName）', async () => {
    const onSaved = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={vi.fn()} />);

    // 名称保持原值，仅改描述
    fireEvent.change(screen.getByDisplayValue('现有描述'), {
      target: { value: '新描述' },
    });
    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    await waitFor(() => {
      expect(mockInvoke).not.toHaveBeenCalledWith('attachment_rename', expect.anything());
      expect(onSaved).toHaveBeenCalledWith({ description: '新描述', tags: ['旅行'] });
    });
  });

  it('名称清空时不触发重命名（防误清空），仅保存描述/标签', async () => {
    const onSaved = vi.fn();
    render(<AttachmentMetaEditDialog item={baseItem} onSaved={onSaved} onClose={vi.fn()} />);

    fireEvent.change(screen.getByDisplayValue('photo.png'), { target: { value: '   ' } });
    fireEvent.click(screen.getByRole('button', { name: /common:save/i }));

    await waitFor(() => {
      expect(mockInvoke).not.toHaveBeenCalledWith('attachment_rename', expect.anything());
    });
    expect(onSaved).toHaveBeenCalledWith({ description: '现有描述', tags: ['旅行'] });
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
