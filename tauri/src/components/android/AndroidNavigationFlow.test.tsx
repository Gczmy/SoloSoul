import { beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen } from '@testing-library/react';
import { MemoryRouter, useLocation } from 'react-router-dom';
import { initReactI18next } from 'react-i18next';
import i18n from '@/lib/i18n';
import enCommon from '@/locales/en-US/common.json';
import enNavigation from '@/locales/en-US/navigation.json';
import { useAuthStore } from '@/stores/authStore';
import { AndroidNavigation } from './AndroidNavigation';

vi.unmock('react-i18next');

function CurrentLocation() {
  const { pathname, search } = useLocation();
  return <output data-testid="current-location">{pathname + search}</output>;
}

function showNavigation(path: string) {
  return render(
    <MemoryRouter initialEntries={[path]}>
      <AndroidNavigation />
      <CurrentLocation />
    </MemoryRouter>,
  );
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
  useAuthStore.setState({
    currentAccount: { id: 'account-a', name: 'Alice' },
    isAuthenticated: true,
  });
});

describe('RF-926 Android navigation and create sheet', () => {
  it('marks the current destination and hides create on settings', () => {
    showNavigation('/workspace?section=travel');
    expect(screen.getByRole('link', { name: 'Objects' })).toHaveAttribute('aria-current', 'page');
    expect(screen.getByRole('button', { name: 'New' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('link', { name: 'Settings' }));
    expect(screen.getByTestId('current-location')).toHaveTextContent('/settings');
    expect(screen.getByRole('link', { name: 'Settings' })).toHaveAttribute('aria-current', 'page');
    expect(screen.queryByRole('button', { name: 'New' })).toBeNull();
  });

  it('opens the web sheet and carries the selected section into object creation', () => {
    showNavigation('/workspace?section=travel');
    fireEvent.click(screen.getByRole('button', { name: 'New' }));
    expect(screen.getByRole('dialog')).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: /New object/ }));
    expect(screen.getByTestId('current-location')).toHaveTextContent('/editor?section=travel');
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('creates within a custom page and returns from the page form without losing the sheet', () => {
    showNavigation('/workspace/custom/page-a');
    fireEvent.click(screen.getByRole('button', { name: 'New' }));
    fireEvent.click(screen.getByRole('button', { name: /Add Page/ }));
    expect(screen.getByRole('textbox', { name: 'Page name' })).toBeInTheDocument();

    fireEvent.click(screen.getByRole('button', { name: 'Back' }));
    expect(screen.getByRole('button', { name: /New object/ })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: /New object/ }));
    expect(screen.getByTestId('current-location')).toHaveTextContent('/editor?parentId=page-a');
    expect(screen.queryByRole('dialog')).toBeNull();
  });
});
