import { render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { UpdateTransferStatus } from './UpdateTransferStatus';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, args?: Record<string, string>) => {
      if (key === 'update_download_source') return `线路：${args?.source}`;
      if (key === 'update_download_speed') return `${args?.speed}/s`;
      return key;
    },
  }),
}));

describe('UpdateTransferStatus', () => {
  it('选源与换源展示明确状态；下载时显示安全 host 和格式化速度', () => {
    const { rerender } = render(<UpdateTransferStatus transfer={{ phase: 'probing' }} />);
    expect(screen.getByText('update_selecting_source')).toBeInTheDocument();
    rerender(
      <UpdateTransferStatus
        transfer={{
          phase: 'downloading',
          source: 'https://user:secret@mirror.example/releases/app?token=private',
          bytesPerSecond: 2097152,
        }}
      />,
    );
    expect(screen.getByText('线路：mirror.example · 2.0 MB/s')).toBeInTheDocument();
    expect(document.body.textContent).not.toMatch(/secret|private|token|releases/);
    rerender(
      <UpdateTransferStatus
        transfer={{ phase: 'switching', source: 'github.com', bytesPerSecond: 2097152 }}
      />,
    );
    expect(screen.getByText('update_switching_source · 线路：github.com')).toBeInTheDocument();
    expect(document.body.textContent).not.toContain('MB/s');
  });

  it('兼容不带遥测的旧后端，忽略无效速度和无效主机', () => {
    const { container, rerender } = render(<UpdateTransferStatus />);
    expect(container).toBeEmptyDOMElement();
    rerender(
      <UpdateTransferStatus
        transfer={{ phase: 'downloading', source: 'invalid host', bytesPerSecond: NaN }}
      />,
    );
    expect(container).toBeEmptyDOMElement();
  });
});
