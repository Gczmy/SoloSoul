import { describe, it, expect, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { isSafeExternalUrl, resolveGuideIdFromHref, GuideRenderer } from './GuideRenderer';

describe('resolveGuideIdFromHref', () => {
  const guides = [
    { id: 'device-sync', files: { zh: 'zh/device_sync.md', en: 'en/device_sync.md' } },
    { id: 'templates', files: { zh: 'zh/templates.md', en: 'en/templates.md' } },
  ];

  it('maps file name to real id when they differ (device_sync.md → device-sync)', () => {
    expect(resolveGuideIdFromHref('device_sync.md', guides)).toBe('device-sync');
  });

  it('maps file name with directory prefix to real id', () => {
    expect(resolveGuideIdFromHref('zh/device_sync.md', guides)).toBe('device-sync');
  });

  it('keeps id when file name matches', () => {
    expect(resolveGuideIdFromHref('templates.md', guides)).toBe('templates');
  });

  it('falls back to file name when no index match', () => {
    expect(resolveGuideIdFromHref('unknown.md', guides)).toBe('unknown');
    expect(resolveGuideIdFromHref('unknown.md', undefined)).toBe('unknown');
  });
});

describe('GuideRenderer H1 divider', () => {
  it('renders the H1 page title with a bottom border divider', () => {
    render(<GuideRenderer content={'# 敏感度与隐私\n\n正文内容'} />);
    const h1 = screen.getByRole('heading', { level: 1 });
    expect(h1).toHaveTextContent('敏感度与隐私');
    // jsdom 不解析 var() 值，直接断言内联样式字符串
    expect(h1.style.borderBottom).toBe('1px solid var(--border-subtle)');
  });
});

describe('GuideRenderer 代码块渲染', () => {
  it('渲染围栏代码块（含 rehype-highlight 链路，不抛错）', () => {
    const md =
      '## 手动同步\n\n```text\nremote 胜出当且仅当：\n  remote.wall_time_ms > local.wall_time_ms\n```\n\n```bash\ncd tauri\nbash scripts/dev-two-instances.sh\n```\n';
    expect(() => render(<GuideRenderer content={md} />)).not.toThrow();
    expect(screen.getAllByRole('button', { name: /复制/ })).toHaveLength(2);
    expect(screen.getAllByText(/remote 胜出当且仅当/).length).toBeGreaterThan(0);
  });
});

describe('isSafeExternalUrl (P229)', () => {
  it('允许 http/https/mailto', () => {
    expect(isSafeExternalUrl('https://example.com')).toBe(true);
    expect(isSafeExternalUrl('http://example.com/path?a=1')).toBe(true);
    expect(isSafeExternalUrl('mailto:user@example.com')).toBe(true);
    expect(isSafeExternalUrl('HTTPS://EXAMPLE.COM')).toBe(true);
  });

  it('允许无协议相对链接，拒绝协议相对链接', () => {
    expect(isSafeExternalUrl('docs/guide.md')).toBe(true);
    expect(isSafeExternalUrl('/absolute/path')).toBe(true);
    expect(isSafeExternalUrl('//evil.example.com')).toBe(false);
  });

  it('拒绝危险协议', () => {
    expect(isSafeExternalUrl('javascript:alert(1)')).toBe(false);
    expect(isSafeExternalUrl('data:text/html,<script>alert(1)</script>')).toBe(false);
    expect(isSafeExternalUrl('file:///etc/passwd')).toBe(false);
    expect(isSafeExternalUrl('vbscript:msgbox(1)')).toBe(false);
    expect(isSafeExternalUrl('ftp://example.com')).toBe(false);
  });

  it('拒绝空串与空白串', () => {
    expect(isSafeExternalUrl('')).toBe(false);
    expect(isSafeExternalUrl('   ')).toBe(false);
  });
});

describe('RF-1025 guide document interactions', () => {
  it('renders custom guide sections and ignores malformed card rows', () => {
    const onLinkClick = vi.fn();
    render(
      <GuideRenderer
        onLinkClick={onLinkClick}
        content={[
          'Before the steps.',
          '<!--stepper Setup-->**First step**<!--/stepper-->',
          '<!--tip-->Tip content<!--/tip-->',
          '<!--info-->Info content<!--/info-->',
          '<!--warning-->Warning content<!--/warning-->',
          '<!--cards-->',
          'This row is not a card',
          '- [Backup](backup.md) — Save a copy',
          '- [Broken](broken.md)',
          '- [Restore](restore.md): Recover a copy',
          '<!--/cards-->',
          'After the cards.',
        ].join('\n\n')}
      />,
    );

    expect(screen.getByText('Before the steps.')).toBeInTheDocument();
    expect(screen.getByText('Setup')).toBeInTheDocument();
    expect(screen.getByText('First step')).toBeInTheDocument();
    expect(screen.getByText('Tip content')).toBeInTheDocument();
    expect(screen.getByText('Info content')).toBeInTheDocument();
    expect(screen.getByText('Warning content')).toBeInTheDocument();
    expect(screen.getByText('After the cards.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: /Broken/ })).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /Backup/ }));
    fireEvent.click(screen.getByRole('button', { name: /Restore/ }));
    expect(onLinkClick.mock.calls).toEqual([['backup.md'], ['restore.md']]);
  });

  it('routes Markdown guide links inside the app and keeps unsafe links non-clickable', () => {
    const onLinkClick = vi.fn();
    render(
      <GuideRenderer
        onLinkClick={onLinkClick}
        content={[
          '[Local guide](guides/privacy.md)',
          '[Website](https://example.com/help)',
          '[Unsafe](javascript:alert(1))',
          '| Item | Value |\n| --- | --- |\n| Account | Local |',
        ].join('\n\n')}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Local guide' }));
    expect(onLinkClick).toHaveBeenCalledWith('guides/privacy.md');
    expect(screen.getByRole('link', { name: 'Website' })).toHaveAttribute(
      'href',
      'https://example.com/help',
    );
    expect(screen.queryByRole('link', { name: 'Unsafe' })).not.toBeInTheDocument();
    expect(screen.getByRole('table')).toHaveTextContent('Account');
  });
});
