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
