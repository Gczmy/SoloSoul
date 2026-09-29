import { render, screen } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import type { TemplateProperty } from '@/types/template';
import { DeprecatedFieldsSection } from './DeprecatedFieldsSection';

it('已归档旧字段的非法敏感度显示为 internal', () => {
  const legacyFields = [
    {
      id: 'old',
      name: '旧字段',
      type: 'text',
      sensitivityLevel: 'unknown',
      deprecatedAt: '2026-09-01T00:00:00Z',
    },
  ] as unknown as TemplateProperty[];
  render(
    <DeprecatedFieldsSection
      editProperties={legacyFields}
      showDeprecated
      fieldUsageMap={{}}
      onToggleShowDeprecated={vi.fn()}
      onRestoreProperty={vi.fn()}
      onPermanentlyRemoveProperty={vi.fn()}
    />,
  );

  expect(screen.getByTitle('sensitivity_label: internal')).toBeInTheDocument();
  expect(screen.queryByTitle('sensitivity_label: unknown')).not.toBeInTheDocument();
});
