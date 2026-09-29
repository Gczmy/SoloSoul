import { beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/react';
import { CircleHelp } from 'lucide-react';
import { AndroidObjectDetailFooter } from './AndroidObjectDetailFooter';
import type { GuidePage } from '@/components/guide/PageGuide';

const mocks = vi.hoisted(() => ({ navigate: vi.fn() }));

vi.mock('react-router-dom', () => ({ useNavigate: () => mocks.navigate }));
vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));
vi.mock('@/components/android/AndroidSheet', () => ({
  AndroidSheet: ({
    title,
    onClose,
    children,
  }: {
    title: string;
    onClose: () => void;
    children: React.ReactNode;
  }) => (
    <div role="dialog" aria-label={title}>
      <button type="button" onClick={onClose}>
        Close sheet
      </button>
      {children}
    </div>
  ),
}));

const guidePages: GuidePage[] = [
  {
    icon: CircleHelp,
    title: 'Overview',
    steps: [{ icon: CircleHelp, title: 'Find fields', description: 'Inspect the object' }],
    helpLinks: [{ title: 'Help article', description: 'More detail', href: '/help/objects' }],
  },
  {
    icon: CircleHelp,
    title: 'Privacy',
    steps: [],
    helpLinks: [],
  },
];

function renderFooter(options?: {
  attachmentCount?: number;
  guidePages?: GuidePage[];
  onEdit?: () => void;
}) {
  const onHistory = vi.fn();
  const onAttachments = vi.fn();
  const onDelete = vi.fn();
  render(
    <AndroidObjectDetailFooter
      objectName="Passport"
      attachmentCount={options?.attachmentCount}
      guidePages={options?.guidePages ?? []}
      onHistory={onHistory}
      onAttachments={onAttachments}
      onEdit={options?.onEdit}
      onDelete={onDelete}
    />,
  );
  return { onHistory, onAttachments, onDelete };
}

beforeEach(() => vi.clearAllMocks());

describe('RF-1022 Android object detail footer', () => {
  it('opens attachments and edit directly, and caps the visible attachment count', () => {
    const onEdit = vi.fn();
    const { onAttachments } = renderFooter({ attachmentCount: 123, onEdit });

    fireEvent.click(screen.getByRole('button', { name: 'material.attachments_with_count' }));
    fireEvent.click(screen.getByRole('button', { name: 'edit' }));
    expect(onAttachments).toHaveBeenCalledOnce();
    expect(onEdit).toHaveBeenCalledOnce();
    expect(screen.getByText('99+')).toBeInTheDocument();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('closes the menu before invoking history or delete', () => {
    const { onHistory, onDelete } = renderFooter();
    const actions = screen.getByRole('button', { name: 'material.object_actions' });
    expect(screen.queryByRole('button', { name: 'edit' })).not.toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'attachments' })).toBeInTheDocument();

    fireEvent.click(actions);
    expect(actions).toHaveAttribute('aria-expanded', 'true');
    const menu = screen.getByRole('dialog', { name: 'more_actions' });
    expect(within(menu).queryByRole('button', { name: 'guide' })).not.toBeInTheDocument();
    fireEvent.click(within(menu).getByRole('button', { name: 'history' }));
    expect(onHistory).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(actions).toHaveAttribute('aria-expanded', 'false');

    fireEvent.click(actions);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'delete' }));
    expect(onDelete).toHaveBeenCalledOnce();
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('focuses the guide and closes it before navigating to a help link', () => {
    renderFooter({ attachmentCount: 3, guidePages });
    const actions = screen.getByRole('button', { name: 'material.object_actions' });
    expect(screen.getByText('3')).toBeInTheDocument();
    fireEvent.click(actions);
    fireEvent.click(within(screen.getByRole('dialog')).getByRole('button', { name: 'guide' }));

    const guide = screen.getByRole('dialog', { name: 'Overview' });
    expect(within(guide).getByText('Find fields')).toBeInTheDocument();
    expect(within(guide).getByText('Privacy')).toBeInTheDocument();
    expect(document.activeElement).toHaveClass('android-object-detail-guide');
    fireEvent.click(within(guide).getByRole('button', { name: /Help article/ }));
    expect(mocks.navigate).toHaveBeenCalledWith('/help/objects');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(actions).toHaveAttribute('aria-expanded', 'false');
  });

  it('restores the collapsed state when the sheet closes without an action', () => {
    renderFooter({ guidePages });
    const actions = screen.getByRole('button', { name: 'material.object_actions' });
    fireEvent.click(actions);
    fireEvent.click(
      within(screen.getByRole('dialog')).getByRole('button', { name: 'Close sheet' }),
    );
    expect(actions).toHaveAttribute('aria-expanded', 'false');
    expect(mocks.navigate).not.toHaveBeenCalled();
  });
});
