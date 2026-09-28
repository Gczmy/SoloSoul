import {
  act,
  cleanup,
  fireEvent,
  render,
  renderHook,
  screen,
  waitFor,
  within,
} from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { invoke } from '@tauri-apps/api/core';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { useUiStore } from '@/stores/uiStore';
import { AttachmentViewer } from './AttachmentViewer';
import { useAttachmentViewer } from './useAttachmentViewer';
// 与 PhotoAlbumOverlay 现有回归一致：预载冷编译依赖，保留真实 lazy 查看器。
import '@/components/attachment/PhotoViewerOverlay';

// 保留父组件、Hook、嵌套浮层、编辑器和 IPC 封装，仅隔离原生拖拽订阅。
vi.mock('@/hooks/useDragToAttach', () => ({
  useDragToAttach: () => ({
    ref: { current: null },
    dragState: {
      isDraggingOver: false,
      isUploading: false,
      currentIndex: 0,
      totalFiles: 0,
      currentFileName: '',
      pendingFiles: 0,
    },
  }),
}));

const mockInvoke = vi.mocked(invoke);
const objectId = 'object-a';
const original: AttachmentItem = {
  id: 'photo-a',
  objectId,
  fileName: 'original.png',
  description: 'Original description',
  tags: ['original-tag'],
  mimeType: 'image/png',
  sizeBytes: 100,
  createdAt: '2026-09-28T00:00:00Z',
  vaultPath: '/synthetic-vault/photo-a.png',
};
const untouched: AttachmentItem = {
  ...original,
  id: 'photo-b',
  fileName: 'untouched.png',
  description: 'Unchanged description',
  tags: ['untouched-tag'],
  vaultPath: '/synthetic-vault/photo-b.png',
};
let rejectMetadataSave = false;

async function renderLoadedViewer() {
  const onClose = vi.fn();
  render(<AttachmentViewer objectId={objectId} onClose={onClose} />);
  const checkbox = await screen.findByRole('checkbox', { name: original.fileName });
  // 附件面板与浮层都保留真实 DOM；限定父面板以免把相册本地副本误当作父列表。
  const panel = checkbox.closest('[data-macos-glass="panel"]');
  if (!(panel instanceof HTMLElement)) throw new Error('Attachment list panel is missing');
  return { panel, onClose };
}

async function openPreview(panel: HTMLElement) {
  fireEvent.click(within(panel).getAllByRole('button', { name: 'common:preview' })[0]);
  return screen.findByTestId('attachment-preview-overlay');
}

async function openAlbumPhoto(panel: HTMLElement, fileName: string) {
  fireEvent.click(within(panel).getByRole('button', { name: /common:photo_album/ }));
  const grid = await screen.findByTestId('photo-album-grid');
  fireEvent.click(within(grid).getByRole('button', { name: fileName }));
  return screen.findByTestId('photo-viewer', {}, { timeout: 8000 });
}

function openOverlayEditor(overlay: HTMLElement) {
  fireEvent.click(within(overlay).getByRole('button', { name: 'common:edit_meta' }));
  return screen.getByRole('dialog');
}

function openRowEditor(panel: HTMLElement) {
  fireEvent.click(within(panel).getAllByRole('button', { name: 'Edit Attachment Attributes' })[0]);
  return screen.getByRole('dialog');
}

function changeAllMetadata(dialog: HTMLElement, name: string, description: string, tag: string) {
  const editor = within(dialog);
  fireEvent.change(editor.getByRole('textbox', { name: 'Name' }), { target: { value: name } });
  fireEvent.change(editor.getByPlaceholderText('Add a description…'), {
    target: { value: description },
  });
  fireEvent.change(editor.getByRole('textbox', { name: 'Tags' }), { target: { value: tag } });
  fireEvent.keyDown(editor.getByRole('textbox', { name: 'Tags' }), { key: 'Enter' });
}

async function saveEditor(dialog: HTMLElement) {
  fireEvent.click(within(dialog).getByRole('button', { name: 'common:save' }));
  await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
}

function expectEditorMetadata(name: string, description: string, tags: string[]) {
  const editor = within(screen.getByRole('dialog'));
  expect(editor.getByRole('textbox', { name: 'Name' })).toHaveValue(name);
  expect(editor.getByPlaceholderText('Add a description…')).toHaveValue(description);
  expect(editor.queryAllByRole('button', { name: 'Remove tag' })).toHaveLength(tags.length);
  for (const tag of tags) expect(editor.getByText(tag)).toBeInTheDocument();
}

function expectUntouchedAttachment(panel: HTMLElement) {
  const list = within(panel);
  expect(list.getByRole('checkbox', { name: untouched.fileName })).toBeInTheDocument();
  expect(list.getByText(untouched.description!)).toBeInTheDocument();
  expect(list.getByText('untouched-tag')).toBeInTheDocument();
}

function expectNoListReload() {
  expect(mockInvoke.mock.calls.filter(([command]) => command === 'attachment_list')).toHaveLength(
    2,
  );
}

