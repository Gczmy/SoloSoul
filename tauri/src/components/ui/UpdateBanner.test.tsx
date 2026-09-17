import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';
import { isMobilePlatformSync } from '@/lib/platform';

vi.mock('@/lib/platform', () => ({ isMobilePlatformSync: vi.fn(() => false) }));

// P015-R2: 更新说明动态导入——mock 使动态加载解析快速且确定性
vi.mock('@/components/ui/ReleaseNotesMarkdown', () => ({
  ReleaseNotesMarkdown: ({ children }: { children: string }) => (
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
  onCancel: vi.fn(),
  onInstall: vi.fn(),
  onSkip: vi.fn(),
  onClose: vi.fn(),
};

describe('UpdateBanner', () => {
  it('移动端图标操作有明确名称，关闭及跳过操作仍独立可用', () => {
    vi.mocked(isMobilePlatformSync).mockReturnValue(true);
    const onUpdate = vi.fn();
    const onSkip = vi.fn();
    const onClose = vi.fn();
    try {
      render(
        <UpdateBanner
          {...baseProps}
          releaseNotes="更新说明"
          onUpdate={onUpdate}
          onSkip={onSkip}
          onClose={onClose}
        />,
      );
      expect(screen.getByRole('button', { name: 'view_release_notes' })).toBeEnabled();
      fireEvent.click(screen.getByRole('button', { name: 'update_now' }));
      fireEvent.click(screen.getByRole('button', { name: 'skip' }));
      fireEvent.click(screen.getByRole('button', { name: 'close' }));
      expect(onUpdate).toHaveBeenCalledOnce();
      expect(onSkip).toHaveBeenCalledOnce();
      expect(onClose).toHaveBeenCalledOnce();
    } finally {
      vi.mocked(isMobilePlatformSync).mockReturnValue(false);
    }
  });

  it('错误详情保留全文，重试和关闭仍能独立操作', () => {
    const error = `download failed: ${'long-file-name-'.repeat(30)}`;
    const onUpdate = vi.fn();
    const onClose = vi.fn();
    render(
      <UpdateBanner
        {...baseProps}
        state="error"
        error={error}
        onUpdate={onUpdate}
        onClose={onClose}
      />,
    );
    expect(screen.getByTitle(error)).toHaveTextContent(error);
    fireEvent.click(screen.getByRole('button', { name: 'retry' }));
    fireEvent.click(screen.getByRole('button', { name: 'close' }));
    expect(onUpdate).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('下载进度旁可取消，取消期间禁用按钮且不允许关闭横幅', () => {
    const cancel = vi.fn();
    const { rerender } = render(
      <UpdateBanner
        {...baseProps}
        state="downloading"
        onCancel={cancel}
        downloadedBytes={25}
        totalBytes={100}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'cancel_download' }));
    expect(cancel).toHaveBeenCalledOnce();
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '25');
    rerender(<UpdateBanner {...baseProps} state="cancelling" onCancel={cancel} />);
    expect(screen.getByRole('button', { name: 'cancel_download' })).toBeDisabled();
    expect(screen.getByText('update_cancelling')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'close' })).not.toBeInTheDocument();
  });

  it('强制更新仍可取消下载，但不能跳过或关闭', () => {
    render(<UpdateBanner {...baseProps} state="downloading" mandatory />);
    expect(screen.getByRole('button', { name: 'cancel_download' })).toBeEnabled();
    expect(screen.queryByRole('button', { name: 'skip' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'close' })).not.toBeInTheDocument();
  });

  it('进入安装阶段显示明确状态，不再允许取消或关闭', () => {
    render(<UpdateBanner {...baseProps} state="installing" />);
    expect(screen.getByRole('status')).toHaveTextContent('update_installing');
    expect(screen.queryByRole('button', { name: 'cancel_download' })).not.toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'close' })).not.toBeInTheDocument();
  });

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
