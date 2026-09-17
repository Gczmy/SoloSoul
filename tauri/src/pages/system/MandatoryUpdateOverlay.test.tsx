import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { MandatoryUpdateOverlay } from './MandatoryUpdateOverlay';
import type { AppInfo, VersionInfo } from '@/hooks/useUpdateChecker';

const baseInfo: AppInfo = {
  appName: 'SoloSoul',
  version: '1.0.0',
  os: 'windows',
  arch: 'x64',
};

const baseVersionInfo: VersionInfo = {
  currentVersion: '1.0.0',
  latestVersion: '2.0.0',
  state: 'available',
  mandatory: true,
};

const baseProps = {
  isMandatory: true,
  info: baseInfo,
  versionInfo: baseVersionInfo,
  downloading: false,
  downloadedBytes: 0,
  totalBytes: 0,
  progressPercent: 0,
  downloadError: null,
  handleUpdate: vi.fn(),
};

describe('MandatoryUpdateOverlay', () => {
  it('renders null when isMandatory is false', () => {
    const { container } = render(<MandatoryUpdateOverlay {...baseProps} isMandatory={false} />);
    expect(container.innerHTML).toBe('');
  });

  it('cancels only the download and keeps the mandatory update options after it settles', () => {
    const cancelDownload = vi.fn();
    const { rerender } = render(
      <MandatoryUpdateOverlay {...baseProps} downloading cancelDownload={cancelDownload} />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'common:cancel_download' }));
    expect(cancelDownload).toHaveBeenCalledOnce();
    rerender(
      <MandatoryUpdateOverlay
        {...baseProps}
        downloading
        cancelling
        cancelDownload={cancelDownload}
      />,
    );
    expect(screen.getByRole('button', { name: 'common:cancel_download' })).toBeDisabled();
    expect(screen.getByText('common:update_cancelling')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Update Now' })).not.toBeInTheDocument();
    rerender(<MandatoryUpdateOverlay {...baseProps} cancelDownload={cancelDownload} />);
    expect(screen.getByRole('heading', { name: '关键安全更新' })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Update Now' })).toBeInTheDocument();
  });

  it('hides cancel only once installation actually starts, not at 100% download progress', () => {
    const { rerender } = render(
      <MandatoryUpdateOverlay
        {...baseProps}
        downloading
        progressPercent={100}
        cancelDownload={vi.fn()}
      />,
    );
    expect(screen.getByRole('button', { name: 'common:cancel_download' })).toBeEnabled();
    expect(screen.queryByText('common:update_installing')).not.toBeInTheDocument();
    rerender(
      <MandatoryUpdateOverlay
        {...baseProps}
        downloading
        installing
        progressPercent={100}
        cancelDownload={vi.fn()}
      />,
    );
    expect(
      screen.queryByRole('button', { name: 'common:cancel_download' }),
    ).not.toBeInTheDocument();
    expect(screen.getByText('common:update_installing')).toBeInTheDocument();
  });

  it('shows view-release-notes button only when release notes provided', () => {
    const { rerender } = render(<MandatoryUpdateOverlay {...baseProps} />);
    // 无 body：按钮不渲染
    expect(screen.queryByLabelText(/view_release_notes/i)).not.toBeInTheDocument();

    rerender(
      <MandatoryUpdateOverlay
        {...baseProps}
        versionInfo={{ ...baseVersionInfo, body: '## New features\n- Sync' }}
      />,
    );
    expect(screen.getByLabelText(/view_release_notes/i)).toBeInTheDocument();
  });

  it('opens release notes dialog on click and closes on Escape', () => {
    render(
      <MandatoryUpdateOverlay
        {...baseProps}
        versionInfo={{ ...baseVersionInfo, body: '## New features\n- Cloud sync' }}
      />,
    );

    fireEvent.click(screen.getByLabelText(/view_release_notes/i));
    // Dialog 标题含版本号，正文经 SafeMarkdown 渲染（列表项独立文本节点）
    expect(screen.getByText(/release_notes_title/i)).toBeInTheDocument();
    expect(screen.getByText('Cloud sync')).toBeInTheDocument();

    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByText(/release_notes_title/i)).not.toBeInTheDocument();
  });

  it('P012: shows checksum warning when versionInfo.checksumWarning present', () => {
    render(
      <MandatoryUpdateOverlay
        {...baseProps}
        versionInfo={{
          ...baseVersionInfo,
          checksumWarning: '校验和签名缺失，无法确认 APK 完整性',
        }}
      />,
    );
    expect(screen.getByText('校验和签名缺失，无法确认 APK 完整性')).toBeInTheDocument();
    expect(screen.getByRole('alert')).toBeInTheDocument();
  });

  it('P012: no checksum warning block when checksumWarning absent', () => {
    const { container } = render(<MandatoryUpdateOverlay {...baseProps} />);
    expect(container.querySelector('[role="alert"]')).not.toBeInTheDocument();
  });

  it('keeps dialog above the overlay (zIndex > 9999)', () => {
    render(
      <MandatoryUpdateOverlay
        {...baseProps}
        versionInfo={{ ...baseVersionInfo, body: '## New features\n- Cloud sync' }}
      />,
    );

    fireEvent.click(screen.getByLabelText(/view_release_notes/i));
    const dialog = document.querySelector('[role="dialog"]');
    const wrapper = dialog?.parentElement;
    // Dialog wrapper 内联 zIndex 必须高于遮罩本体的 9999，否则弹卡被全屏遮罩盖住
    expect(wrapper?.style.zIndex).toBe('10000');
  });
});
