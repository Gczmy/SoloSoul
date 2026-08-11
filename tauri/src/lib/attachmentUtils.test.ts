import { describe, it, expect, vi, beforeEach } from 'vitest';

// mock 动态 import('@/lib/dialog')——downloadAttachmentFile 内 await import
const saveMock = vi.hoisted(() => vi.fn());
vi.mock('@/lib/dialog', () => ({
  saveWithPause: saveMock,
}));

import { downloadAttachmentFile, countActiveAttachments } from './attachmentUtils';

const tMock = ((key: string, opts?: Record<string, unknown>) =>
  String(key) + (opts ? JSON.stringify(opts) : '')) as never;

function makeParams(overrides: Record<string, unknown> = {}) {
  return {
    filePath: '/vault/attachments/a.pdf',
    fileName: 'a.pdf',
    invoke: vi.fn().mockResolvedValue(undefined),
    showToast: vi.fn(),
    t: tMock,
    downloadViaStage: vi.fn().mockResolvedValue(undefined),
    ...overrides,
  };
}

beforeEach(() => {
  vi.clearAllMocks();
});

describe('downloadAttachmentFile', () => {
  it('选择保存路径后调用 downloadViaStage 并 toast 成功', async () => {
    saveMock.mockResolvedValue('/Users/me/Downloads/a.pdf');
    const p = makeParams();
    const result = await downloadAttachmentFile(p as never);

    expect(result).toBe(true);
    expect(p.downloadViaStage).toHaveBeenCalledWith(
      '/vault/attachments/a.pdf',
      '/Users/me/Downloads/a.pdf',
      'a.pdf',
      expect.any(Function),
    );
    expect(p.showToast).toHaveBeenCalledWith({
      type: 'success',
      message: expect.stringContaining('common:download_result'),
    });
  });

  it('用户取消保存对话框时返回 false，不执行下载', async () => {
    saveMock.mockResolvedValue(null);
    const p = makeParams();
    const result = await downloadAttachmentFile(p as never);

    expect(result).toBe(false);
    expect(p.downloadViaStage).not.toHaveBeenCalled();
    expect(p.showToast).not.toHaveBeenCalled();
  });

  it('下载抛错时 toast 失败并返回 false', async () => {
    saveMock.mockResolvedValue('/Users/me/Downloads/a.pdf');
    const p = makeParams({
      downloadViaStage: vi.fn().mockRejectedValue(new Error('disk full')),
    });
    const result = await downloadAttachmentFile(p as never);

    expect(result).toBe(false);
    expect(p.showToast).toHaveBeenCalledWith({
      type: 'error',
      message: expect.stringContaining('common:download_failed'),
    });
  });
});

describe('countActiveAttachments', () => {
  it('扁平统计活跃附件总数（不含回收站）', () => {
    const pages = [
      { objects: [{ attachments: [{ id: 'a1' }, { id: 'a2' }] }] },
      {
        objects: [
          { attachments: [{ id: 'a3' }] },
          { attachments: [{ id: 'a4' }, { id: 'a5' }] },
        ],
      },
    ];
    expect(countActiveAttachments(pages as never)).toBe(5);
  });

  it('undefined / 空树返回 0', () => {
    expect(countActiveAttachments(undefined)).toBe(0);
    expect(countActiveAttachments([])).toBe(0);
  });
});
