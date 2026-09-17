import { fireEvent, render, screen, within } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import { MandatoryUpdateOverlay } from '@/pages/system/MandatoryUpdateOverlay';
import { UpdateInfoCard } from '@/pages/system/UpdateInfoCard';
import { ReleaseNotesMarkdown } from './ReleaseNotesMarkdown';
import { UpdateBanner } from './UpdateBanner';

// 使用正式发布说明的表格格式，验证真实解析器，而不是 mock Markdown 后匹配原文。
const releaseNotes = [
  '**下载**',
  '',
  '| 平台 | 文件 |',
  '|------|------|',
  '| macOS (Apple Silicon) | `SoloSoul_2.13.0_arm64.dmg` |',
  '| Windows x64 | `SoloSoul_2.13.0_x64-setup.exe` |',
  '| Android (通用) | `SoloSoul_2.13.0_universal-release.apk` |',
].join('\n');

const updateProps = {
  info: { appName: 'SoloSoul', version: '2.12.3', os: 'macos', arch: 'aarch64' },
  versionInfo: {
    currentVersion: '2.12.3',
    latestVersion: '2.13.0',
    state: 'available' as const,
    body: releaseNotes,
  },
  downloading: false,
  downloadedBytes: 0,
  totalBytes: 0,
  downloadError: null,
  progressPercent: 0,
  handleUpdate: vi.fn(),
};

function expectDownloadTable(table: HTMLElement) {
  expect(within(table).getByRole('columnheader', { name: '平台' })).toBeInTheDocument();
  expect(within(table).getByRole('columnheader', { name: '文件' })).toBeInTheDocument();
  expect(within(table).getAllByRole('row')).toHaveLength(4);
  expect(
    within(table).getByRole('cell', { name: 'SoloSoul_2.13.0_universal-release.apk' }),
  ).toBeInTheDocument();
  expect(within(table).getByText('SoloSoul_2.13.0_arm64.dmg').tagName).toBe('CODE');
  expect(table.textContent).not.toContain('|------|');
}

describe('ReleaseNotesMarkdown', () => {
  it('renders a GFM download table with headers, rows and inline code', () => {
    render(<ReleaseNotesMarkdown>{releaseNotes}</ReleaseNotesMarkdown>);
    expectDownloadTable(screen.getByRole('table'));
  });

  it('keeps raw HTML disabled and strips unsafe link protocols', () => {
    const { container } = render(
      <ReleaseNotesMarkdown>
        {[
          '<script>alert("unsafe")</script>',
          '',
          '<iframe src="https://example.com"></iframe>',
          '',
          '[Unsafe](javascript:alert%281%29)',
          '',
          '[Release](https://github.com/Gczmy/SoloSoul/releases/tag/v2.13.0)',
        ].join('\n')}
      </ReleaseNotesMarkdown>,
    );

    expect(container.querySelector('script, iframe')).toBeNull();
    expect(screen.getByText('Unsafe')).not.toHaveAttribute(
      'href',
      expect.stringContaining('javascript:'),
    );
    expect(screen.getByRole('link', { name: 'Release' })).toHaveAttribute(
      'href',
      'https://github.com/Gczmy/SoloSoul/releases/tag/v2.13.0',
    );
  });

  it('renders the real GFM table after opening the banner release notes', async () => {
    render(
      <UpdateBanner
        version="2.13.0"
        state="available"
        downloadedBytes={0}
        totalBytes={0}
        releaseNotes={releaseNotes}
        onUpdate={vi.fn()}
        onCancel={vi.fn()}
        onInstall={vi.fn()}
        onSkip={vi.fn()}
        onClose={vi.fn()}
      />,
    );
    expect(screen.queryByRole('table')).not.toBeInTheDocument();
    fireEvent.click(screen.getByLabelText('view_release_notes'));
    expectDownloadTable(await screen.findByRole('table'));
  });

  it('renders the same GFM table in the About page update card', () => {
    render(
      <UpdateInfoCard
        {...updateProps}
        loading={false}
        checking={false}
        downloadProgress={null}
        runCheck={vi.fn()}
      />,
    );
    expectDownloadTable(screen.getByRole('table'));
  });

  it('renders the same GFM table in mandatory update release notes', () => {
    render(<MandatoryUpdateOverlay {...updateProps} isMandatory />);
    fireEvent.click(screen.getByLabelText(/view_release_notes/i));
    expectDownloadTable(screen.getByRole('table'));
  });
});
