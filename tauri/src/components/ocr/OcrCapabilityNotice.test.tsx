import { act, fireEvent, render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { invokeTypedCommand } from '@/lib/typedIpc';
import { platformCapabilityStore, unavailableCapabilities } from '@/lib/platformCapabilities';
import { capabilityFixture } from '@/lib/__fixtures__/platformCapabilityFixture';
import { OcrCapabilityNotice } from './OcrCapabilityNotice';

vi.mock('@/lib/typedIpc', () => ({ invokeTypedCommand: vi.fn() }));
vi.mock('react-i18next', () => ({ useTranslation: () => ({ t: (key: string) => key }) }));
beforeEach(() => {
  vi.mocked(invokeTypedCommand).mockReset();
  platformCapabilityStore.setState({
    capabilities: unavailableCapabilities('capabilities_read_failed'),
    loaded: false,
    pending: null,
  });
});

describe('RF205 OCR 能力恢复', () => {
  it('重试只发送一个读取，两处提示同步禁用并在恢复后消失', async () => {
    let resolve!: (value: ReturnType<typeof capabilityFixture>) => void;
    vi.mocked(invokeTypedCommand).mockReturnValue(
      new Promise((done) => {
        resolve = done;
      }),
    );
    render(
      <>
        <OcrCapabilityNotice />
        <OcrCapabilityNotice />
      </>,
    );
    fireEvent.click(screen.getAllByRole('button', { name: 'common:retry' })[0]);
    for (const button of screen.getAllByRole('button')) expect(button).toBeDisabled();
    await act(async () => {
      await Promise.resolve();
    });
    expect(invokeTypedCommand).toHaveBeenCalledExactlyOnceWith('get_platform_capabilities');
    await act(async () => {
      resolve(capabilityFixture('android'));
    });
    expect(screen.queryByRole('status')).not.toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('iOS 未实现扫描时显示专门原因，不能以重试冒充支持', () => {
    platformCapabilityStore.setState({ capabilities: capabilityFixture('ios'), loaded: true });
    render(<OcrCapabilityNotice />);
    expect(screen.getByRole('status')).toHaveTextContent('ocr:ios_ocr_unsupported');
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
    expect(invokeTypedCommand).not.toHaveBeenCalled();
  });
});
