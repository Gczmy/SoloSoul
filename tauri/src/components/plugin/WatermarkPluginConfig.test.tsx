import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { WatermarkPluginConfig } from './WatermarkPluginConfig';

const dialogMocks = vi.hoisted(() => ({ openWithPause: vi.fn() }));

vi.mock('@tauri-apps/api/path', () => ({ downloadDir: async () => 'test-output' }));
vi.mock('@/lib/dialog', () => ({ openWithPause: dialogMocks.openWithPause }));
vi.mock('@/lib/ipcClient', () => ({
  invokeCommand: async () =>
    JSON.stringify({
      pages: [
        {
          pageName: 'identity',
          objects: [
            {
              objectId: 'obj-1',
              objectName: 'Passport',
              attachments: [
                {
                  id: 'att-1',
                  objectId: 'obj-1',
                  fileName: 'scan.pdf',
                  mimeType: 'application/pdf',
                },
              ],
            },
          ],
        },
      ],
    }),
}));
vi.mock('@/hooks/useAttachmentPageSort', () => ({
  useAttachmentPageSort: (pages: unknown[]) => pages,
}));

beforeEach(() => {
  vi.clearAllMocks();
  dialogMocks.openWithPause.mockResolvedValue(null);
  useAuthStore.setState({
    isAuthenticated: true,
    currentAccount: { id: 'account-a', name: 'Alice' },
  });
});

async function setup() {
  const onParamsChange = vi.fn();
  render(<WatermarkPluginConfig onParamsChange={onParamsChange} />);
  await screen.findByLabelText('scan.pdf');
  await waitFor(() => expect(onParamsChange.mock.lastCall?.[0].outputDir).toBe('test-output'));
  onParamsChange.mockClear();
  return onParamsChange;
}

describe('WatermarkPluginConfig 原生 label', () => {
  it('平铺复选框和文字各只切换一次，不再被父 label 再次反转', async () => {
    const onParamsChange = await setup();
    const checkbox = screen.getByLabelText('平铺水印');
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
    expect(onParamsChange).toHaveBeenCalledTimes(1);
    expect(JSON.parse(onParamsChange.mock.lastCall![0].watermarkConfig).tile).toBe(true);
    fireEvent.click(screen.getByText('平铺水印'));
    expect(checkbox).not.toBeChecked();
    expect(onParamsChange).toHaveBeenCalledTimes(2);
  });

  it('附件复选框和文件名文本都只更新一次运行参数', async () => {
    const onParamsChange = await setup();
    const checkbox = screen.getByLabelText('scan.pdf');
    fireEvent.click(checkbox);
    expect(checkbox).toBeChecked();
    expect(onParamsChange).toHaveBeenCalledTimes(1);
    expect(JSON.parse(onParamsChange.mock.lastCall![0].selectedAttachments)).toEqual([
      expect.objectContaining({ objectId: 'obj-1', attachmentId: 'att-1', fileName: 'scan.pdf' }),
    ]);
    fireEvent.click(screen.getByText('scan.pdf'));
    expect(checkbox).not.toBeChecked();
    expect(onParamsChange).toHaveBeenCalledTimes(2);
    expect(JSON.parse(onParamsChange.mock.lastCall![0].selectedAttachments)).toEqual([]);
  });
});

describe('RF-1024 watermark run parameters', () => {
  it('sends edited watermark options and chosen output directory to the plugin runner', async () => {
    const onParamsChange = await setup();
    dialogMocks.openWithPause.mockResolvedValueOnce('C:/Exports');

    fireEvent.change(screen.getByLabelText('水印文本'), { target: { value: 'Confidential' } });
    fireEvent.change(screen.getByLabelText('字号'), { target: { value: '96' } });
    const opacityInput = screen.getByText('透明度').closest('label')?.querySelector('input');
    expect(opacityInput).not.toBeNull();
    fireEvent.change(opacityInput as HTMLInputElement, { target: { value: '0.65' } });
    fireEvent.change(screen.getByLabelText('旋转角度'), { target: { value: '30' } });
    const colorInputs = screen
      .getByText('颜色 (R,G,B)')
      .closest('label')
      ?.querySelectorAll('input');
    expect(colorInputs).toHaveLength(3);
    fireEvent.change(colorInputs![0], { target: { value: '12' } });
    fireEvent.change(colorInputs![1], { target: { value: '34' } });
    fireEvent.change(colorInputs![2], { target: { value: '56' } });
    fireEvent.change(screen.getByLabelText('位置'), { target: { value: 'bottomRight' } });
    fireEvent.click(screen.getByLabelText('平铺水印'));
    fireEvent.click(screen.getByTitle('更改输出目录'));

    await waitFor(() => expect(onParamsChange.mock.lastCall?.[0].outputDir).toBe('C:/Exports'));
    expect(dialogMocks.openWithPause).toHaveBeenCalledWith({ directory: true });
    expect(JSON.parse(onParamsChange.mock.lastCall![0].watermarkConfig)).toMatchObject({
      text: 'Confidential',
      fontSize: 96,
      color: [12, 34, 56],
      opacity: 0.65,
      angle: 30,
      position: 'bottomRight',
      tile: true,
    });
    expect(onParamsChange.mock.lastCall![0].selectedAttachments).toBe('[]');
  });

  it('keeps the current output directory when the picker is cancelled', async () => {
    const onParamsChange = await setup();
    fireEvent.click(screen.getByTitle('更改输出目录'));
    await waitFor(() => expect(dialogMocks.openWithPause).toHaveBeenCalledOnce());
    expect(onParamsChange).not.toHaveBeenCalled();
    expect(screen.getByText('test-output')).toBeInTheDocument();
  });
});
