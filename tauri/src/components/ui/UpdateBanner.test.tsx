import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';

// P015-R2: SafeMarkdown 改为动态导入——mock 使动态加载解析快速且确定性
vi.mock('@/components/ui/SafeMarkdown', () => ({
  SafeMarkdown: ({ children }: { children: string }) => (
    <div data-testid="release-notes-md">{children}</div>
  ),
}));

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

  it('P015-R2: release notes 先以纯文本降级渲染，动态加载后切换到 markdown', async () => {
    render(<UpdateBanner {...baseProps} releaseNotes="第一行\n- 列表项" />);
    fireEvent.click(screen.getByLabelText('view_release_notes'));
    // 动态导入未完成前：纯文本 <pre> 兜底（不空白）
    const pre = document.querySelector('pre.release-notes-md');
    expect(pre).not.toBeNull();
    expect(pre?.textContent).toContain('第一行');
    // 动态导入完成后：切换到 markdown 渲染
    expect(await screen.findByTestId('release-notes-md')).toBeInTheDocument();
  });
});
