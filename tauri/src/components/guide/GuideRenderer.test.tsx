import { describe, it, expect } from 'vitest';
import { isSafeExternalUrl } from './GuideRenderer';

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
