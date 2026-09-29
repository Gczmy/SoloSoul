import { describe, it, expect, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { Suspense } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { LazyPhotoViewerOverlay } from './LazyPhotoViewerOverlay';
import type { AttachmentItem } from '@/lib/attachmentUtils';
// 预载真实实现以隔离 Vite/framer-motion 冷编译耗时；下方仍通过 React.lazy
// 包装渲染，验证命名导出映射与实际查看器内容。
import './PhotoViewerOverlay';

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
    // 外层测试超时需长于 waitFor，以保留断言执行时间。
    await waitFor(
      () => {
        expect(screen.getByTestId('photo-viewer-counter')).toHaveTextContent('1 / 2');
      },
      { timeout: 8000 },
    );
  }, 12000);
});
