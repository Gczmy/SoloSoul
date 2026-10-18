import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { render, screen, waitFor } from '@testing-library/react';
import { Html5Qrcode } from 'html5-qrcode';
import { RecoveryQrScanner } from './RecoveryQrScanner';

const { isAndroidSync, nativeProbe, nativeFormats, translate } = vi.hoisted(() => ({
  isAndroidSync: vi.fn(() => true),
  nativeProbe: vi.fn(),
  nativeFormats: vi.fn(async () => ['qr_code']),
  translate: (key: string) => key,
}));

vi.mock('@/lib/platform', () => ({ isAndroidSync }));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: translate }),
}));

describe('RecoveryQrScanner real decoder selection', () => {
  beforeEach(() => {
    isAndroidSync.mockReturnValue(true);
    nativeProbe.mockClear();
    nativeFormats.mockClear();
    class NativeDetector {
      static getSupportedFormats = nativeFormats;
      constructor(...args: unknown[]) {
        nativeProbe(...args);
      }
      async detect() {
        return [];
      }
    }
    vi.stubGlobal('BarcodeDetector', NativeDetector);
    vi.stubGlobal('navigator', {
      userAgent: navigator.userAgent,
      mediaDevices: {
        getUserMedia: vi.fn(async () => {
          throw new DOMException('Camera permission denied', 'NotAllowedError');
        }),
      },
    });
    vi.spyOn(Html5Qrcode, 'getCameras').mockResolvedValue([]);
  });

  afterEach(() => {
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it('Android never probes the native detector and still reports camera denial', async () => {
    const onError = vi.fn();
    const { unmount } = render(<RecoveryQrScanner onScan={vi.fn()} onError={onError} />);
    await waitFor(() => expect(onError).toHaveBeenCalledTimes(1));
    expect(screen.getByText('common:recovery_qr_permission_denied')).toBeInTheDocument();
    expect(nativeProbe).not.toHaveBeenCalled();
    expect(nativeFormats).not.toHaveBeenCalled();
    expect(() => unmount()).not.toThrow();
  });

  it('other platforms retain the native decoder selection when available', async () => {
    isAndroidSync.mockReturnValue(false);
    const onError = vi.fn();
    const { unmount } = render(<RecoveryQrScanner onScan={vi.fn()} onError={onError} />);
    await waitFor(() => expect(onError).toHaveBeenCalledTimes(1));
    expect(nativeProbe).toHaveBeenCalled();
    expect(() => unmount()).not.toThrow();
  });
});
