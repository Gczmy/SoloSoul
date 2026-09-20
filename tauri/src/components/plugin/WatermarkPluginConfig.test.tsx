import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { WatermarkPluginConfig } from './WatermarkPluginConfig';

vi.mock('@tauri-apps/api/path', () => ({ downloadDir: async () => 'test-output' }));
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
