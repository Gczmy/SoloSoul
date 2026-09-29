import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { RecoveryManualView } from './RecoveryManualView';
import type { RecoveryDiscoveredHost } from './recoveryReceiveTypes';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const host: RecoveryDiscoveredHost = {
  name: 'Laptop',
  addr: '192.168.1.8:9042',
  fingerprint: 'abc123',
};

function setup(overrides: Partial<React.ComponentProps<typeof RecoveryManualView>> = {}) {
  const callbacks = {
    onHostAddrChange: vi.fn(),
    onPinChange: vi.fn(),
    onFingerprintChange: vi.fn(),
    onToggleAdvanced: vi.fn(),
    onScanLan: vi.fn(),
    onSelectHost: vi.fn(),
    onNext: vi.fn(),
  };
  const props: React.ComponentProps<typeof RecoveryManualView> = {
    loading: false,
    scanning: false,
    discoveredHosts: [],
    scanError: null,
    scanDone: false,
    hostAddr: '',
    pin: '',
    fingerprint: '',
    showAdvanced: false,
    error: null,
    ...callbacks,
    ...overrides,
  };
  const view = render(<RecoveryManualView {...props} />);
  return { ...callbacks, props, view };
}

beforeEach(() => vi.clearAllMocks());

describe('RF-1023 manual recovery view', () => {
  it('passes address, normalized six-digit PIN, advanced fingerprint and next action to the flow', () => {
    const {
      props,
      view,
      onHostAddrChange,
      onPinChange,
      onFingerprintChange,
      onToggleAdvanced,
      onNext,
    } = setup();

    fireEvent.change(screen.getByPlaceholderText('common:recovery_receive_addr_placeholder'), {
      target: { value: '192.168.1.8:9042' },
    });
    fireEvent.change(screen.getByPlaceholderText('123456'), {
      target: { value: 'a1b2c3d4e5f6' },
    });
    expect(onHostAddrChange).toHaveBeenCalledWith('192.168.1.8:9042');
    expect(onPinChange).toHaveBeenCalledWith('123456');

    fireEvent.click(screen.getByRole('button', { name: 'common:recovery_advanced_show' }));
    expect(onToggleAdvanced).toHaveBeenCalledOnce();
    view.rerender(<RecoveryManualView {...props} showAdvanced />);
    fireEvent.change(screen.getByPlaceholderText('common:recovery_fingerprint_placeholder'), {
      target: { value: 'abc123' },
    });
    expect(onFingerprintChange).toHaveBeenCalledWith('abc123');
    expect(
      screen.getByRole('button', { name: 'common:recovery_advanced_hide' }),
    ).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'common:next' }));
    expect(onNext).toHaveBeenCalledOnce();
  });

  it('scans the LAN and forwards the selected discovered host', () => {
    const { onScanLan, onSelectHost } = setup({ discoveredHosts: [host], scanDone: true });

    fireEvent.click(screen.getByRole('button', { name: 'common:recovery_scan_button' }));
    fireEvent.click(screen.getByText('Laptop').closest('button') as HTMLButtonElement);
    expect(onScanLan).toHaveBeenCalledOnce();
    expect(onSelectHost).toHaveBeenCalledWith(host);
    expect(screen.getByText(host.addr)).toBeInTheDocument();
  });

  it('prevents scan and form actions while loading and hides stale errors during scanning', () => {
    const { props, view, onScanLan, onSelectHost, onNext } = setup({
      scanning: true,
      scanError: 'old network error',
      discoveredHosts: [host],
    });
    expect(screen.getByRole('button', { name: 'common:recovery_scan_scanning' })).toBeDisabled();
    expect(screen.queryByText('old network error')).not.toBeInTheDocument();

    view.rerender(
      <RecoveryManualView {...props} scanning={false} loading error="Cannot continue" />,
    );
    const scanButton = screen.getByRole('button', { name: 'common:recovery_scan_button' });
    const hostButton = screen.getByText('Laptop').closest('button') as HTMLButtonElement;
    expect(scanButton).toBeDisabled();
    expect(hostButton).toBeDisabled();
    expect(screen.getByPlaceholderText('common:recovery_receive_addr_placeholder')).toBeDisabled();
    expect(screen.getByPlaceholderText('123456')).toBeDisabled();
    expect(screen.getByRole('button', { name: 'common:next' })).toBeDisabled();
    expect(screen.getByText('old network error')).toBeInTheDocument();
    expect(screen.getByText('Cannot continue')).toBeInTheDocument();
    fireEvent.click(scanButton);
    fireEvent.click(hostButton);
    fireEvent.click(screen.getByRole('button', { name: 'common:next' }));
    expect(onScanLan).not.toHaveBeenCalled();
    expect(onSelectHost).not.toHaveBeenCalled();
    expect(onNext).not.toHaveBeenCalled();
  });

  it('distinguishes an empty completed scan from a failed scan', () => {
    const { props, view } = setup({ scanDone: true, scanError: 'No devices found' });
    expect(screen.getByText('No devices found')).toHaveStyle({ color: 'var(--text-tertiary)' });

    view.rerender(<RecoveryManualView {...props} scanDone={false} />);
    expect(screen.getByText('No devices found')).toHaveStyle({ color: '#e74c3c' });
  });
});
