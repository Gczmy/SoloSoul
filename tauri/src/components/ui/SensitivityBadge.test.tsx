import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import { SensitivityBadge, getSensitivityStyle, type SensitivityLevel } from './SensitivityBadge';

describe('SensitivityBadge', () => {
  const levels: SensitivityLevel[] = ['public', 'internal', 'sensitive', 'critical'];

  it('renders for all sensitivity levels', () => {
    levels.forEach((level) => {
      const { unmount } = render(<SensitivityBadge level={level} />);
      expect(screen.getByText(level)).toBeInTheDocument();
      unmount();
    });
  });

  it('renders with correct title attribute', () => {
    render(<SensitivityBadge level="public" />);
    const badge = screen.getByText('public');
    expect(badge.closest('span')).toHaveAttribute('title', 'sensitivity_label: public');
  });

  it('applies inline styles for public level', () => {
    render(<SensitivityBadge level="public" />);
    const badge = screen.getByText('public').closest('span');
    expect(badge).toHaveStyle({
      display: 'inline-flex',
      fontSize: 'var(--text-badge)',
      fontWeight: '600',
    });
  });
});

describe('getSensitivityStyle', () => {
  it('returns style for known levels', () => {
    expect(getSensitivityStyle('public').fg).toBe('#3D8B5E');
    expect(getSensitivityStyle('internal').fg).toBe('#4A90D9');
    expect(getSensitivityStyle('sensitive').fg).toBe('#D4850A');
    expect(getSensitivityStyle('critical').fg).toBe('#C0392B');
  });

  it('falls back to internal for unknown levels', () => {
    const style = getSensitivityStyle('unknown' as SensitivityLevel);
    expect(style).toEqual(getSensitivityStyle('internal'));
  });
});
