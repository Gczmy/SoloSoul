import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { UpdateBanner } from './UpdateBanner';

const baseProps = {
  version: '2.1.0',
  onUpdate: vi.fn(),
  onInstall: vi.fn(),
  onSkip: vi.fn(),
  onClose: vi.fn(),
  downloadedBytes: 0,
  totalBytes: 0,
};

describe('UpdateBanner', () => {
  it('renders version info and action buttons when available', () => {
    render(<UpdateBanner {...baseProps} state="available" />);

    expect(screen.getByText(/update_available/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /update_now/i })).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /skip_version/i })).toBeInTheDocument();
  });

  it('calls onUpdate when clicking update button', () => {
    const onUpdate = vi.fn();
    render(<UpdateBanner {...baseProps} state="available" onUpdate={onUpdate} />);

    fireEvent.click(screen.getByRole('button', { name: /update_now/i }));
    expect(onUpdate).toHaveBeenCalledTimes(1);
  });

  it('calls onSkip when clicking skip button', () => {
    const onSkip = vi.fn();
    render(<UpdateBanner {...baseProps} state="available" onSkip={onSkip} />);

    fireEvent.click(screen.getByRole('button', { name: /skip_version/i }));
    expect(onSkip).toHaveBeenCalledTimes(1);
  });

  it('calls onClose when clicking close button', () => {
    const onClose = vi.fn();
    render(<UpdateBanner {...baseProps} state="available" onClose={onClose} />);

    fireEvent.click(screen.getByLabelText(/close/i));
    expect(onClose).toHaveBeenCalledTimes(1);
  });

  it('shows view-release-notes button only when available and releaseNotes provided', () => {
    const { rerender } = render(<UpdateBanner {...baseProps} state="available" />);
    // 未提供 releaseNotes：按钮不渲染
    expect(screen.queryByLabelText(/view_release_notes/i)).not.toBeInTheDocument();

    const notes = '## New features\n- Sync';
    rerender(<UpdateBanner {...baseProps} state="available" releaseNotes={notes} />);
    expect(screen.getByLabelText(/view_release_notes/i)).toBeInTheDocument();

    // downloading 状态：按钮消失（点击安装后进入进度条不再展示）
    rerender(
      <UpdateBanner
        {...baseProps}
        state="downloading"
        releaseNotes={notes}
        downloadedBytes={10}
        totalBytes={100}
      />,
    );
    expect(screen.queryByLabelText(/view_release_notes/i)).not.toBeInTheDocument();
  });

  it('opens release notes dialog on click and closes it', () => {
    render(
      <UpdateBanner
        {...baseProps}
        state="available"
        releaseNotes={'## New features\n- Cloud sync'}
      />,
    );

    fireEvent.click(screen.getByLabelText(/view_release_notes/i));
    // Dialog 标题含版本号，正文经 SafeMarkdown 渲染（列表项独立文本节点）
    expect(screen.getByText(/release_notes_title/i)).toBeInTheDocument();
    expect(screen.getByText('Cloud sync')).toBeInTheDocument();

    // Dialog 组件无独立 X 按钮（本项目统一 backdrop/Escape 关闭），
    // 用 Escape 键关闭弹卡（注意不能点横幅自身的 close 按钮——那关闭的是横幅）。
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByText(/release_notes_title/i)).not.toBeInTheDocument();
  });

  it('renders progress info when downloading', () => {
    render(
      <UpdateBanner
        {...baseProps}
        state="downloading"
        downloadedBytes={50 * 1024 * 1024}
        totalBytes={100 * 1024 * 1024}
      />,
    );

    expect(screen.getByText(/update_downloading/i)).toBeInTheDocument();
    expect(screen.getByText(/50.0 MB \/ 100.0 MB/i)).toBeInTheDocument();
  });

  it('keeps byte counter in LTR order (no rtl bidi reflow)', () => {
    // 防回归：direction: rtl 会对「27.0 MB / 44.2 MB」这类数字+单位文本做 bidi 重排（MB 27.0）。
    // 断言字节数文本 span 未设置 rtl 方向，且文本保持「数字 单位 / 数字 单位」顺序。
    render(
      <UpdateBanner
        {...baseProps}
        state="downloading"
        downloadedBytes={27 * 1024 * 1024}
        totalBytes={44.2 * 1024 * 1024}
      />,
    );

    const counter = screen.getByText(/27.0 MB \/ 44.2 MB/i);
    expect(counter).toBeInTheDocument();
    // DOM 文本必须保持逻辑顺序：数字在前、单位在后（bidi 重排发生在视觉层，但 rtl 方向是根因，一并防住）
    expect(counter.textContent).toBe('27.0 MB / 44.2 MB');
    expect(counter.style.direction).not.toBe('rtl');
  });

  it('renders install button when downloaded', () => {
    render(<UpdateBanner {...baseProps} state="downloaded" />);

    expect(screen.getByText(/update_downloaded/i)).toBeInTheDocument();
    expect(screen.getByRole('button', { name: /install_update/i })).toBeInTheDocument();
  });

  it('calls onInstall when clicking install button', () => {
    const onInstall = vi.fn();
    render(<UpdateBanner {...baseProps} state="downloaded" onInstall={onInstall} />);

    fireEvent.click(screen.getByRole('button', { name: /install_update/i }));
    expect(onInstall).toHaveBeenCalledTimes(1);
  });
});
