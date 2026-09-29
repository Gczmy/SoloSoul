import { beforeAll, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { IdCard } from 'lucide-react';
import { initReactI18next } from 'react-i18next';
import i18n from '@/lib/i18n';
import enCommon from '@/locales/en-US/common.json';
import enEditor from '@/locales/en-US/editor.json';
import type { ObjectSummary } from '@/stores/objectStore';
import { AndroidObjectRow } from './AndroidObjectRow';

vi.unmock('react-i18next');

const obj: ObjectSummary = {
  id: 'object-a',
  name: 'Passport',
  typeId: 'identity',
  sensitivityLevel: 'internal',
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-02T00:00:00Z',
};

const callbacks = {
  onClick: vi.fn(),
  onHistory: vi.fn(),
  onAttachments: vi.fn(),
  onEdit: vi.fn(),
  onDelete: vi.fn(),
  onSync: vi.fn(),
  onDismissSync: vi.fn(),
};

function renderRow(
  options: { needsSync?: boolean; snapshotCount?: number; attachmentCount?: number } = {},
) {
  vi.clearAllMocks();
  return render(
    <AndroidObjectRow
      obj={obj}
      Icon={IdCard}
      collectionLabel="Identity"
      templateName="Passport template"
      sensitivities={['internal', 'critical']}
      needsSync={options.needsSync ?? false}
      snapshotCount={options.snapshotCount}
      attachmentCount={options.attachmentCount}
      {...callbacks}
    />,
  );
}

beforeAll(async () => {
  await i18n.use(initReactI18next).init({
    lng: 'en-US',
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'editor'],
    resources: { 'en-US': { common: enCommon, editor: enEditor } },
    interpolation: { escapeValue: false },
  });
});

describe('AndroidObjectRow actions', () => {
  it('opens the object from its main row and restores action trigger focus after editing', () => {
    renderRow();
    fireEvent.click(screen.getByRole('button', { name: /Passport.*Identity/ }));
    expect(callbacks.onClick).toHaveBeenCalledOnce();

    const trigger = screen.getByRole('button', { name: 'Actions for Passport' });
    fireEvent.click(trigger);
    const dialog = screen.getByRole('dialog', { name: 'Passport' });
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    fireEvent.click(within(dialog).getByRole('button', { name: 'Edit' }));
    expect(callbacks.onEdit).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(trigger).toHaveFocus();
  });

  it('keeps zero history count visible and routes history, attachments, and delete separately', () => {
    renderRow({ snapshotCount: 0, attachmentCount: 2 });
    const trigger = screen.getByRole('button', { name: 'Actions for Passport' });

    fireEvent.click(trigger);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'History · 0' }),
    );
    expect(callbacks.onHistory).toHaveBeenCalledOnce();
    fireEvent.click(trigger);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Attachments · 2' }),
    );
    expect(callbacks.onAttachments).toHaveBeenCalledOnce();
    fireEvent.click(trigger);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'Delete' }));
    expect(callbacks.onDelete).toHaveBeenCalledOnce();
    expect(callbacks.onEdit).not.toHaveBeenCalled();
  });

  it('offers template update and skip only when synchronization is needed', () => {
    const view = renderRow();
    const trigger = screen.getByRole('button', { name: 'Actions for Passport' });
    fireEvent.click(trigger);
    expect(
      within(screen.getByRole('dialog')).queryByRole('button', { name: 'Skip template update' }),
    ).toBeNull();
    fireEvent.keyDown(document, { key: 'Escape' });
    expect(screen.queryByRole('dialog')).toBeNull();

    view.rerender(
      <AndroidObjectRow
        obj={obj}
        Icon={IdCard}
        collectionLabel="Identity"
        templateName="Passport template"
        sensitivities={['internal', 'critical']}
        needsSync
        {...callbacks}
      />,
    );
    fireEvent.click(trigger);
    const dialog = screen.getByRole('dialog');
    fireEvent.click(
      within(dialog).getByRole('button', { name: /template for this object has been updated/ }),
    );
    expect(callbacks.onSync).toHaveBeenCalledOnce();
    fireEvent.click(trigger);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Skip template update' }),
    );
    expect(callbacks.onDismissSync).toHaveBeenCalledOnce();
  });
});
