import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
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
    const { container } = render(
      <AttachmentPreviewOverlay item={null} onClose={vi.fn()} />,
    );
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
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ vaultPath: null })}
        onClose={vi.fn()}
      />,
    );

    expect(await screen.findByText(/not stored in vault/i)).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalled();
  });

  it('shows error when vaultPath is a content URI', async () => {
    render(
      <AttachmentPreviewOverlay
        item={makeItem({ vaultPath: 'content://test/uri' })}
        onClose={vi.fn()}
      />,
    );

    expect(await screen.findByText(/not stored in vault/i)).toBeInTheDocument();
    expect(mockInvoke).not.toHaveBeenCalled();
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

    const button = await screen.findByText(/open with system app/i);
    button.click();
    expect(onOpenExternal).toHaveBeenCalled();
  });
});
