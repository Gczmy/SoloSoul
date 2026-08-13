import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import {
  getAttachmentFormatType,
  AttachmentTypeIcon,
  AttachmentExtBadge,
} from './AttachmentFormatBadge';

const mk = (fileName: string, mimeType: string) => ({ fileName, mimeType });

describe('getAttachmentFormatType', () => {
  it('图片：按 mimeType 识别', () => {
    expect(getAttachmentFormatType(mk('a.png', 'image/png'))).toBe('image');
    expect(getAttachmentFormatType(mk('b.jpg', 'image/jpeg'))).toBe('image');
  });

  it('图片：仅按扩展名识别（mimeType 为空/非图片——真实数据常见场景）', () => {
    expect(getAttachmentFormatType(mk('a.png', ''))).toBe('image');
    expect(getAttachmentFormatType(mk('b.webp', 'application/octet-stream'))).toBe('image');
  });

  it('PDF：按 mimeType 或扩展名识别', () => {
    expect(getAttachmentFormatType(mk('a.pdf', 'application/pdf'))).toBe('pdf');
    expect(getAttachmentFormatType(mk('b.pdf', ''))).toBe('pdf');
  });

  it('文本类归 other（图标回落回形针，与回收站参考一致）', () => {
    expect(getAttachmentFormatType(mk('a.txt', 'text/plain'))).toBe('other');
    expect(getAttachmentFormatType(mk('b.json', ''))).toBe('other');
  });

  it('未知格式归 other', () => {
    expect(getAttachmentFormatType(mk('a.xyz', 'application/octet-stream'))).toBe('other');
  });
});

describe('AttachmentTypeIcon', () => {
  it('按格式渲染对应 lucide 图标（image→Image / pdf→FileText / 其余→回形针）', () => {
    const { container: img } = render(
      <AttachmentTypeIcon item={mk('a.png', 'image/png')} size={12} />,
    );
    expect(img.querySelector('svg.lucide-image')).not.toBeNull();

    const { container: pdf } = render(
      <AttachmentTypeIcon item={mk('b.pdf', 'application/pdf')} size={12} />,
    );
    expect(pdf.querySelector('svg.lucide-file-text')).not.toBeNull();

    const { container: other } = render(
      <AttachmentTypeIcon item={mk('c.zip', 'application/zip')} size={12} />,
    );
    expect(other.querySelector('svg.lucide-paperclip')).not.toBeNull();
  });
});

describe('AttachmentExtBadge', () => {
  it('扩展名大写展示', () => {
    render(<AttachmentExtBadge fileName="photo.png" />);
    expect(screen.getByText('PNG')).toBeInTheDocument();
  });

  it('无扩展名回退 FILE（文件名以点结尾产生空扩展名）', () => {
    render(<AttachmentExtBadge fileName="trailing." />);
    expect(screen.getByText('FILE')).toBeInTheDocument();
  });
});
