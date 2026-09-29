import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { isAndroidSync } from '@/lib/platform';
import { AttachmentActions } from './AttachmentActions';

vi.mock('@/lib/platform', () => ({ isAndroidSync: vi.fn(() => false) }));

function callbacks() {
  return {
    onPreview: vi.fn(),
    onDownload: vi.fn(),
    onShare: vi.fn(),
    onEditMeta: vi.fn(),
    onSoftDelete: vi.fn(),
    onRestore: vi.fn(),
    onPermanentDelete: vi.fn(),
  };
}

describe('AttachmentActions', () => {
  beforeEach(() => {
    vi.mocked(isAndroidSync).mockReturnValue(false);
  });

  it('routes desktop active-file actions without exposing trash-only actions', () => {
    const actions = callbacks();
    render(<AttachmentActions fileName="report.pdf" showTrash={false} {...actions} />);

    fireEvent.click(screen.getByRole('button', { name: 'common:preview' }));
    fireEvent.click(screen.getByRole('button', { name: 'common:download' }));
    fireEvent.click(screen.getByRole('button', { name: 'common:forward' }));
    fireEvent.click(screen.getByRole('button', { name: 'Edit Attachment Attributes' }));
    fireEvent.click(screen.getByRole('button', { name: 'common:delete' }));

    expect(actions.onPreview).toHaveBeenCalledOnce();
    expect(actions.onDownload).toHaveBeenCalledOnce();
    expect(actions.onShare).toHaveBeenCalledOnce();
    expect(actions.onEditMeta).toHaveBeenCalledOnce();
    expect(actions.onSoftDelete).toHaveBeenCalledOnce();
    expect(actions.onRestore).not.toHaveBeenCalled();
    expect(actions.onPermanentDelete).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'common:delete_permanently' })).toBeNull();
  });

  it('routes desktop trash actions only to restore and permanent delete', () => {
    const actions = callbacks();
    render(<AttachmentActions fileName="report.pdf" showTrash {...actions} />);

    fireEvent.click(screen.getByRole('button', { name: 'common:restore' }));
    fireEvent.click(screen.getByRole('button', { name: 'common:delete_permanently' }));

    expect(actions.onRestore).toHaveBeenCalledOnce();
    expect(actions.onPermanentDelete).toHaveBeenCalledOnce();
    expect(actions.onSoftDelete).not.toHaveBeenCalled();
    expect(screen.queryByRole('button', { name: 'common:delete' })).toBeNull();
  });

  it('opens the Android active-file sheet without triggering the row and closes after an action', () => {
    vi.mocked(isAndroidSync).mockReturnValue(true);
    const actions = callbacks();
    const onRowClick = vi.fn();
    render(
      <div onClick={onRowClick}>
        <AttachmentActions fileName="report.pdf" showTrash={false} {...actions} />
      </div>,
    );

    const trigger = screen.getByRole('button', { name: 'attachment_actions_for' });
    fireEvent.click(trigger);
    expect(onRowClick).not.toHaveBeenCalled();
    expect(trigger).toHaveAttribute('aria-expanded', 'true');
    const dialog = screen.getByRole('dialog', { name: 'more_actions' });
    expect(within(dialog).getByText('report.pdf')).toBeInTheDocument();
    fireEvent.click(within(dialog).getByRole('button', { name: 'edit_meta' }));

    expect(actions.onEditMeta).toHaveBeenCalledOnce();
    expect(actions.onSoftDelete).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(trigger).toHaveFocus();

    fireEvent.click(trigger);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
    expect(actions.onSoftDelete).toHaveBeenCalledOnce();
    expect(actions.onPermanentDelete).not.toHaveBeenCalled();
  });

  it('does not offer Android metadata editing when the action is unavailable', () => {
    vi.mocked(isAndroidSync).mockReturnValue(true);
    const actions = callbacks();
    render(
      <AttachmentActions
        fileName="report.pdf"
        showTrash={false}
        {...actions}
        onEditMeta={undefined}
      />,
    );

    fireEvent.click(screen.getByRole('button', { name: 'attachment_actions_for' }));
    const dialog = screen.getByRole('dialog', { name: 'more_actions' });
    expect(within(dialog).queryByRole('button', { name: 'edit_meta' })).toBeNull();
    expect(within(dialog).getByRole('button', { name: 'preview' })).toBeInTheDocument();
  });

  it('keeps Android trash actions distinct from soft delete', () => {
    vi.mocked(isAndroidSync).mockReturnValue(true);
    const actions = callbacks();
    render(<AttachmentActions fileName="report.pdf" showTrash {...actions} />);

    fireEvent.click(screen.getByRole('button', { name: 'attachment_actions_for' }));
    const dialog = screen.getByRole('dialog', { name: 'more_actions' });
    expect(within(dialog).queryByRole('button', { name: 'preview' })).toBeNull();
    expect(within(dialog).queryByRole('button', { name: 'delete' })).toBeNull();
    fireEvent.click(within(dialog).getByRole('button', { name: 'delete_permanently' }));

    expect(actions.onPermanentDelete).toHaveBeenCalledOnce();
    expect(actions.onSoftDelete).not.toHaveBeenCalled();
    expect(screen.queryByRole('dialog')).toBeNull();

    fireEvent.click(screen.getByRole('button', { name: 'attachment_actions_for' }));
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'restore' }));
    expect(actions.onRestore).toHaveBeenCalledOnce();
  });
});
