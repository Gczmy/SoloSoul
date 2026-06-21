import { render, screen, fireEvent } from '@testing-library/react';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { PluginResultPanel } from './PluginResultPanel';
import type { PluginResultPayload } from '@/lib/plugin';

describe('PluginResultPanel', () => {
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
    expect(screen.getByText('Summary')).toBeInTheDocument();
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
