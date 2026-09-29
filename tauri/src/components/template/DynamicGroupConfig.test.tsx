import { useState } from 'react';
import { describe, expect, it } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import type { PropertyType } from '@/types/template';
import { DynamicGroupConfig } from './DynamicGroupConfig';

function ControlledConfig({ initialAllowedTypes }: { initialAllowedTypes: PropertyType[] }) {
  const [allowedTypes, setAllowedTypes] = useState(initialAllowedTypes);

  return (
    <DynamicGroupConfig
      allowedTypes={allowedTypes}
      onAllowedTypesChange={setAllowedTypes}
      onMaxItemsChange={() => {}}
      onSensitivityChange={() => {}}
    />
  );
}

describe('DynamicGroupConfig', () => {
  it('keeps the last allowed type instead of silently removing the restriction', () => {
    render(<ControlledConfig initialAllowedTypes={['text']} />);

    fireEvent.click(screen.getByRole('button', { name: /editor:dynamic_group_allowed_types/ }));
    const textType = screen.getByRole('checkbox', { name: 'editor:field_types.text' });

    expect(textType).toBeChecked();
    fireEvent.click(textType);

    expect(textType).toBeChecked();
    expect(
      screen.getByRole('button', { name: /editor:dynamic_group_allowed_types: 1\/12/ }),
    ).toBeInTheDocument();
    expect(textType).toBeDisabled();
  });

  it('still lets users change a multi-type restriction and explicitly select all', () => {
    render(<ControlledConfig initialAllowedTypes={['text', 'email']} />);

    fireEvent.click(screen.getByRole('button', { name: /editor:dynamic_group_allowed_types/ }));
    const textType = screen.getByRole('checkbox', { name: 'editor:field_types.text' });
    const emailType = screen.getByRole('checkbox', { name: 'editor:field_types.email' });

    fireEvent.click(emailType);
    expect(emailType).not.toBeChecked();
    expect(textType).toBeChecked();
    expect(textType).toBeDisabled();

    fireEvent.click(screen.getByRole('button', { name: 'editor:dynamic_group_select_all' }));
    expect(textType).toBeChecked();
    expect(emailType).toBeChecked();
    expect(textType).not.toBeDisabled();
  });
});
