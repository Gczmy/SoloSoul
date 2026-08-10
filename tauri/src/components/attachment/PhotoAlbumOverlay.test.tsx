import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { PhotoAlbumOverlay } from './PhotoAlbumOverlay';
import type { AttachmentItem } from '@/lib/attachmentUtils';

const mockInvoke = vi.mocked(invoke);

function makeItem(id: string): AttachmentItem {
  return {
    id,
    objectId: 'obj-1',
    fileName: `${id}.png`,
    mimeType: 'image/png',
    sizeBytes: 100,
    createdAt: '2024-01-01T00:00:00Z',
    vaultPath: `/vault/attachments/obj-1/${id}.png`,
    srcPath: null,
  };
}

describe('PhotoAlbumOverlay', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('opens the viewer on cell click and returns to grid on back', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(<PhotoAlbumOverlay items={[makeItem('a'), makeItem('b')]} onClose={vi.fn()} />);

    const grid = await screen.findByTestId('photo-album-grid');
    const cells = within(grid).getAllByRole('button');
    fireEvent.click(cells[0]);

    await waitFor(() => {
      expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
    });

    fireEvent.click(screen.getByRole('button', { name: /common:back_to_album/i }));
    await waitFor(() => {
      expect(screen.queryByTestId('photo-viewer-counter')).not.toBeInTheDocument();
    });
  });

  it('closes via header close button', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    const onClose = vi.fn();
    render(<PhotoAlbumOverlay items={[makeItem('a')]} onClose={onClose} />);

    // 等待缩略图异步加载完成，避免测试结束时状态更新产生 act() 警告
    await screen.findByAltText('a.png');
    fireEvent.click(screen.getByRole('button', { name: /common:close/i }));
    expect(onClose).toHaveBeenCalled();
  });
});