describe('RF-911 attachment metadata save wiring', () => {
  beforeEach(() => {
    rejectMetadataSave = false;
    mockInvoke.mockReset();
    mockInvoke.mockImplementation((command, args) => {
      switch (command) {
        case 'attachment_list': {
          const request = args as { objectId: string; showDeleted: boolean };
          expect(request.objectId).toBe(objectId);
          return Promise.resolve(request.showDeleted ? [] : [original, untouched]);
        }
        case 'attachment_update_meta':
          return rejectMetadataSave
            ? Promise.reject(new Error('metadata write failed'))
            : Promise.resolve(undefined);
        case 'attachment_rename':
        case 'set_status_bar_style':
          return Promise.resolve(undefined);
        case 'fs_read_file_as_data_url':
        case 'fs_read_image_preview':
          return Promise.resolve('data:image/png;base64,AA==');
        default:
          throw new Error(`Unexpected IPC: ${command}`);
      }
    });
  });

  afterEach(() => {
    cleanup();
    for (const toast of useUiStore.getState().toasts) useUiStore.getState().dismissToast(toast.id);
  });

  it('预览保存三项元数据后同步父列表，关闭再开仍显示新值', async () => {
    const { panel, onClose } = await renderLoadedViewer();
    const preview = await openPreview(panel);
    const editor = openOverlayEditor(preview);
    changeAllMetadata(editor, 'preview-edited.png', 'Preview description', 'preview-tag');
    await saveEditor(editor);

    expect(mockInvoke).toHaveBeenCalledWith('attachment_rename', {
      objectId,
      attachmentId: original.id,
      newName: 'preview-edited.png',
    });
    expect(mockInvoke).toHaveBeenCalledWith('attachment_update_meta', {
      objectId,
      attachmentId: original.id,
      description: 'Preview description',
      tags: ['original-tag', 'preview-tag'],
    });
    expect(within(preview).getByText('preview-edited.png')).toBeInTheDocument();
    expect(within(panel).getByRole('checkbox', { name: 'preview-edited.png' })).toBeInTheDocument();
    expect(within(panel).getByText('Preview description')).toBeInTheDocument();
    expect(within(panel).getByText('preview-tag')).toBeInTheDocument();
    expectUntouchedAttachment(panel);

    fireEvent.click(within(preview).getByRole('button', { name: 'common:back' }));
    expect(screen.queryByTestId('attachment-preview-overlay')).not.toBeInTheDocument();
    openOverlayEditor(await openPreview(panel));
    expectEditorMetadata('preview-edited.png', 'Preview description', [
      'original-tag',
      'preview-tag',
    ]);
    expectNoListReload();
    expect(onClose).not.toHaveBeenCalled();
  });

  it('照片集保存后不被父级旧数据覆盖，关闭照片集再开仍保留修改', async () => {
    const { panel, onClose } = await renderLoadedViewer();
    const viewer = await openAlbumPhoto(panel, original.fileName);
    const editor = openOverlayEditor(viewer);
    changeAllMetadata(editor, 'album-edited.png', 'Album description', 'album-tag');
    await saveEditor(editor);

    expect(within(viewer).getByText('album-edited.png')).toBeInTheDocument();
    expect(within(panel).getByRole('checkbox', { name: 'album-edited.png' })).toBeInTheDocument();
    expect(within(panel).getByText('Album description')).toBeInTheDocument();
    expect(within(panel).getByText('album-tag')).toBeInTheDocument();
    expectUntouchedAttachment(panel);
    expect(mockInvoke).toHaveBeenCalledWith('attachment_update_meta', {
      objectId,
      attachmentId: original.id,
      description: 'Album description',
      tags: ['original-tag', 'album-tag'],
    });

    fireEvent.click(within(viewer).getByRole('button', { name: 'common:back_to_album' }));
    expect(screen.queryByTestId('photo-viewer')).not.toBeInTheDocument();
    const album = screen.getByTestId('photo-album-overlay');
    fireEvent.click(within(album).getByRole('button', { name: 'common:close' }));
    expect(screen.queryByTestId('photo-album-overlay')).not.toBeInTheDocument();
    openOverlayEditor(await openAlbumPhoto(panel, 'album-edited.png'));
    expectEditorMetadata('album-edited.png', 'Album description', ['original-tag', 'album-tag']);
    expectNoListReload();
    expect(onClose).not.toHaveBeenCalled();
  }, 12000);

  it('行编辑清空描述和标签时保留名称与其他附件，同一查看器内重开为空', async () => {
    const { panel } = await renderLoadedViewer();
    const editor = openRowEditor(panel);
    fireEvent.change(within(editor).getByPlaceholderText('Add a description…'), {
      target: { value: '' },
    });
    fireEvent.click(within(editor).getByRole('button', { name: 'Remove tag' }));
    await saveEditor(editor);

    expect(mockInvoke).not.toHaveBeenCalledWith('attachment_rename', expect.anything());
    expect(mockInvoke).toHaveBeenCalledWith('attachment_update_meta', {
      objectId,
      attachmentId: original.id,
      description: '',
      tags: [],
    });
    expect(within(panel).getByRole('checkbox', { name: original.fileName })).toBeInTheDocument();
    expect(within(panel).queryByText(original.description!)).not.toBeInTheDocument();
    expect(within(panel).queryByText('original-tag')).not.toBeInTheDocument();
    expectUntouchedAttachment(panel);
    openRowEditor(panel);
    expectEditorMetadata(original.fileName, '', []);
    // 验证空串传参与本地空值回写；IPC mock 不代替真实 Vault 持久化验证。
    expectNoListReload();
  });

  it('预览保存失败不回写父列表，编辑器保留草稿并允许取消', async () => {
    rejectMetadataSave = true;
    const { panel, onClose } = await renderLoadedViewer();
    const preview = await openPreview(panel);
    const editor = openOverlayEditor(preview);
    fireEvent.change(within(editor).getByPlaceholderText('Add a description…'), {
      target: { value: 'Unsaved description' },
    });
    fireEvent.click(within(editor).getByRole('button', { name: 'common:save' }));

    await waitFor(() => {
      expect(useUiStore.getState().toasts.map((toast) => toast.type)).toEqual(['error']);
    });
    expect(mockInvoke).toHaveBeenCalledWith('attachment_update_meta', {
      objectId,
      attachmentId: original.id,
      description: 'Unsaved description',
      tags: ['original-tag'],
    });
    expect(mockInvoke).not.toHaveBeenCalledWith('attachment_rename', expect.anything());
    expectEditorMetadata(original.fileName, 'Unsaved description', ['original-tag']);
    expect(within(panel).getByText(original.description!)).toBeInTheDocument();
    expect(within(panel).queryByText('Unsaved description')).not.toBeInTheDocument();
    expect(within(preview).getByText(original.fileName)).toBeInTheDocument();
    expectUntouchedAttachment(panel);

    fireEvent.click(within(editor).getByRole('button', { name: 'common:cancel' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    openOverlayEditor(preview);
    expectEditorMetadata(original.fileName, original.description!, ['original-tag']);
    expectNoListReload();
    expect(onClose).not.toHaveBeenCalled();
  });

  // 以下两项验证新显式身份/部分补丁契约，不计入修复前四项UI红色基线。
  it('部分元数据补丁保留未提供的字段、原附件路径和其他附件', async () => {
    const { result } = renderHook(() => useAttachmentViewer({ objectId, onClose: vi.fn() }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => result.current.setPreviewItem(original));
    act(() => {
      result.current.handleMetaSaved(original, {
        description: 'Only the description changed',
        fileName: undefined,
        tags: undefined,
      });
    });
    const expected = { ...original, description: 'Only the description changed' };
    expect(result.current.items).toEqual([expected, untouched]);
    expect(result.current.previewItem).toEqual(expected);
    expect(result.current.items[1]).toBe(untouched);
    expect(result.current.metaEditItem).toBeNull();
    expect(original.description).toBe('Original description');
    expectNoListReload();
  });

  it('相同附件ID按对象身份分开更新，回收站和预览只同步各自目标', async () => {
    const otherObjectItem: AttachmentItem = {
      ...original,
      objectId: 'object-b',
      fileName: 'other-object.png',
      description: 'Other object description',
      deletedAt: '2026-09-28T01:00:00Z',
      vaultPath: '/synthetic-vault/object-b/photo-a.png',
    };
    mockInvoke.mockImplementation((command, args) => {
      if (command !== 'attachment_list') throw new Error(`Unexpected IPC: ${command}`);
      const request = args as { showDeleted: boolean };
      return Promise.resolve(request.showDeleted ? [otherObjectItem] : [original, untouched]);
    });
    const { result } = renderHook(() => useAttachmentViewer({ objectId, onClose: vi.fn() }));
    await waitFor(() => expect(result.current.loading).toBe(false));
    act(() => result.current.setPreviewItem(otherObjectItem));
    act(() => result.current.handleMetaSaved(original, { fileName: 'renamed-active.png' }));
    const activeExpected = { ...original, fileName: 'renamed-active.png' };
    expect(result.current.items).toEqual([activeExpected, untouched]);
    expect(result.current.trashItems[0]).toBe(otherObjectItem);
    expect(result.current.previewItem).toBe(otherObjectItem);

    act(() =>
      result.current.handleMetaSaved(otherObjectItem, { description: 'Updated trash item' }),
    );
    const trashExpected = { ...otherObjectItem, description: 'Updated trash item' };
    expect(result.current.trashItems).toEqual([trashExpected]);
    expect(result.current.previewItem).toEqual(trashExpected);
    expect(result.current.items).toEqual([activeExpected, untouched]);
    expectNoListReload();
  });
});
