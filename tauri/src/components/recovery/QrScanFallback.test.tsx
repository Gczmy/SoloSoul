import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { QrScanFallback } from './QrScanFallback';

function setup(props: Partial<Parameters<typeof QrScanFallback>[0]> = {}) {
  const onAction = vi.fn();
  render(
    <QrScanFallback
      cameraCapability="supported"
      scannerError={null}
      unsupportedText="No camera"
      unsupportedButtonLabel="Manual"
      scannerErrorButtonLabel="Use manual"
      onAction={onAction}
      {...props}
    >
      <div>scanner-content</div>
    </QrScanFallback>,
  );
  return { onAction };
}

describe('QrScanFallback', () => {
  it('renders scanner children when camera supported and no error', () => {
    setup();
    expect(screen.getByText('scanner-content')).toBeInTheDocument();
  });

  it('shows unsupported placeholder with action button when no camera', () => {
    const { onAction } = setup({ cameraCapability: 'unsupported' });
    expect(screen.getByText('No camera')).toBeInTheDocument();
    fireEvent.click(screen.getByText('Manual'));
    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it('shows scanner-error fallback button when scannerError set', () => {
    const { onAction } = setup({ scannerError: 'permission denied' });
    fireEvent.click(screen.getByText('Use manual'));
    expect(onAction).toHaveBeenCalledTimes(1);
  });

  it('hides scanner-error fallback when camera unsupported', () => {
    setup({ cameraCapability: 'unsupported', scannerError: 'permission denied' });
    expect(screen.queryByText('Use manual')).not.toBeInTheDocument();
  });
});
