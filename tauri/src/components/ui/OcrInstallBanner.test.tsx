import { act, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { OcrInstallBanner } from './OcrInstallBanner';

afterEach(() => {
  vi.useRealTimers();
});

describe('OcrInstallBanner', () => {
  it('准备模型时显示可访问的进度，并允许关闭通知', () => {
    const onClose = vi.fn();
    render(
      <OcrInstallBanner
        phase="installing"
        progress={42}
        error={null}
        onRetry={vi.fn()}
        onClose={onClose}
      />,
    );
    expect(screen.getByRole('progressbar')).toHaveAttribute('aria-valuenow', '42');
    fireEvent.click(screen.getByRole('button', { name: 'close' }));
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('错误状态展示诊断信息并保留重试和关闭入口', () => {
    const onRetry = vi.fn();
    const onClose = vi.fn();
    const error = `network error: ${'model-download-'.repeat(30)}`;
    render(
      <OcrInstallBanner
        phase="error"
        progress={30}
        error={error}
        onRetry={onRetry}
        onClose={onClose}
      />,
    );
    expect(screen.getByTitle(error)).toHaveTextContent(error);
    expect(screen.queryByRole('progressbar')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'first_install_retry' }));
    fireEvent.click(screen.getByRole('button', { name: 'close' }));
    expect(onRetry).toHaveBeenCalledOnce();
    expect(onClose).toHaveBeenCalledOnce();
  });

  it('完成后仍自动关闭；主动关闭会取消剩余倒计时', () => {
    vi.useFakeTimers();
    const onClose = vi.fn();
    const { unmount } = render(
      <OcrInstallBanner
        phase="completed"
        progress={100}
        error={null}
        onRetry={vi.fn()}
        onClose={onClose}
        autoDismissSeconds={2}
      />,
    );
    act(() => vi.advanceTimersByTime(2000));
    expect(onClose).toHaveBeenCalledOnce();
    unmount();

    const manualClose = vi.fn();
    render(
      <OcrInstallBanner
        phase="completed"
        progress={100}
        error={null}
        onRetry={vi.fn()}
        onClose={manualClose}
        autoDismissSeconds={2}
      />,
    );
    fireEvent.click(screen.getByRole('button', { name: 'close' }));
    act(() => vi.advanceTimersByTime(5000));
    expect(manualClose).toHaveBeenCalledOnce();
  });
});
