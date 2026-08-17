import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { Suspense } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { LazyPhotoViewerOverlay } from './LazyPhotoViewerOverlay';
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

describe('LazyPhotoViewerOverlay', () => {
  it('resolves to the real PhotoViewerOverlay (named-export mapping intact)', async () => {
    mockInvoke.mockResolvedValue('data:image/png;base64,abc');
    render(
      <Suspense fallback={<div>lazy-loading</div>}>
        <LazyPhotoViewerOverlay
          items={[makeItem('a'), makeItem('b')]}
          initialIndex={0}
          onBack={() => {}}
          onClose={() => {}}
        />
      </Suspense>,
    );

    // 若 PhotoViewerOverlay 命名导出被重命名，lazy 工厂的 m.PhotoViewerOverlay
    // 解析为 undefined → React 抛 "Element type is invalid" → 本测试失败（漂移保护）。
    // 冷启动动态 import（含 framer-motion 手势引擎）较慢，放宽 waitFor 超时防抖动。
    await waitFor(
      () => {
        expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
      },
      { timeout: 8000 },
    );
  });
});
