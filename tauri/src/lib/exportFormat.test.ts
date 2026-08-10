import { describe, it, expect } from 'vitest';
import { swapDocumentExt } from './exportFormat';

describe('swapDocumentExt', () => {
  it('把已选路径的扩展名替换为新格式扩展名', () => {
    expect(swapDocumentExt('/vault/export/报告.docx', 'pdf')).toBe('/vault/export/报告.pdf');
    expect(swapDocumentExt('C:\\Users\\me\\Documents\\report.docx', 'html')).toBe(
      'C:\\Users\\me\\Documents\\report.html',
    );
    expect(swapDocumentExt('/vault/export/a.PDF', 'docx')).toBe('/vault/export/a.docx');
  });

  it('已经是目标格式扩展名时原样返回', () => {
    expect(swapDocumentExt('/vault/export/a.pdf', 'pdf')).toBe('/vault/export/a.pdf');
    expect(swapDocumentExt('/vault/export/a.html', 'html')).toBe('/vault/export/a.html');
  });

  it('无扩展名时追加新格式扩展名', () => {
    expect(swapDocumentExt('/vault/export/SoloSoul_导出', 'pdf')).toBe(
      '/vault/export/SoloSoul_导出.pdf',
    );
  });

  it('隐藏文件（.env）追加扩展名而非吞掉文件名', () => {
    expect(swapDocumentExt('/vault/export/.env', 'docx')).toBe('/vault/export/.env.docx');
  });

  it('路径中目录带点（如版本目录）不受影响', () => {
    expect(swapDocumentExt('/vault/v2.1/export/报告', 'pdf')).toBe('/vault/v2.1/export/报告.pdf');
  });

  it('多段扩展名只替换最后一段', () => {
    expect(swapDocumentExt('/vault/export/a.tar.gz', 'docx')).toBe('/vault/export/a.tar.docx');
  });

  it('SAF URI 原样返回（移动端由系统接管）', () => {
    expect(swapDocumentExt('content://com.android.externalstorage.documents/tree/1/download/导出.docx', 'pdf')).toBe(
      'content://com.android.externalstorage.documents/tree/1/download/导出.docx',
    );
  });

  it('txt / markdown 格式切换扩展名', () => {
    expect(swapDocumentExt('/vault/export/报告.docx', 'txt')).toBe('/vault/export/报告.txt');
    expect(swapDocumentExt('/vault/export/报告.docx', 'markdown')).toBe('/vault/export/报告.md');
    expect(swapDocumentExt('/vault/export/报告.md', 'pdf')).toBe('/vault/export/报告.pdf');
    expect(swapDocumentExt('/vault/export/报告.txt', 'markdown')).toBe('/vault/export/报告.md');
    // 已是目标扩展名 → 原样
    expect(swapDocumentExt('/vault/export/报告.md', 'markdown')).toBe('/vault/export/报告.md');
  });
});
