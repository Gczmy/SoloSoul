import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { isMobilePlatformSync } from '@/lib/platform';
import { AttachmentPageCard } from './AttachmentPageCard';
import type { AttachmentMeta, AttachmentTreePage } from './attachmentManagerTypes';

vi.mock('@/lib/platform', () => ({
  isMobilePlatformSync: vi.fn(() => false),
  isAndroidSync: vi.fn(() => false),
}));

const attachment: AttachmentMeta = {
  id: 'att1',
  objectId: 'obj1',
  fileName: 'report.pdf',
  mimeType: 'application/pdf',
  sizeBytes: 1024,
  createdAt: '2026-07-01T00:00:00Z',
};

const page: AttachmentTreePage = {
  pageId: 'page1',
  pageName: 'Documents',
  objects: [{ objectId: 'obj1', objectName: 'Record', attachments: [attachment] }],
};

describe('AttachmentPageCard', () => {
  it('页面图标 ID 是原型链名称时回退到默认文档图标', () => {
    vi.mocked(isMobilePlatformSync).mockReturnValueOnce(true);
    render(
      <AttachmentPageCard
        page={{ ...page, pageIcon: 'toString' }}
        pageKey="page1"
        isExpanded={false}
        showTrash={false}
        selectedIds={new Set()}
        expandedObjects={new Set()}
        onToggle={vi.fn()}
        onToggleObject={vi.fn()}
        onUpload={vi.fn()}
        loadData={vi.fn()}
        onToggleSelect={vi.fn()}
        onPreview={vi.fn()}
        onDownload={vi.fn()}
        onShare={vi.fn()}
        onSoftDelete={vi.fn()}
        onRestore={vi.fn()}
        onPermanentDelete={vi.fn()}
      />,
    );

    expect(document.querySelector('.lucide-file-text')).not.toBeNull();
  });

  it('passes updated callbacks through the expanded page and object to its attachment', () => {
    const oldRestore = vi.fn();
    const newRestore = vi.fn();
    const selectedIds = new Set<string>();
    const expandedObjects = new Set(['page1::obj1']);
    const handlers = {
      onToggle: vi.fn(),
      onToggleObject: vi.fn(),
      onUpload: vi.fn(),
      loadData: vi.fn(),
      onToggleSelect: vi.fn(),
      onPreview: vi.fn(),
      onDownload: vi.fn(),
      onShare: vi.fn(),
      onSoftDelete: vi.fn(),
      onPermanentDelete: vi.fn(),
    };
    const renderCard = (onRestore: typeof oldRestore) => (
      <AttachmentPageCard
        page={page}
        pageKey="page1"
        isExpanded
        showTrash
        selectedIds={selectedIds}
        expandedObjects={expandedObjects}
        {...handlers}
        onRestore={onRestore}
      />
    );

    const { rerender } = render(renderCard(oldRestore));
    rerender(renderCard(newRestore));
    fireEvent.click(screen.getByTitle('common:restore'));

    expect(newRestore).toHaveBeenCalledWith(attachment, 'obj1');
    expect(oldRestore).not.toHaveBeenCalled();
  });
});
