import { render, screen, fireEvent, waitFor } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { PluginResultPanel } from './PluginResultPanel';
import { invoke } from '@tauri-apps/api/core';
import { saveWithPause, openWithPause } from '@/lib/dialog';
import { useUiStore } from '@/stores/uiStore';
import type { PluginResultPayload } from '@/lib/plugin';

vi.mock('@/lib/dialog', () => ({ saveWithPause: vi.fn(), openWithPause: vi.fn() }));
vi.mock('@tauri-apps/api/path', () => ({
  dirname: async () => 'synthetic-destination',
  basename: async () => 'saved.png',
}));

describe('PluginResultPanel', () => {
  it('RF306 filters arbitrary wire JSON and malformed nested results before rendering', () => {
    const results: PluginResultPayload[] = [
      null,
      false,
      42,
      'scalar',
      ['array'],
      { type: 'unknown', content: 'unknown-content' },
      { type: 'text', content: { private: 'invalid-content' } },
      { type: 'key_value', title: 'invalid-pair', pairs: [null] },
      { type: 'table', headers: ['Header'], rows: [null] },
      { type: 'watermark_result', outputDir: 'synthetic', items: [{ fileName: 'invalid-file' }] },
      { type: 'expiry_guardian', title: 'invalid-expiry', items: [] },
      { type: 'text', content: 'visible valid result' },
    ];
    render(<PluginResultPanel results={results} />);
    expect(screen.getByText('visible valid result')).toBeInTheDocument();
    expect(screen.queryByText('invalid-pair')).not.toBeInTheDocument();
    expect(screen.queryByText('invalid-file')).not.toBeInTheDocument();
    expect(screen.queryByText('invalid-expiry')).not.toBeInTheDocument();
    expect(screen.queryByText('unknown-content')).not.toBeInTheDocument();
  });

  it('RF306 shows the empty state when no wire result is displayable', () => {
    render(<PluginResultPanel results={[null, { type: 'future' }]} />);
    expect(screen.getByText(/No result yet/i)).toBeInTheDocument();
  });

  it('renders empty state', () => {
    render(<PluginResultPanel results={[]} />);
    expect(screen.getByText(/No result yet/i)).toBeInTheDocument();
  });

  it('renders text result', () => {
    const results: PluginResultPayload[] = [{ type: 'text', content: 'Hello world' }];
    render(<PluginResultPanel results={results} />);
    expect(screen.getByText('Hello world')).toBeInTheDocument();
  });

  it('renders key_value result with title and per-pair copy', () => {
    const results: PluginResultPayload[] = [
      {
        type: 'key_value',
        title: 'Summary',
        pairs: [
          { key: 'Name', value: 'Alice' },
          { key: 'Age', value: '30' },
        ],
      },
    ];
    render(<PluginResultPanel results={results} />);
    expect(screen.getByText('Name')).toBeInTheDocument();
    expect(screen.getByText('Alice')).toBeInTheDocument();
  });

  it('renders table result', () => {
    const results: PluginResultPayload[] = [
      {
        type: 'table',
        headers: ['A', 'B'],
        rows: [['1', '2']],
      },
    ];
    render(<PluginResultPanel results={results} />);
    expect(screen.getByText('A')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
  });

  it('renders markdown result', () => {
    const results: PluginResultPayload[] = [{ type: 'markdown', content: '# Title' }];
    render(<PluginResultPanel results={results} />);
    expect(screen.getByText('# Title')).toBeInTheDocument();
  });

  describe('country badge', () => {
    it('shows localized Default badge when tagCode is DEFAULT', () => {
      const results: PluginResultPayload[] = [
        {
          type: 'key_value',
          title: 'Addresses',
          pairs: [{ key: 'Address 1', value: 'Unknown St, Mystery City', tagCode: 'DEFAULT' }],
        },
      ];
      render(<PluginResultPanel results={results} />);
      expect(screen.getByText('Default')).toBeInTheDocument();
    });

    it('shows localized country badge for recognized tagCode', () => {
      const results: PluginResultPayload[] = [
        {
          type: 'key_value',
          title: 'Addresses',
          pairs: [{ key: 'Address 1', value: '...', tagCode: 'CN' }],
        },
      ];
      render(<PluginResultPanel results={results} />);
      expect(screen.getByText('CN')).toBeInTheDocument();
    });

    it('shows default badge when no tag/tagCode is provided', () => {
      const results: PluginResultPayload[] = [
        {
          type: 'key_value',
          title: 'Addresses',
          pairs: [{ key: 'Address 1', value: 'No country info' }],
        },
      ];
      render(<PluginResultPanel results={results} />);
      expect(screen.getByText('Default')).toBeInTheDocument();
    });
  });

  describe('per-pair copy', () => {
    const originalClipboard = navigator.clipboard;
    let writeText: ReturnType<typeof vi.fn>;

    beforeEach(() => {
      writeText = vi.fn().mockResolvedValue(undefined);
      Object.defineProperty(navigator, 'clipboard', {
        value: { writeText },
        configurable: true,
      });
    });

    afterEach(() => {
      Object.defineProperty(navigator, 'clipboard', {
        value: originalClipboard,
        configurable: true,
      });
    });

    it('copies key_value pair entry to clipboard', () => {
      const results: PluginResultPayload[] = [
        {
          type: 'key_value',
          title: 'Summary',
          pairs: [{ key: 'Name', value: 'Alice' }],
        },
      ];
      render(<PluginResultPanel results={results} />);
      const copyBtn = screen.getByRole('button', { name: /copy this entry/i });
      fireEvent.click(copyBtn);
      expect(writeText).toHaveBeenCalledWith('Name: Alice');
    });
  });
});

describe('RF306 watermark typed output commands', () => {
  afterEach(() => {
    for (const toast of useUiStore.getState().toasts) useUiStore.getState().dismissToast(toast.id);
  });

  beforeEach(() => {
    vi.mocked(invoke).mockReset().mockResolvedValue(null);
    vi.mocked(saveWithPause).mockReset().mockResolvedValue('synthetic-destination/saved.png');
    vi.mocked(openWithPause).mockReset().mockResolvedValue('synthetic-batch');
  });

  it('preview and one-file download preserve the authorized output root and exact argument names', async () => {
    const results: PluginResultPayload[] = [
      {
        type: 'watermark_result',
        outputDir: 'synthetic-output',
        items: [
          {
            objectId: 'object',
            attachmentId: 'attachment',
            fileName: 'result.png',
            mimeType: 'image/png',
            outputPath: 'synthetic-output/result.png',
          },
        ],
      },
    ];
    render(<PluginResultPanel results={results} />);
    fireEvent.click(screen.getByTitle('预览'));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('plugin_open_output_file', {
        outputDir: 'synthetic-output',
        path: 'synthetic-output/result.png',
      }),
    );
    fireEvent.click(screen.getByTitle('下载'));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('plugin_copy_output_file', {
        outputDir: 'synthetic-output',
        path: 'synthetic-output/result.png',
        destDir: 'synthetic-destination',
        fileName: 'saved.png',
      }),
    );
    expect(saveWithPause).toHaveBeenCalledWith({ defaultPath: 'result.png' });
  });

  it('batch copy passes the chosen destination and original per-file names', async () => {
    render(
      <PluginResultPanel
        results={[
          {
            type: 'watermark_result',
            outputDir: 'synthetic-output',
            items: [
              {
                objectId: 'object',
                attachmentId: 'attachment',
                fileName: 'original.png',
                mimeType: 'image/png',
                outputPath: 'synthetic-output/original.png',
              },
            ],
          },
        ]}
      />,
    );
    fireEvent.click(screen.getByLabelText('全选'));
    fireEvent.click(screen.getByRole('button', { name: '下载已选项 ({{count}})' }));
    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith('plugin_copy_output_file', {
        outputDir: 'synthetic-output',
        path: 'synthetic-output/original.png',
        destDir: 'synthetic-batch',
        fileName: 'original.png',
      }),
    );
    expect(openWithPause).toHaveBeenCalledWith({ directory: true });
  });
});
