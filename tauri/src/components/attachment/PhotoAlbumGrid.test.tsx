import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor, fireEvent, within } from '@testing-library/react';
import { invoke } from '@tauri-apps/api/core';
import { PhotoAlbumGrid } from './PhotoAlbumGrid';
import { THUMB_MAX_DIM } from '@/lib/photoAlbumPreview';
import type { AttachmentItem } from '@/lib/attachmentUtils';

const mockInvoke = vi.mocked(invoke);

function makeItem(id: string, overrides: Partial<AttachmentItem> = {}): AttachmentItem {
  return {
    id,
    objectId: 'obj-1',
    fileName: `${id}.png`,
    mimeType: 'image/png',
    sizeBytes: 100,
    createdAt: '2024-01-01T00:00:00Z',
    vaultPath: `/vault/attachments/obj-1/${id}.png`,
    srcPath: null,
    ...overrides,
  };
}

describe('PhotoAlbumGrid', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
  });

  it('loads thumbnails via fs_read_image_preview with maxDim 256', async () => {
    mockInvoke.mockResolvedValue('data:image/jpeg;base64,abc');
    render(<PhotoAlbumGrid items={[makeItem('a'), makeItem('b')]} onSelect={vi.fn()} />);

    await waitFor(() => {
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_image_preview', {
        path: '/vault/attachments/obj-1/a.png',
        maxDim: THUMB_MAX_DIM,
      });
      expect(mockInvoke).toHaveBeenCalledWith('fs_read_image_preview', {
        path: '/vault/attachments/obj-1/b.png',
        maxDim: THUMB_MAX_DIM,
      });
    });
  });

  it('calls onSelect with item and index on cell click', async () => {
    mockInvoke.mockResolvedValue('data:image/jpeg;base64,abc');
    const onSelect = vi.fn();
    render(<PhotoAlbumGrid items={[makeItem('c'), makeItem('d')]} onSelect={onSelect} />);

    const cells = within(await screen.findByTestId('photo-album-grid')).getAllByRole('button');
    fireEvent.click(cells[1]);
    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ id: 'd' }), 1);
  });

  it('shows failure placeholder when thumbnail load fails', async () => {
    mockInvoke.mockRejectedValue(new Error('decode failed'));
    render(<PhotoAlbumGrid items={[makeItem('e')]} onSelect={vi.fn()} />);

    expect(await screen.findByText(/common:photo_thumbnail_failed/i)).toBeInTheDocument();
  });

  it('does not invoke IPC when vaultPath is missing', async () => {
    render(<PhotoAlbumGrid items={[makeItem('f', { vaultPath: null })]} onSelect={vi.fn()} />);

    expect(await screen.findByText(/common:photo_thumbnail_failed/i)).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalled();
  });
});
