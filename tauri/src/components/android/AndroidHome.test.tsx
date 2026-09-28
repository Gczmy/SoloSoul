import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { initReactI18next } from 'react-i18next';
import { invoke } from '@tauri-apps/api/core';
import i18n from '@/lib/i18n';
import enCommon from '@/locales/en-US/common.json';
import enNavigation from '@/locales/en-US/navigation.json';
import { useAuthStore } from '@/stores/authStore';
import { useSettingsStore, type CustomPage } from '@/stores/settingsStore';
import { prefetchRegistry } from '@/lib/prefetch/registry';
import { AndroidHome } from './AndroidHome';

vi.unmock('react-i18next');

const activePage: CustomPage = {
  id: 'page-a',
  name: 'Private Page',
  iconId: 'star',
  createdAt: '2026-01-01',
  sortOrder: 0,
};

function CurrentLocation() {
  const { pathname, search } = useLocation();
  return <output data-testid="current-location">{pathname + search}</output>;
}

function showHome(onEditPage = vi.fn(), onPhotos = vi.fn()) {
  render(
    <MemoryRouter>
      <AndroidHome onEditPage={onEditPage} onPhotos={onPhotos} />
      <CurrentLocation />
    </MemoryRouter>,
  );
  return { onEditPage, onPhotos };
}

beforeAll(async () => {
  await i18n.use(initReactI18next).init({
    lng: 'en-US',
    fallbackLng: 'en-US',
    defaultNS: 'common',
    ns: ['common', 'navigation'],
    resources: { 'en-US': { common: enCommon, navigation: enNavigation } },
    interpolation: { escapeValue: false },
  });
});

beforeEach(() => {
  vi.mocked(invoke).mockReset();
  useAuthStore.setState({
    currentAccount: { id: 'account-a', name: 'Alice' },
    isAuthenticated: true,
  });
  useSettingsStore.setState((state) => ({
    settings: { ...state.settings, customPages: [] },
  }));
  prefetchRegistry.androidOverview.reset();
});

describe('RF-925 Android home overview', () => {
  it('shows only object summaries and opens the selected recent object', async () => {
    vi.mocked(invoke).mockResolvedValue([
      {
        id: 'passport/1',
        name: 'Passport',
        typeId: 'identity',
        updatedAt: '2026-09-01',
        properties: { password: 'PRIVATE-FIELD-VALUE' },
      },
    ]);
    showHome();

    const recent = await screen.findByRole('button', { name: /Passport/ });
    expect(screen.getByText('Welcome back, Alice')).toBeInTheDocument();
    expect(screen.getByText('1')).toBeInTheDocument();
    expect(screen.queryByText('PRIVATE-FIELD-VALUE')).not.toBeInTheDocument();
    expect(invoke).toHaveBeenCalledWith('object_list', { accountId: 'account-a' });

    fireEvent.click(recent);
    expect(screen.getByTestId('current-location')).toHaveTextContent(
      '/workspace?objectId=passport%2F1',
    );
  });

  it('hides deleted custom pages while keeping edit, navigation, and photo actions', async () => {
    vi.mocked(invoke).mockResolvedValue([]);
    useSettingsStore.setState((state) => ({
      settings: {
        ...state.settings,
        customPages: [
          activePage,
          { ...activePage, id: 'page-deleted', name: 'Deleted Page', deletedAt: '2026-09-01' },
        ],
      },
    }));
    const { onEditPage, onPhotos } = showHome();
    await screen.findByText('No objects yet');

    expect(screen.queryByText('Deleted Page')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit page: Private Page' }));
    expect(onEditPage).toHaveBeenCalledWith(activePage, expect.anything());
    fireEvent.click(screen.getByRole('button', { name: 'Photo Album' }));
    expect(onPhotos).toHaveBeenCalledOnce();

    fireEvent.click(screen.getByRole('button', { name: /^Private Page/ }));
    expect(screen.getByTestId('current-location')).toHaveTextContent('/workspace/custom/page-a');
  });

  it('offers a retry after overview loading fails', async () => {
    vi.mocked(invoke)
      .mockRejectedValueOnce(new Error('offline'))
      .mockResolvedValueOnce([
        { id: 'a', name: 'Recovered object', typeId: 'identity', updatedAt: '2026-09-01' },
      ]);
    showHome();

    fireEvent.click(await screen.findByRole('button', { name: 'Could not load. Tap to retry' }));
    await screen.findByText('Recovered object');
    await waitFor(() => expect(invoke).toHaveBeenCalledTimes(2));
    expect(screen.queryByRole('button', { name: 'Could not load. Tap to retry' })).toBeNull();
  });
});
