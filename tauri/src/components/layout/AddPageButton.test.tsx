import { act, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';
import { DEFAULT_CUSTOM_ICON } from '@/lib/pageIcons';
import { AddPageButton } from './AddPageButton';

const mocks = vi.hoisted(() => ({
  addCustomPage: vi.fn(),
  onCreate: vi.fn(),
  onError: vi.fn(),
}));

vi.mock('@/hooks/useToastError', () => ({
  useToastError: () => ({ onError: mocks.onError }),
}));

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((done, fail) => {
    resolve = done;
    reject = fail;
  });
  return { promise, resolve, reject };
}

const page: CustomPage = {
  id: 'page-a',
  name: 'My page',
  iconId: DEFAULT_CUSTOM_ICON,
  createdAt: '2026-09-29T00:00:00Z',
  sortOrder: 0,
};

beforeEach(() => {
  vi.clearAllMocks();
  useAuthStore.setState({
    currentAccount: { id: 'account-a', name: 'Alice' },
    isAuthenticated: true,
  });
  useSettingsStore.setState((state) => ({
    addCustomPage: mocks.addCustomPage,
    settings: { ...state.settings, customPages: [] },
  }));
});

function openAndFill() {
  render(<AddPageButton onCreate={mocks.onCreate} />);
  fireEvent.click(screen.getByRole('button', { name: 'add_page' }));
  const input = screen.getByRole('textbox', { name: 'add_page_placeholder' });
  fireEvent.change(input, { target: { value: 'My page' } });
  return input;
}

describe('desktop add-page submission', () => {
  it('keeps the draft and popover open while saving and after a failed save', async () => {
    const save = deferred<CustomPage>();
    const retry = deferred<CustomPage>();
    mocks.addCustomPage.mockReturnValueOnce(save.promise).mockReturnValueOnce(retry.promise);
    const input = openAndFill();

    fireEvent.click(screen.getByRole('button', { name: 'common:confirm' }));
    expect(mocks.addCustomPage).toHaveBeenCalledTimes(1);
    expect(input).toHaveValue('My page');
    expect(screen.getByRole('textbox', { name: 'add_page_placeholder' })).toBeVisible();

    await act(async () => save.reject(new Error('save failed')));

    expect(screen.getByRole('textbox', { name: 'add_page_placeholder' })).toHaveValue('My page');
    expect(mocks.onCreate).not.toHaveBeenCalled();
    expect(mocks.onError).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByRole('button', { name: 'common:confirm' }));
    expect(mocks.addCustomPage).toHaveBeenCalledTimes(2);
    await act(async () => retry.resolve(page));
    expect(mocks.onCreate).toHaveBeenCalledExactlyOnceWith(page);
    expect(screen.queryByRole('textbox', { name: 'add_page_placeholder' })).not.toBeInTheDocument();
  });

  it('creates only once while saving and closes after success', async () => {
    const save = deferred<CustomPage>();
    mocks.addCustomPage.mockReturnValue(save.promise);
    openAndFill();

    fireEvent.click(screen.getByRole('button', { name: 'common:confirm' }));
    fireEvent.click(screen.getByRole('button', { name: 'common:confirm' }));
    expect(mocks.addCustomPage).toHaveBeenCalledTimes(1);
    expect(screen.getByRole('textbox', { name: 'add_page_placeholder' })).toBeVisible();

    await act(async () => save.resolve(page));

    await waitFor(() => expect(mocks.onCreate).toHaveBeenCalledExactlyOnceWith(page));
    expect(screen.queryByRole('textbox', { name: 'add_page_placeholder' })).not.toBeInTheDocument();
  });
});
