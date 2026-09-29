import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';
import type { TemplateSyncResult } from '@/lib/templateSync';
import { TemplateSyncConfirmDialog } from './TemplateSyncConfirmDialog';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

const changedResult: TemplateSyncResult = {
  hasChanges: true,
  templateHash: 'hash-new',
  fieldsAdded: [{ id: 'new-field', name: 'New field', fieldType: 'text' }],
  fieldsDeprecated: [],
  fieldsUpdated: [],
  fieldsIncompatible: [],
};

describe('RF-1016 template sync confirmation lifecycle', () => {
  it.each([null, changedResult])(
    'keeps the dialog open during a pending preview or apply, then allows closing',
    (result) => {
      const onCancel = vi.fn();
      const onConfirm = vi.fn();
      const { rerender } = render(
        <TemplateSyncConfirmDialog
          isOpen
          result={result}
          loading
          onCancel={onCancel}
          onConfirm={onConfirm}
        />,
      );
      expect(screen.getByRole('dialog')).toBeInTheDocument();
      fireEvent.keyDown(document, { key: 'Escape' });
      fireEvent.click(document.querySelector('[data-macos-glass-backdrop]')!);
      expect(onCancel).not.toHaveBeenCalled();
      expect(screen.getByRole('dialog')).toBeInTheDocument();

      rerender(
        <TemplateSyncConfirmDialog
          isOpen
          result={result}
          loading={false}
          onCancel={onCancel}
          onConfirm={onConfirm}
        />,
      );
      fireEvent.keyDown(document, { key: 'Escape' });
      expect(onCancel).toHaveBeenCalledOnce();
    },
  );
});
