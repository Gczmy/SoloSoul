import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { AttachmentPreviewOverlay } from './AttachmentPreviewOverlay';
import type { AttachmentItem } from '@/lib/attachmentUtils';

const mockInvoke = vi.mocked(invoke);

function makeItem(overrides: Partial<AttachmentItem> = {}): AttachmentItem {
  return {
    id: 'att-1',
    objectId: 'obj-1',
    fileName: 'test.png',
    mimeType: 'image/png',
    sizeBytes: 100,
    createdAt: '2024-01-01T00:00:00Z',
    vaultPath: '/vault/attachments/obj-1/att-1/test.png',
    srcPath: 'content://test',
    ...overrides,
  };
}

describe('AttachmentPreviewOverlay', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('renders nothing when item is null', () => {
    const { container } = render(<AttachmentPreviewOverlay item={null} onClose={vi.fn()} />);
    expect(container.firstChild).toBeNull();
  });

  it('loads image preview via fs_read_file_as_data_url', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(<AttachmentPreviewOverlay item={makeItem()} onClose={vi.fn()} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_file_as_data_url', {
        path: '/vault/attachments/obj-1/att-1/test.png',
      });
    });
  });

  it('loads pdf preview via fs_read_file_as_data_url', async () => {
    mockInvoke.mockResolvedValue('data:application/pdf;base64,abc');
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ fileName: 'doc.pdf', mimeType: 'application/pdf' })}
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_file_as_data_url', {
        path: '/vault/attachments/obj-1/att-1/test.png',
      });
    });
  });

  it('loads text preview via fs_read_file_as_text', async () => {
    mockInvoke.mockResolvedValue('hello world');
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ fileName: 'note.txt', mimeType: 'text/plain' })}
        onClose={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_file_as_text', {
        path: '/vault/attachments/obj-1/att-1/test.png',
      });
    });

    expect(await screen.findByText('hello world')).toBeInTheDocument();
  });

  it('shows error when vaultPath is missing', async () => {
    render(<AttachmentPreviewOverlay item={makeItem({ vaultPath: null })} onClose={vi.fn()} />);

    expect(await screen.findByText(/common:attachment_not_in_vault/i)).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalledWith('fs_read_file_as_data_url', expect.anything());
    expect(mockInvoke).not.toHaveBeenCalledWith('fs_read_file_as_text', expect.anything());
  });

  it('shows error when vaultPath is a content URI', async () => {
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ vaultPath: 'content://test/uri' })}
        onClose={vi.fn()}
      />,
    );

    expect(await screen.findByText(/common:attachment_not_in_vault/i)).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalledWith('fs_read_file_as_data_url', expect.anything());
    expect(mockInvoke).not.toHaveBeenCalledWith('fs_read_file_as_text', expect.anything());
  });

  it('顶栏提供左上角返回按钮，点击关闭预览（与照片集查看器一致）', () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    render(<AttachmentPreviewOverlay item={makeItem()} onClose={onClose} />);

    // i18n 未初始化时 t() 返回原始 key，aria-label 为 common:back
    const backBtn = screen.getByLabelText(/common:back/i);
    expect(backBtn).toBeInTheDocument();
    backBtn.click();
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('点击预览背景仅关闭预览本身（不冒泡关闭附件卡片容器）', () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    const { container } = render(<AttachmentPreviewOverlay item={makeItem()} onClose={onClose} />);

    // 点击遮罩根元素（空白背景）：只触发一次 onClose
    fireEvent.click(container.firstChild as HTMLElement);
    expect(onClose).toHaveBeenCalledTimes(1);

    // 点击内容区滚动容器背景：经外层统一处理也只触发一次（无双重调用）
    const scrollArea = container.querySelector('[style*="overflow: auto"]');
    expect(scrollArea).toBeTruthy();
    if (scrollArea) {
      fireEvent.click(scrollArea);
      expect(onClose).toHaveBeenCalledTimes(2);
    }
  });

  it('calls onOpenExternal for unsupported types', async () => {
    const onOpenExternal = vi.fn();
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ fileName: 'archive.zip', mimeType: 'application/zip' })}
        onClose={vi.fn()}
        onOpenExternal={onOpenExternal}
      />,
    );

    const button = await screen.findByText(/common:attachment_open_system/i);
    button.click();
    expect(onOpenExternal).toHaveBeenCalled();
  });

  // ── 安卓端手势（T010）：双指捏合缩放 + 双击缩放 ──
  const touchPoints = (pts: Array<[number, number]>) =>
    pts.map(([clientX, clientY]) => ({ clientX, clientY }));

  async function renderImage() {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(<AttachmentPreviewOverlay item={makeItem()} onClose={vi.fn()} />);
    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });
    return screen.getByTestId('attachment-preview-content');
  }

  it('pinches with two fingers to zoom in and out', async () => {
    const area = await renderImage();
    expect(screen.getByText('100%')).toBeInTheDocument();

    fireEvent.touchStart(area, {
      touches: touchPoints([
        [100, 100],
        [200, 100],
      ]),
    });
    fireEvent.touchMove(area, {
      touches: touchPoints([
        [100, 100],
        [300, 100],
      ]),
    });
    expect(screen.getByText('200%')).toBeInTheDocument();
    fireEvent.touchMove(area, {
      touches: touchPoints([
        [100, 100],
        [200, 100],
      ]),
    });
    expect(screen.getByText('100%')).toBeInTheDocument();
    fireEvent.touchEnd(area, { touches: [] });
  });

  it('double-tap toggles zoom between fit and fit×2', async () => {
    const area = await renderImage();
    const tap = () => {
      fireEvent.touchStart(area, { touches: touchPoints([[150, 400]]) });
      fireEvent.touchEnd(area, { touches: [] });
    };
    tap();
    tap();
    expect(screen.getByText('200%')).toBeInTheDocument();
  });
});
