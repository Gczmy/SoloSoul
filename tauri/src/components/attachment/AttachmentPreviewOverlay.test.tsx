import { describe, it, expect, vi, beforeEach } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { AttachmentPreviewOverlay } from './AttachmentPreviewOverlay';
import type { AttachmentItem } from '@/lib/attachmentUtils';
import { isWindowsSync } from '@/lib/platform';

// 与既有测试（AttachmentRow/ObjectDetailFieldsList 等）同款：mock 平台判定。
// 默认非 Windows（macOS/Linux 走原 data URL 路径）；Windows 分支在用例内临时翻转。
vi.mock('@/lib/platform', () => ({
  isMobilePlatformSync: vi.fn(() => false),
  isWindowsSync: vi.fn(() => false),
}));

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
    // 平台 mock 复位：默认非 Windows（macOS/Linux data URL 路径）
    vi.mocked(isWindowsSync).mockReset();
    vi.mocked(isWindowsSync).mockReturnValue(false);
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

  // W-PDF：Windows（WebView2/PDFium）无法渲染 data:/blob: URL 的 embed，且
  // fs_read_file_as_data_url 有 10 MiB 上限会拒绝真实 PDF。改为经自定义协议
  // solosoul-pdf:// 直出 application/pdf（后端白名单校验），不再 invoke、不再 base64。
  it('pdf preview uses solosoul-pdf:// custom protocol on Windows', async () => {
    vi.mocked(isWindowsSync).mockReturnValue(true);
    mockInvoke.mockResolvedValue('data:application/pdf;base64,abc');
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ fileName: 'doc.pdf', mimeType: 'application/pdf' })}
        onClose={vi.fn()}
      />,
    );

    const embed = screen.getByTitle('doc.pdf');
    expect(embed.tagName.toLowerCase()).toBe('embed');
    expect(embed.getAttribute('type')).toBe('application/pdf');
    // Windows 形态：http://solosoul-pdf.localhost/<encodeURIComponent(path)>
    expect(embed.getAttribute('src')).toBe(
      `http://solosoul-pdf.localhost/${encodeURIComponent(
        '/vault/attachments/obj-1/att-1/test.png',
      )}`,
    );
    // PDF 不触发任何 invoke（不走 10 MiB 上限的 data URL 读取）
    expect(mockInvoke).not.toHaveBeenCalledWith('fs_read_file_as_data_url', expect.anything());
    expect(mockInvoke).not.toHaveBeenCalledWith('fs_read_file_as_text', expect.anything());
  });

  // 平台门控：macOS/Linux（WKWebView）不切自定义协议，保持原 data URL 路径，
  // 避免 WebKit 对自定义 scheme 的 <embed> 渲染不确定引入回归。
  it('pdf preview keeps data URL path on non-Windows desktop', async () => {
    vi.mocked(isWindowsSync).mockReturnValue(false);
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
    const embed = screen.getByTitle('doc.pdf');
    expect(embed.getAttribute('src')).toBe('data:application/pdf;base64,abc');
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

  it('disableMetaEdit 隐藏「编辑附件属性」按钮（回收站只读上下文）', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(<AttachmentPreviewOverlay item={makeItem()} onClose={vi.fn()} disableMetaEdit />);
    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_file_as_data_url', expect.anything());
    });
    expect(screen.queryByTitle(/common:edit_meta/i)).not.toBeInTheDocument();
  });

  it('disableMetaEdit 且 vaultPath 缺失时提示「文件已不存在」而非「未存储在保险库」', async () => {
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ vaultPath: null })}
        onClose={vi.fn()}
        disableMetaEdit
      />,
    );
    expect(await screen.findByText(/common:attachment_file_missing/i)).toBeInTheDocument();
    expect(screen.queryByText(/common:attachment_not_in_vault/i)).not.toBeInTheDocument();
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

  // 回归（T010 真机修复）：真实调用方（AttachmentViewer / GlobalAttachmentManager）
  // 始终挂载本组件且 previewItem 初始为 null——首帧组件 return null、手势目标元素
  // 尚未存在，若监听只在挂载时绑定一次则永不生效。验证「先空挂载 → 再给 item」
  // 后捏合/双击仍可用（useTouchZoom 需在元素延迟就绪后自动绑定）。
  it('binds gestures when mounted empty then given an item (real caller pattern)', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const { rerender } = render(<AttachmentPreviewOverlay item={null} onClose={vi.fn()} />);
    // 空挂载：无内容
    expect(screen.queryByTestId('attachment-preview-content')).not.toBeInTheDocument();

    rerender(<AttachmentPreviewOverlay item={makeItem()} onClose={vi.fn()} />);
    const area = screen.getByTestId('attachment-preview-content');
    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });

    // 双击缩放应生效（若监听未绑定，两次点按后仍为 100%）
    expect(screen.getByText('100%')).toBeInTheDocument();
    const tap = () => {
      fireEvent.touchStart(area, { touches: touchPoints([[150, 400]]) });
      fireEvent.touchEnd(area, { touches: [] });
    };
    tap();
    tap();
    expect(screen.getByText('200%')).toBeInTheDocument();
  });

  it('overrides touch-action to pan-y when image fits viewport (browser must not grab pinch)', async () => {
    const area = await renderImage();
    // jsdom 无自然尺寸 → fitsViewport 为 true → 图片模式 touch-action 应为 pan-y
    expect(area).toHaveStyle('touch-action: pan-y');
  });

  // 审查回归：关闭（item→null）再重新打开（null→item）不应重复绑定监听——
  // 每次打开手势都应恰好生效一次（若重复绑定，单次双击可能触发多次缩放）
  it('reopening after close does not double-bind gestures', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const { rerender } = render(<AttachmentPreviewOverlay item={null} onClose={vi.fn()} />);

    // 打开 → 关闭 → 再打开（模拟真机反复预览不同附件）
    rerender(<AttachmentPreviewOverlay item={makeItem()} onClose={vi.fn()} />);
    let area = screen.getByTestId('attachment-preview-content');
    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });
    rerender(<AttachmentPreviewOverlay item={null} onClose={vi.fn()} />);
    expect(screen.queryByTestId('attachment-preview-content')).not.toBeInTheDocument();
    rerender(<AttachmentPreviewOverlay item={makeItem({ id: 'att-2' })} onClose={vi.fn()} />);
    area = screen.getByTestId('attachment-preview-content');
    await waitFor(() => {
      expect(screen.getByTitle(/common:attachment_zoom_in/i)).toBeInTheDocument();
    });

    // 双击：若监听重复绑定，一次双击会触发多次切换 → 比例跳到 400%/800% 而非 200%
    expect(screen.getByText('100%')).toBeInTheDocument();
    const tap = () => {
      fireEvent.touchStart(area, { touches: touchPoints([[150, 400]]) });
      fireEvent.touchEnd(area, { touches: [] });
    };
    tap();
    tap();
    expect(screen.getByText('200%')).toBeInTheDocument();
    expect(screen.queryByText('400%')).not.toBeInTheDocument();
  });
});
