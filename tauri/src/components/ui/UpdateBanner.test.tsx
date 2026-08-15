import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';

// 与 OperationLogCard.test 同款 mock：useTranslation 返回 key（无 defaultValue 时）。
vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (key: string, options?: { defaultValue?: string }) => options?.defaultValue ?? key,
    i18n: { language: 'en' },
  }),
}));

const baseProps = {
  version: '2.0.0',
  state: 'available' as const,
  downloadedBytes: 0,
  totalBytes: 0,
  onUpdate: vi.fn(),
  onInstall: vi.fn(),
  onSkip: vi.fn(),
  onClose: vi.fn(),
};

describe('UpdateBanner', () => {
  it('P012: renders checksum warning strip in available state when provided', () => {
    render(
      <UpdateBanner
        {...baseProps}
        checksumWarning="校验和签名缺失或验签失败，无法确认 APK 完整性"
      />,
    );
    expect(screen.getByText('校验和签名缺失或验签失败，无法确认 APK 完整性')).toBeInTheDocument();
  });

  it('P012: no warning strip when checksumWarning absent', () => {
    const { container } = render(<UpdateBanner {...baseProps} />);
    expect(container.textContent).not.toContain('校验和');
  });

  it('P012: no warning strip during downloading state (仅 available 展示)', () => {
    render(<UpdateBanner {...baseProps} state="downloading" checksumWarning="warn" />);
    expect(screen.queryByText('warn')).not.toBeInTheDocument();
  });
});
